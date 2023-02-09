use atom_syndication::{
    extension::ExtensionBuilder, Entry, EntryBuilder, FeedBuilder, Generator, LinkBuilder,
    PersonBuilder, Text,
};
use axum::{
    body::boxed,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use futures::StreamExt;
use reqwest::Client as HTTPClient;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::Hasher,
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use ytextract::{Client as YTClient, Video};

const VIDEO_LIMIT: usize = 20;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive("ytfeed=INFO".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    let yt_client = YTClient::new();
    let http_client = HTTPClient::new();
    let app = Router::new()
        .route("/videos/:channel_id", get(get_feed))
        .layer(Extension(yt_client))
        .layer(Extension(http_client));

    info!("Starting at http://0.0.0.0:3000");

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Error, Debug)]
enum Error {
    #[error("YouTube extractor error: {0}")]
    YTErr(#[from] ytextract::Error),
    #[error("Invalid id: {0}")]
    InvalidId(String),
    #[error("Failed HTTP request: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("{0}")]
    Scrape(String),
}

impl<const N: usize> From<ytextract::error::Id<N>> for Error {
    fn from(value: ytextract::error::Id<N>) -> Self {
        Self::InvalidId(value.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let err = self.to_string();
        info!("Error: {err}");
        let status = match self {
            Error::YTErr(_) | Error::Scrape(_) | Error::HttpRequest(_) => StatusCode::BAD_GATEWAY,
            Error::InvalidId(_) => StatusCode::BAD_REQUEST,
        };
        Response::builder().status(status).body(boxed(err)).unwrap()
    }
}

#[derive(Deserialize, Hash)]
struct Filter {
    #[serde(alias = "c")]
    count: Option<usize>,
    #[serde(alias = "l")]
    min_length: Option<u64>,
}

fn calculate_hash<T: std::hash::Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

async fn get_feed(
    Path(channel_name): Path<String>,
    Query(filter): Query<Filter>,
    Extension(yt_client): Extension<YTClient>,
    Extension(http_client): Extension<HTTPClient>,
) -> Result<Response, Error> {
    let channel_id = if channel_name.starts_with("@") {
        get_channel_id(&channel_name, &http_client).await?
    } else {
        channel_name
    };
    let channel = yt_client.channel(channel_id.parse()?).await?;
    let videos: Arc<Mutex<Vec<Video>>> = Arc::new(Mutex::new(Vec::new()));
    let count = if let Some(count) = filter.count {
        VIDEO_LIMIT.min(count)
    } else {
        VIDEO_LIMIT
    };
    channel
        .uploads()
        .await?
        .take(count)
        .for_each_concurrent(32, |video| {
            let videos = videos.clone();
            async move {
                match video {
                    Ok(video) => match video.upgrade().await {
                        Ok(video) => {
                            videos.lock().await.push(video);
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
    let videos: Vec<Video> = videos.lock().await.to_vec();
    info!("Got {} videos for '{}'", videos.len(), channel.name());

    let entries: Vec<Entry> = videos
        .into_iter()
        .filter(|v| filter_video(v, &filter))
        .map(map_video)
        .collect();
    let last_update = if let Some(e) = entries.get(0) {
        e.updated
    } else {
        FixedOffset::west_opt(0)
            .unwrap()
            .from_utc_datetime(&NaiveDateTime::from_timestamp_opt(0, 0).unwrap())
    };
    let rss = FeedBuilder::default()
        .namespaces(BTreeMap::from([
            (
                "yt".to_string(),
                "http://www.youtube.com/xml/schemas/2015".to_string(),
            ),
            (
                "media".to_string(),
                "http://search.yahoo.com/mrss/".to_string(),
            ),
            (
                "itunes".to_string(),
                "http://www.itunes.com/dtds/podcast-1.0.dtd".to_string(),
            ),
        ]))
        .id(format!(
            "ytfeed-{}-{}",
            channel.id(),
            calculate_hash(&filter).to_string()
        ))
        .title(channel.name())
        .updated(last_update)
        .generator(Some(Generator {
            value: "ytfeed".to_string(),
            version: Some("0.0.1".to_string()),
            ..Default::default()
        }))
        .subtitle(Some(Text::plain(format!(
            "{} YouTube's channel feed",
            channel.name()
        ))))
        .link(
            LinkBuilder::default()
                .href("https://www.youtube.com/channel/".to_string())
                .rel("alternate".to_string())
                .build(),
        )
        .entries(entries)
        .build();
    Ok(Response::builder()
        .header("Content-Type", "text/xml")
        .body(boxed(rss.to_string()))
        .unwrap())
}

fn filter_video(v: &Video, filter: &Filter) -> bool {
    if Some(v.duration().as_secs()) > filter.min_length {
        return true;
    }
    false
}

fn map_video(v: Video) -> Entry {
    let published_date = FixedOffset::west_opt(0)
        .unwrap()
        .from_utc_datetime(&v.date().and_hms_opt(0, 0, 0).unwrap());
    let thumbnail_attrs = BTreeMap::from([
        ("url".to_string(), v.thumbnails()[0].url.to_string()),
        ("width".to_string(), v.thumbnails()[0].width.to_string()),
        ("height".to_string(), v.thumbnails()[0].height.to_string()),
    ]);
    let thumbnail_ext = ExtensionBuilder::default()
        .name("media:thumbnail".to_string())
        .attrs(thumbnail_attrs.clone())
        .build();
    EntryBuilder::default()
        .id(format!("yt:video:{}", v.id()))
        .title(Text::plain(v.title().to_string()))
        .summary(Some(Text::plain(v.description().to_string())))
        .updated(published_date)
        .link(
            LinkBuilder::default()
                .href(format!("https://www.youtube.com/watch?v={}", v.id()))
                .rel("alternate".to_string())
                .build(),
        )
        .published(Some(published_date))
        .author(
            PersonBuilder::default()
                .name(v.channel().name().to_string())
                .uri(Some(format!(
                    "https://www.youtube.com/channel/{}",
                    v.channel().id()
                )))
                .build(),
        )
        .extension((
            "extensions".to_string(),
            BTreeMap::from([
                (
                    "yt".to_string(),
                    vec![
                        ExtensionBuilder::default()
                            .name("yt:videoId".to_string())
                            .value(Some(v.id().to_string()))
                            .build(),
                        ExtensionBuilder::default()
                            .name("yt:channelId".to_string())
                            .value(Some(v.channel().id().to_string()))
                            .build(),
                    ],
                ),
                (
                    "media".to_string(),
                    vec![
                        ExtensionBuilder::default()
                            .name("media:group".to_string())
                            .children(BTreeMap::from([(
                                "media:group".to_string(),
                                vec![
                                    ExtensionBuilder::default()
                                        .name("media:title".to_string())
                                        .value(Some(v.title().to_string()))
                                        .build(),
                                    thumbnail_ext.clone(),
                                    ExtensionBuilder::default()
                                        .name("media:description".to_string())
                                        .value(Some(v.description().to_string()))
                                        .build(),
                                ],
                            )]))
                            .build(),
                        thumbnail_ext,
                        ExtensionBuilder::default()
                            .name("itunes:image".to_string())
                            .attrs(thumbnail_attrs.clone())
                            .build(),
                    ],
                ),
            ]),
        ))
        .build()
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
