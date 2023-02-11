mod cache;
mod error;
mod feed;
mod filter;

use crate::{cache::Cache, error::Error};
use axum::{
    body::boxed,
    extract::{Path, Query},
    response::Response,
    routing::get,
    Extension, Router,
};
use clap::Parser;
use filter::Filter;
use futures::StreamExt;
use parking_lot::Mutex;
use reqwest::Client as HTTPClient;
use scraper::{Html, Selector};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use ytextract::{Channel, Client as YTClient, Video};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Config {
    /// IP bind address
    #[arg(short = 'b', long = "bind", default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000))]
    bind_address: SocketAddr,

    /// Maximum amount of videos to fetch per channel
    #[arg(short = 'l', long = "limit", default_value_t = 20)]
    video_limit: usize,

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

    let bind_address = config.bind_address.clone();
    let app = Router::new()
        .route("/:channel_id", get(get_feed))
        .layer(Extension(YTClient::new()))
        .layer(Extension(HTTPClient::new()))
        .layer(Extension(Cache::<String, Option<String>>::new(None)))
        .layer(Extension(Cache::<String, Option<ChannelInfo>>::new(Some(
            Duration::from_secs(config.cache_timeout.clone()),
        ))))
        .layer(Extension(config));

    info!("Starting server at http://{}", bind_address);

    axum::Server::bind(&bind_address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct ChannelInfo {
    channel: Channel,
    videos: Vec<Video>,
}

async fn get_feed(
    Path(channel_name): Path<String>,
    Query(filter): Query<Filter>,
    Extension(config): Extension<Config>,
    Extension(yt_client): Extension<YTClient>,
    Extension(http_client): Extension<HTTPClient>,
    Extension(id_cache): Extension<Cache<String, Option<String>>>,
    Extension(video_cache): Extension<Cache<String, Option<ChannelInfo>>>,
) -> Result<Response, Error> {
    // Get channel id
    let channel_id = {
        let channel_name = channel_name.clone();
        if channel_name.starts_with("@") {
            id_cache
                .get_cached(channel_name.clone(), || {
                    Box::pin(async move {
                        match get_channel_id(&channel_name, &http_client).await {
                            Ok(id) => {
                                debug!("Got channel id '{id}' for '{channel_name}'");
                                Ok::<_, Error>(Some(id))
                            }
                            Err(err) => {
                                error!("Failed to get channel_id for '{channel_name}': {err}");
                                Ok::<_, Error>(None)
                            }
                        }
                    })
                })
                .await?
        } else {
            Some(channel_name)
        }
    };
    let channel_id = channel_id.ok_or_else(|| Error::ChannelNotFound(channel_name))?;

    let info = {
        let channel_id = channel_id.clone();
        video_cache
            .get_cached(channel_id.clone(), || {
                Box::pin(async move {
                    match get_channel_info(yt_client, &channel_id, config.video_limit).await {
                        Ok(c) => Ok::<_, Error>(Some(c)),
                        Err(err) => {
                            error!("Failed to extract videos from channel '{channel_id}': {err}");
                            Ok::<_, Error>(None)
                        }
                    }
                })
            })
            .await?
    };
    let info = info.ok_or_else(|| Error::Extraction(channel_id))?;

    let feed = feed::convert_feed(info.videos, info.channel, filter);
    Ok(Response::builder()
        .header("Content-Type", "text/xml")
        .body(boxed(feed))
        .unwrap())
}

/// Get the channel info from a channel id
async fn get_channel_info(
    yt_client: YTClient,
    channel_id: &str,
    video_count: usize,
) -> Result<ChannelInfo, Error> {
    let channel = yt_client.channel(channel_id.parse()?).await?;
    let videos: Arc<Mutex<Vec<Video>>> = Arc::new(Mutex::new(Vec::new()));
    channel
        .uploads()
        .await?
        .take(video_count)
        .for_each_concurrent(32, |video| {
            let videos = videos.clone();
            async move {
                match video {
                    Ok(video) => match video.upgrade().await {
                        Ok(video) => {
                            videos.lock().push(video);
                        }
                        Err(e) => error!("Failed to get video info: {e}"),
                    },
                    Err(e) => {
                        error!("Failed to get video: {e}")
                    }
                }
            }
        })
        .await;
    let videos: Vec<Video> = videos.lock().to_vec();
    debug!("Got {} videos for '{}'", videos.len(), channel.name());
    Ok(ChannelInfo { channel, videos })
}

/// Scrapes a channel page to get the channel id
async fn get_channel_id(channel_name: &str, client: &HTTPClient) -> Result<String, Error> {
    let response = client
        .get(format!("https://www.youtube.com/{}", channel_name))
        .send()
        .await?
        .error_for_status()?;
    let text = response.text().await?;
    let html = Html::parse_fragment(&text);
    let selector = Selector::parse("meta").unwrap();
    for element in html.select(&selector) {
        if element.value().attr("itemprop") == Some("channelId") {
            return element
                .value()
                .attr("content")
                .map(|v| v.to_string())
                .ok_or_else(|| Error::Scrape("Missing attribute content".to_string()));
        }
    }
    Err(Error::Scrape("Missing element meta".to_string()))
}
