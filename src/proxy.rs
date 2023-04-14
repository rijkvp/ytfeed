use crate::{
    error::Error,
    extractor::{self, Extraction},
};
use atom_syndication::Feed;
use bytes::Buf;
use reqwest::Client;
use tokio::join;

#[derive(Clone)]
pub struct FeedInfo {
    pub feed: Feed,
    pub extraction: Extraction,
}

pub async fn proxy_feed(channel_name: &str, client: &Client) -> Result<FeedInfo, Error> {
    if channel_name.starts_with('@') {
        // Peform extraction first to aquire channel id
        let extraction = extractor::extract_data(channel_name, client).await?;
        let feed = fetch_feed(&extraction.channel.id, client).await?;
        Ok(FeedInfo { feed, extraction })
    } else {
        // Concurrently peform both tasks
        let feed_task = fetch_feed(channel_name, client);
        let extract_task = extractor::extract_data(channel_name, client);
        let (feed, extraction) = join!(feed_task, extract_task);
        Ok(FeedInfo {
            feed: feed?,
            extraction: extraction?,
        })
    }
}

/// Get a feed from the YouTube RSS server
async fn fetch_feed(channel_id: &str, client: &Client) -> Result<Feed, Error> {
    let feed_url = format!(
        "https://www.youtube.com/feeds/videos.xml?channel_id={}",
        channel_id
    );
    let feed_bytes = client.get(&feed_url).send().await?.bytes().await?;
    let feed = Feed::read_from(feed_bytes.reader())?;
    Ok(feed)
}
