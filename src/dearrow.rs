use reqwest::Client;
use serde::Deserialize;

use crate::error::Error;

#[derive(Debug, Deserialize)]
struct Titles {
    titles: Vec<Title>,
}

#[derive(Debug, Deserialize)]
struct Title {
    title: String,
    original: bool,
}

pub async fn get_dearrow_tile(video_id: &str, client: &Client) -> Result<Option<String>, Error> {
    let url = format!("https://sponsor.ajay.app/api/branding?videoID={}", video_id);
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?
        .text()
        .await?;
    let titles: Titles = serde_json::from_str(&response)?;
    Ok(titles
        .titles
        .into_iter()
        .find(|t| !t.original)
        .map(|t| t.title))
}
