use crate::{error::Error, Channel, ChannelInfo, Thumbnail, Video};
use chrono::NaiveDate;
use reqwest::{Client, StatusCode};
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;
use tracing::debug;

pub async fn extract_channel_data(channel_id: &str, client: &Client) -> Result<ChannelInfo, Error> {
    let response = client
        .get(format!("https://www.youtube.com/{}/videos", channel_id))
        .send()
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(Error::ChannelNotFound(channel_id.to_string()));
    }
    let text = response.error_for_status()?.text().await?;
    let html = Html::parse_fragment(&text);
    let selector = Selector::parse("script").unwrap();
    for element in html.select(&selector) {
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
            let channel = Channel {
                title: meta_data["title"].as_str().unwrap().to_string(),
                id: meta_data["externalId"].as_str().unwrap().to_string(),
                description: meta_data["description"].as_str().unwrap().to_string(),
                url: meta_data["channelUrl"].as_str().unwrap().to_string(),
            };
            let video_tab = &data["contents"]["twoColumnBrowseResultsRenderer"]["tabs"][1];
            let videos_parent =
                &video_tab["tabRenderer"]["content"]["richGridRenderer"]["contents"];
            let mut videos = Vec::new();
            for item in videos_parent.as_array().unwrap() {
                if let Some(item_renderer) = item.get("richItemRenderer") {
                    if let Some(video_renderer) = item_renderer["content"].get("videoRenderer") {
                        let id = video_renderer["videoId"].as_str().unwrap().to_string();
                        let title = video_renderer["title"]["runs"][0]["text"]
                            .as_str()
                            .unwrap()
                            .to_string();
                        let description = video_renderer["descriptionSnippet"]["runs"][0]["text"]
                            .as_str()
                            .unwrap()
                            .to_string();

                        let length_text =
                            video_renderer["lengthText"]["simpleText"].as_str().unwrap();
                        let mut parts = length_text.split(':');
                        let minutes: u64 = parts.next().unwrap().parse().unwrap();
                        let seconds: u64 = parts.next().unwrap().parse().unwrap();
                        let length = Duration::from_secs(minutes * 60 + seconds);

                        let views_text = video_renderer["viewCountText"]["simpleText"]
                            .as_str()
                            .unwrap();
                        let views: u64 = views_text
                            .split(' ')
                            .next()
                            .unwrap()
                            .replace(',', "")
                            .parse()
                            .unwrap();

                        let thumbnails: Vec<Thumbnail> = serde_json::from_value(
                            video_renderer["thumbnail"]["thumbnails"].clone(),
                        )?;
                        let video = Video {
                            id,
                            title,
                            date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                            description,
                            length,
                            views,
                            thumbnails,
                        };
                        videos.push(video);
                    }
                }
            }
            debug!("Extracted {} videos from '{}'", videos.len(), channel.title);
            return Ok(ChannelInfo { channel, videos });
        }
    }
    Err(Error::ChannelNotFound(channel_id.to_string()))
}
