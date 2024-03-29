mod cache;
mod error;
mod extractor;
mod filter;
mod media;
mod proxy;
mod range;

use crate::{cache::Cache, error::Error};
use axum::{
    body::Body,
    extract::{Path, Query},
    response::Response,
    routing::get,
    Extension, Router,
};
use clap::Parser;
use filter::Filter;
use proxy::FeedInfo;
use reqwest::{header::HeaderMap, Client};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Config {
    /// Socket address
    #[arg(short = 's', long = "socket", default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000))]
    socket_addres: SocketAddr,
    /// How long to keep feeds cached (in seconds)
    #[arg(short = 'c', long = "cache", default_value_t = 300)]
    cache_timeout: u64,
}

#[tokio::main]
async fn main() {
    let config = Config::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("ytfeed=DEBUG".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    let socket_address = config.socket_addres;

    let mut headers = HeaderMap::new();
    headers.insert("accept-language", "en".parse().unwrap());
    let client = Client::builder()
        .brotli(true)
        .timeout(Duration::new(10, 0))
        .default_headers(headers)
        .build()
        .unwrap();

    let app = Router::new()
        .route("/:channel_id", get(get_feed))
        .layer(Extension(client))
        .layer(Extension(HashMap::<String, String>::new()))
        .layer(Extension(Cache::<String, Option<FeedInfo>>::new(Some(
            Duration::from_secs(config.cache_timeout),
        ))));

    info!("Starting server at http://{}", socket_address);

    let listener = TcpListener::bind(socket_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_feed(
    Path(channel): Path<String>,
    Query(filter): Query<Filter>,
    Extension(http_client): Extension<Client>,
    // Cache for channel handles
    Extension(mut handle_cache): Extension<HashMap<String, String>>,
    // Cache for channel feeds
    Extension(feed_cache): Extension<Cache<String, Option<FeedInfo>>>,
) -> Result<Response, Error> {
    info!("Get feed '{}'", channel);

    let channel_name = if channel.starts_with('@') {
        handle_cache.get(&channel).unwrap_or(&channel)
    } else {
        &channel
    };

    let feed_info = {
        let channel_name = channel_name.clone();
        feed_cache
            // TODO: Currently, the cache isn't shared between channel handles and channel ids
            // Make it possible to update the cache with the returned key value
            // (Note: this is an exceptional situation due to way the proxy works)
            .get_cached(channel_name.clone(), || {
                Box::pin(async move {
                    match proxy::proxy_feed(&channel_name, &http_client).await {
                        Ok(feed_info) => {
                            if channel_name.starts_with('@') {
                                // Cache the channel id associated with the handle
                                info!(
                                    "Cached channel id '{}' for handle '{}'",
                                    feed_info.extraction.channel.id, channel_name
                                );
                                handle_cache
                                    .insert(channel_name, feed_info.extraction.channel.id.clone());
                            }
                            Ok::<_, Error>(Some(feed_info))
                        }
                        Err(err) => {
                            error!("Failed to extract data from channel '{channel_name}': {err}");
                            Ok::<_, Error>(None)
                        }
                    }
                })
            })
            .await?
    }
    .ok_or(Error::Proxy(channel))?;

    let feed = filter::filter_feed(feed_info, filter)?;

    Ok(Response::builder()
        .header("Content-Type", "application/atom+xml")
        .body(Body::from(feed))
        .unwrap()
        .into())
}
