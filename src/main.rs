mod cache;
mod error;
mod extractor;
mod feed;
mod filter;

use crate::{cache::Cache, error::Error};
use axum::{
    body::boxed,
    extract::{Path, Query},
    http::HeaderMap,
    response::Response,
    routing::get,
    Extension, Router,
};
use chrono::NaiveDate;
use clap::Parser;
use filter::Filter;
use reqwest::Client;
use serde::Deserialize;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tracing::{error, info, debug};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Config {
    /// IP bind address
    #[arg(short = 'b', long = "bind", default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000))]
    bind_address: SocketAddr,

    /// How long to keep videos cached (in seconds)
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
        .layer(Extension(Cache::<String, Option<ChannelInfo>>::new(Some(
            Duration::from_secs(config.cache_timeout),
        ))));

    info!("Starting server at http://{}", bind_address);

    axum::Server::bind(&bind_address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Clone, Deserialize)]
pub struct Thumbnail {
    width: u64,
    height: u64,
    url: String,
}

#[derive(Debug, Clone)]
pub struct Video {
    id: String,
    title: String,
    description: String,
    date: NaiveDate,
    length: Duration,
    views: u64,
    thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Clone)]
struct Channel {
    title: String,
    description: String,
    id: String,
    url: String,
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    channel: Channel,
    videos: Vec<Video>,
}

async fn get_feed(
    Path(channel_id): Path<String>,
    Query(filter): Query<Filter>,
    Extension(http_client): Extension<Client>,
    Extension(video_cache): Extension<Cache<String, Option<ChannelInfo>>>,
) -> Result<Response, Error> {
    debug!("GET feed for '{}'", channel_id);

    let info = {
        let channel_id = channel_id.clone();
        video_cache
            .get_cached(channel_id.clone(), || {
                Box::pin(async move {
                    match extractor::extract_channel_data(&channel_id, &http_client).await {
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
    .ok_or(Error::Extraction(channel_id))?;

    let feed = feed::convert_feed(info, filter);
    Ok(Response::builder()
        .header("Content-Type", "text/xml")
        .body(boxed(feed))
        .unwrap())
}
