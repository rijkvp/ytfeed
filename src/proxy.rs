use crate::{
    error::Error,
    extractor::{self},
    feed::{Feed, Video},
};
use atom_syndication::Feed as AtomFeed;
use bytes::Buf;
use reqwest::Client;

/// Proxies a YouTube channel feed, filters out shorts from the RSS feed by extracting video
/// information from the channel page
pub async fn proxy_feed(handle: &str, client: &Client) -> Result<Feed, Error> {
    // 1. extract channel data and video information from YouTube
    let mut extraction = extractor::extract_data(handle, client).await?;
    let extracted_videos_count = extraction.videos.len();
    // 2. Use channel id to fetch feed from YouTube RSS server
    let feed = fetch_feed(&extraction.channel.id, client).await?;
    let feed_entries_count = feed.entries.len();

    // 3. Match & combine both data sources into a single feed
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
        "proxied {} videos ({} extracted, {} feed)",
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
    let feed_url = format!(
        "https://www.youtube.com/feeds/videos.xml?channel_id={}",
        channel_id
    );
    tracing::debug!("fetching feed from {}", feed_url);
    let feed_bytes = client.get(&feed_url).send().await?.bytes().await?;
    let feed = AtomFeed::read_from(feed_bytes.reader())?;
    Ok(feed)
}
