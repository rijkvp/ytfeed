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
use filter::Filter;
use futures::StreamExt;
use parking_lot::Mutex;
use reqwest::Client as HTTPClient;
use scraper::{Html, Selector};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use ytextract::{Channel, Client as YTClient, Video};

const VIDEO_LIMIT: usize = 20;
const VIDEO_TIMEOUT: u64 = 15;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("ytfeed=DEBUG".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    let app = Router::new()
        .route("/videos/:channel_id", get(get_feed))
        .layer(Extension(YTClient::new()))
        .layer(Extension(HTTPClient::new()))
        .layer(Extension(Cache::<String, Option<String>>::new(None)))
        .layer(Extension(Cache::<String, Option<ChannelInfo>>::new(
            Some(Duration::from_secs(VIDEO_TIMEOUT * 60)),
        )));

    info!("Starting at http://0.0.0.0:3000");

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
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

    let video_count = if let Some(count) = filter.count {
        VIDEO_LIMIT.min(count)
    } else {
        VIDEO_LIMIT
    };
    let info = {
        let channel_id = channel_id.clone();
        video_cache
            .get_cached(channel_id.clone(), || {
                Box::pin(async move {
                    match get_channel_info(yt_client, &channel_id, video_count).await {
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
