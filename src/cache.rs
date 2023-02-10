/*
 * Code based on: https://fasterthanli.me/articles/request-coalescing-in-async-rust#making-it-generic
 */
use futures::Future;
use parking_lot::Mutex;
use std::{
    pin::Pin,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use tracing::info;

pub type BoxFut<'a, O> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

#[derive(Clone)]
pub struct Cache<T>
where
    T: Clone + Send + Sync + 'static,
{
    inner: Arc<Mutex<CacheInner<T>>>,
    refresh_interval: Option<Duration>,
}

struct CacheInner<T>
where
    T: Clone + Send + Sync + 'static,
{
    cached: Option<(Instant, T)>,
    task: Option<Weak<broadcast::Sender<Result<T, CacheError>>>>,
}

impl<T> Default for CacheInner<T>
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

impl<T> Cache<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(refresh_interval: Option<Duration>) -> Self {
        Self {
            inner: Default::default(),
            refresh_interval,
        }
    }

    pub async fn get_cached<F, E>(&self, f: F) -> Result<T, CacheError>
    where
        F: FnOnce() -> BoxFut<'static, Result<T, E>>,
        E: std::fmt::Display + 'static,
    {
        let mut rx = {
            let mut inner = self.inner.lock();

            // Check if item is in the cache
            if let Some((fetched_at, value)) = inner.cached.as_ref() {
                if self.refresh_interval.is_none()
                    || Some(fetched_at.elapsed()) < self.refresh_interval
                {
                    return Ok(value.clone());
                } else {
                    info!("stale, refresh item");
                }
            }

            if let Some(tasks) = inner.task.as_ref().and_then(Weak::upgrade) {
                // Subscribe to the task's channel if already being fetched
                tasks.subscribe()
            } else {
                // Create a new channel to fetch the value
                let (tx, rx) = broadcast::channel::<Result<T, CacheError>>(1);
                let tx = Arc::new(tx);
                inner.task = Some(Arc::downgrade(&tx));
                let inner = self.inner.clone();

                // Execute the closure first to avoid sending it across threads
                let fut = f();
                tokio::spawn(async move {
                    let res = fut.await;
                    {
                        let mut inner = inner.lock();
                        inner.task = None;

                        match res {
                            Ok(value) => {
                                inner.cached.replace((Instant::now(), value.clone()));
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
