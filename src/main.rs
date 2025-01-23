mod cache;
mod dearrow;
mod error;
mod extractor;
mod feed;
mod filter;
mod proxy;
mod range;

use crate::{cache::Cache, error::Error};
use axum::{
    body::Body,
    extract::{Path, Query},
    http::Request,
    response::Response,
    routing::get,
    Extension, Router,
};
use clap::Parser;
use feed::Feed;
use filter::Filter;
use reqwest::Client;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
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
    /// Base URL used in the feed
    #[arg(
        short = 'b',
        long = "base_url",
        default_value = "http://localhost:8000/"
    )]
    base_url: String,
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

    let client = Client::builder()
        .brotli(true) // reduce bandwidth, youtube.com supports it
        .gzip(true)
        .timeout(Duration::new(10, 0))
        .build()
        .unwrap();

    let trace_layer = TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
        let uri = request.uri().to_string();
        tracing::info_span!("http_request", method = ?request.method(), uri)
    });

    let router = Router::new()
        .route("/@{handle}", get(get_feed))
        .layer(Extension(client))
        .layer(Extension(HashMap::<String, String>::new()))
        .layer(Extension(Cache::<String, Option<Feed>>::new(Some(
            Duration::from_secs(config.cache_timeout),
        ))))
        .layer(Extension(config.base_url))
        .layer(trace_layer);

    tracing::info!("starting server at http://{}", socket_address);

    let listener = TcpListener::bind(socket_address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn get_feed(
    Path(handle): Path<String>,
    Query(filter): Query<Filter>,
    Extension(http_client): Extension<Client>,
    Extension(feed_cache): Extension<Cache<String, Option<Feed>>>,
    Extension(base_url): Extension<String>,
) -> Result<Response, Error> {
    tracing::info!("get feed '{}'", handle);

    let feed = {
        let handle = handle.clone();
        let http_client = http_client.clone();
        feed_cache
            .get_cached(handle.clone(), || {
                Box::pin(async move {
                    match proxy::proxy_feed(&handle, &http_client).await {
                        Ok(feed) => Ok::<_, Error>(Some(feed)),
                        Err(err) => {
                            tracing::error!("failed to get data from channel '{handle}': {err}");
                            Ok::<_, Error>(None)
                        }
                    }
                })
            })
            .await?
    }
    .ok_or(Error::Proxy(handle.clone()))?;

    let filtered_feed = filter.apply(feed)?;

    let feed_str = filtered_feed
        .into_atom(&base_url, &handle, &filter.query_string()?)
        .to_string();

    Ok(Response::builder()
        // officially the atom MIME type is application/atom+xml, but text/xml is more widely supported
        .header("Content-Type", "text/xml")
        .body(Body::from(feed_str))
        .unwrap())
}
