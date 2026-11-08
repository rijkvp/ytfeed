use crate::{
    error::Error,
    extractor::{self},
    feed::{Feed, Video},
};
use atom_syndication::Feed as AtomFeed;
use bytes::Buf;
use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Max number of attempts when fetching the RSS feed
const FEED_MAX_ATTEMPTS: u32 = 4;
/// Delay before the first retry, exponential backoff on every retry (0.5s, 1s, 2s)
const FEED_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);

/// Proxies a YouTube channel feed, filters out shorts from the RSS feed by extracting video
/// information from the channel page
pub async fn proxy_feed(handle: &str, client: &Client) -> Result<Feed, Error> {
    // 1. extract channel data and video information from YouTube
    let mut extraction = extractor::extract_data(handle, client).await?;
    let extracted_videos_count = extraction.videos.len();
    // 2. Use channel id to fetch UULF feed from YouTube RSS server
    let feed = fetch_feed(&extraction.channel.id, client).await?;
    let feed_entries_count = feed.entries.len();

    // 3. Match & combine both data sources into a single feed
    // This process automatically filters out shorts since the channel data extraction only contains videos
    let mut videos: Vec<Video> = feed
        .entries
        .into_iter()
        .filter_map(|e| {
            if let Some(video_idx) = extraction
                .videos
                .iter()
                .position(|v| Some(v.id.clone()) == e.extensions["yt"]["videoId"][0].value)
            {
                let video = extraction.videos.swap_remove(video_idx);
                Some(Video::from_entry_and_info(e, video))
            } else {
                None
            }
        })
        .collect();

    tracing::debug!(
        "Proxied {} videos ({} extracted, {} feed)",
        videos.len(),
        extracted_videos_count,
        feed_entries_count
    );

    // sort by published date
    videos.sort_by(|a, b| b.published.cmp(&a.published));

    Ok(Feed {
        channel: extraction.channel,
        videos,
    })
}

/// Get a feed from the YouTube RSS server
async fn fetch_feed(channel_id: &str, client: &Client) -> Result<AtomFeed, Error> {
    let playlist_id = format!(
        "UULF{}",
        channel_id.strip_prefix("UC").unwrap_or(channel_id)
    );
    let feed_url = format!(
        "https://www.youtube.com/feeds/videos.xml?playlist_id={}",
        playlist_id
    );
    tracing::debug!("Fetching feed from '{}'", feed_url);
    let mut attempt = 1;
    loop {
        match try_fetch_feed(&feed_url, client).await {
            Ok(feed) => return Ok(feed),
            Err(err) if attempt < FEED_MAX_ATTEMPTS && is_transient(&err) => {
                let delay = FEED_RETRY_BASE_DELAY * 2u32.pow(attempt - 1);
                tracing::warn!(
                    "Transient error fetching feed (attempt {attempt}/{FEED_MAX_ATTEMPTS}), retrying in {delay:?}: {err}"
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

async fn try_fetch_feed(feed_url: &str, client: &Client) -> Result<AtomFeed, Error> {
    let response = client.get(feed_url).send().await?.error_for_status()?;
    let feed_bytes = response.bytes().await?;
    let feed = AtomFeed::read_from(feed_bytes.reader())?;
    Ok(feed)
}

// The YouTube RSS endpoint sometimes briefly returns 404, hence it is treated as transient
fn is_transient(err: &Error) -> bool {
    match err {
        Error::HttpRequest(e) => e
            .status()
            .map(|s| {
                s.is_server_error()
                    || s == StatusCode::TOO_MANY_REQUESTS
                    || s == StatusCode::NOT_FOUND
            })
            .unwrap_or(true),
        Error::Feed(_) => true,
        _ => false,
    }
}
