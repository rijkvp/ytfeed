use crate::{error::Error, extractor, filter::Filter};
use atom_syndication::Feed;
use bytes::Buf;
use reqwest::Client;
use tokio::join;
use tracing::debug;

pub async fn proxy_feed(
    channel_id: &str,
    filter: Filter,
    client: &Client,
) -> Result<String, Error> {
    let feed_task = fetch_feed(channel_id, client);
    let scrape_task = extractor::extract_data(channel_id, client);
    // Concurrently peform both tasks
    let (feed, info) = join!(feed_task, scrape_task);
    let mut feed = feed?;
    let info = info?;
    debug!("{:#?}", feed.entries);
    debug!("{:#?}", info.videos);
    for x in (0..feed.entries.len()).rev() {
        let entry = feed.entries.get_mut(x).unwrap();
        if let Some(video_info) = info
            .videos
            .iter()
            .find(|v| Some(v.id.clone()) == entry.extensions["yt"]["videoId"][0].value)
        {
            if filter.filter_entry(entry, video_info) {
                // TODO: Fix this mess!!!
                let existing_desc = &mut entry
                    .extensions
                    .get_mut("media")
                    .unwrap()
                    .get_mut("group")
                    .unwrap()
                    .get_mut(0)
                    .unwrap()
                    .children
                    .get_mut("description")
                    .unwrap()
                    .get_mut(0)
                    .unwrap();
                let existing_text = existing_desc
                    .value
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("");
                existing_desc.value = Some(format!(
                    "{} views ({:?})\n{existing_text}",
                    video_info.views, video_info.length
                ));
            } else {
                feed.entries.swap_remove(x);
            }
        } else {
            // Remove the item if it is not found in the scrape (likely a short)
            debug!("Id not found");
            feed.entries.swap_remove(x);
        }
    }
    feed.set_id(info.channel.id + &filter.id());
    debug!("Filtered/proxied to {} videos", feed.entries.len());
    Ok(feed.to_string())
}

async fn fetch_feed(channel_id: &str, client: &Client) -> Result<Feed, Error> {
    let rss_url = format!(
        "https://www.youtube.com/feeds/videos.xml?channel_id={}",
        channel_id
    );
    let feed_bytes = client.get(&rss_url).send().await?.bytes().await?;
    let feed = Feed::read_from(feed_bytes.reader())?;
    Ok(feed)
}
