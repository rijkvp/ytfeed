/*
 * Code based on: https://fasterthanli.me/articles/request-coalescing-in-async-rust#making-it-generic
 */
use futures::Future;
use parking_lot::Mutex;
use std::hash::Hash;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tracing::debug;

#[derive(Clone)]
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone + Send + Sync + 'static,
{
    timeout: Option<Duration>,
    items: Arc<Mutex<HashMap<K, CacheItem<V>>>>,
}

struct CacheItem<T>
where
    T: Clone + Send + Sync + 'static,
{
    cached: Option<(Instant, T)>,
    task: Option<Weak<broadcast::Sender<Result<T, CacheError>>>>,
}

impl<T> Default for CacheItem<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            cached: None,
            task: None,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct CacheError(String);

pub type BoxFut<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + std::fmt::Debug + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(timeout: Option<Duration>) -> Self {
        Self {
            items: Default::default(),
            timeout,
        }
    }

    pub async fn get_cached<F, E>(&self, key: K, f: F) -> Result<V, CacheError>
    where
        F: FnOnce() -> BoxFut<'static, Result<V, E>>,
        E: std::fmt::Display + 'static,
    {
        let mut rx = {
            let mut items = self.items.lock();

            // Get exsisting or create new item
            let item = items.entry(key.clone()).or_default();

            // Check if item is in the cache
            if let Some((fetched_at, value)) = item.cached.as_ref() {
                if self.timeout.is_none() || Some(fetched_at.elapsed()) < self.timeout {
                    return Ok(value.clone());
                } else {
                    debug!("{key:?} has timed-out, fetching new value");
                }
            }

            if let Some(tasks) = item.task.as_ref().and_then(Weak::upgrade) {
                // Subscribe to the task's channel if already being fetched
                tasks.subscribe()
            } else {
                // Create a new channel to fetch the value
                let (tx, rx) = broadcast::channel::<Result<V, CacheError>>(1);
                let tx = Arc::new(tx);
                item.task = Some(Arc::downgrade(&tx));

                let items = self.items.clone();
                let key = key.clone();
                // Execute the closure first to avoid sending it across threads
                let fut = f();
                tokio::spawn(async move {
                    let res = fut.await;
                    {
                        let mut items = items.lock();
                        let item = items.entry(key).or_default();
                        item.task = None;

                        match res {
                            Ok(value) => {
                                item.cached.replace((Instant::now(), value.clone()));
                                let _ = tx.send(Ok(value));
                            }
                            Err(e) => {
                                let _ = tx.send(Err(CacheError(e.to_string())));
                            }
                        };
                    }
                });
                rx
            }
        };
        rx.recv().await.map_err(|e| CacheError(e.to_string()))?
    }
}
