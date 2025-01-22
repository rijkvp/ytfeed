use crate::error::Error;
use reqwest::{Client, StatusCode};
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub id: String,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub title: String,
    pub description: String,
    pub id: String,
    pub handle: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Extraction {
    pub channel: ChannelInfo,
    pub videos: Vec<VideoInfo>,
}

/// Extracts channel data and video information by scraping the YouTube website
pub async fn extract_data(channel_name: &str, client: &Client) -> Result<Extraction, Error> {
    let has_at = channel_name.starts_with('@');
    let url = if has_at {
        format!("https://www.youtube.com/{}/videos", channel_name)
    } else {
        format!("https://www.youtube.com/channel/{}/videos", channel_name)
    };
    let response = client.get(&url).send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(Error::ChannelNotFound(channel_name.to_string()));
    }
    let text = response.error_for_status()?.text().await?;
    let html = Html::parse_fragment(&text);
    let script_selector = Selector::parse("script").unwrap();
    for element in html.select(&script_selector) {
        let script = element.inner_html();
        let script = script.trim();
        if script.starts_with("var ytInitialData") {
            let json = script
                .strip_prefix("var ytInitialData = ")
                .ok_or_else(|| Error::Scrape(String::from("Failed to strip prefix")))?
                .strip_suffix(';')
                .ok_or_else(|| Error::Scrape(String::from("Failed to strip suffix")))?;
            let data: Value = serde_json::from_str(json)?;
            let meta_data = &data["metadata"]["channelMetadataRenderer"];
            let vanity_url = meta_data["vanityChannelUrl"]
                .as_str()
                .expect("expected string")
                .to_string();
            let handle = vanity_url.split('/').last().unwrap().to_string();
            let channel = ChannelInfo {
                title: meta_data["title"].as_str().unwrap().to_string(),
                id: meta_data["externalId"].as_str().unwrap().to_string(),
                handle,
                description: meta_data["description"].as_str().unwrap().to_string(),
                url,
            };
            let video_tab = &data["contents"]["twoColumnBrowseResultsRenderer"]["tabs"][1];
            let videos_parent =
                &video_tab["tabRenderer"]["content"]["richGridRenderer"]["contents"];
            let mut videos = Vec::new();
            for item in videos_parent.as_array().unwrap() {
                if let Some(item_renderer) = item.get("richItemRenderer") {
                    if let Some(video_renderer) = item_renderer["content"].get("videoRenderer") {
                        let id = video_renderer["videoId"].as_str().unwrap().to_string();
                        let length_text =
                            video_renderer["lengthText"]["simpleText"].as_str().unwrap();
                        let parts: Vec<&str> = length_text.split(':').collect();
                        let duration = if parts.len() == 3 {
                            let hours: u64 = parts[0].parse().unwrap();
                            let minutes: u64 = parts[1].parse().unwrap();
                            let seconds: u64 = parts[2].parse().unwrap();
                            Duration::from_secs(hours * 3600 + minutes * 60 + seconds)
                        } else if parts.len() == 2 {
                            let minutes: u64 = parts[0].parse().unwrap();
                            let seconds: u64 = parts[1].parse().unwrap();
                            Duration::from_secs(minutes * 60 + seconds)
                        } else {
                            return Err(Error::Scrape(
                                "Invalid number of parts in length text".to_string(),
                            ));
                        };
                        let video = VideoInfo { id, duration };
                        videos.push(video);
                    }
                }
            }
            tracing::debug!("scraped {} videos from '{}'", videos.len(), channel.title);
            return Ok(Extraction { channel, videos });
        }
    }
    Err(Error::ChannelNotFound(channel_name.to_string()))
}
