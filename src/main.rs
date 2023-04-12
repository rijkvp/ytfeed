mod cache;
mod error;
mod extractor;
mod filter;
mod proxy;

use crate::{cache::Cache, error::Error};
use axum::{
    body::boxed,
    extract::{Path, Query},
    http::HeaderMap,
    response::Response,
    routing::get,
    Extension, Router,
};
use clap::Parser;
use filter::Filter;
use reqwest::Client;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Config {
    /// IP bind address
    #[arg(short = 'b', long = "bind", default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000))]
    bind_address: SocketAddr,

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

    let bind_address = config.bind_address;

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
        .layer(Extension(Cache::<String, Option<String>>::new(Some(
            Duration::from_secs(config.cache_timeout),
        ))));

    info!("Starting server at http://{}", bind_address);

    axum::Server::bind(&bind_address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_feed(
    Path(channel_id): Path<String>,
    Query(filter): Query<Filter>,
    Extension(http_client): Extension<Client>,
    Extension(feed_cache): Extension<Cache<String, Option<String>>>,
) -> Result<Response, Error> {
    info!("GET feed for '{}'", channel_id);

    // TODO: Add channel tag stuff back
    let feed = {
        let channel_id = channel_id.clone();
        feed_cache
            .get_cached(channel_id.clone(), || {
                Box::pin(async move {
                    match proxy::proxy_feed(&channel_id, filter, &http_client).await {
                        Ok(c) => Ok::<_, Error>(Some(c)),
                        Err(err) => {
                            error!("Failed to extract data from channel '{channel_id}': {err}");
                            Ok::<_, Error>(None)
                        }
                    }
                })
            })
            .await?
    }
    .ok_or(Error::Proxy(channel_id))?;

    Ok(Response::builder()
        .header("Content-Type", "text/xml")
        .body(boxed(feed))
        .unwrap())
}
