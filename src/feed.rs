use crate::extractor::VideoInfo;
use atom_syndication::{
    Entry, EntryBuilder, Feed as AtomFeed, FeedBuilder, LinkBuilder, PersonBuilder, Text,
};
use chrono::{DateTime, FixedOffset};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Feed {
    pub channel: Channel,
    pub videos: Vec<Video>,
}

impl Feed {
    pub fn into_atom(self, filter_hash: &str) -> AtomFeed {
        FeedBuilder::default()
            .title(self.channel.title.clone())
            .updated(self.videos.iter().map(|v| v.updated).max().unwrap())
            .author(
                PersonBuilder::default()
                    .name(self.channel.title)
                    .uri(Some(self.channel.url))
                    .build(),
            )
            .id(format!("ytfeed:{}#{}", self.channel.id, filter_hash))
            .entries(
                self.videos
                    .into_iter()
                    .map(|v| {
                        EntryBuilder::default()
                            .link(
                                LinkBuilder::default()
                                    .href(format!("https://www.youtube.com/watch?v={}", v.id))
                                    .rel("alternate")
                                    .build(),
                            )
                            .id(v.id)
                            .title(v.title)
                            .updated(v.updated)
                            .published(v.published)
                            .summary(Text::plain(v.description))
                            .build()
                    })
                    .collect::<Vec<Entry>>(),
            )
            .build()
    }
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub title: String,
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Video {
    pub id: String,
    pub published: DateTime<FixedOffset>,
    pub updated: DateTime<FixedOffset>,
    pub title: String,
    pub description: String,
    pub duration: Duration,
    pub likes: Option<u64>,
    pub views: u64,
}

impl Video {
    pub fn from_entry_and_info(entry: Entry, info: VideoInfo) -> Self {
        let group = &entry.extensions["media"]["group"][0];
        let community = &group.children["community"][0];
        let likes: Option<u64> = community
            .children
            .get("starRating")
            .map(|e| e[0].attrs["count"].parse().unwrap());
        let views: u64 = community.children["statistics"][0].attrs["views"]
            .parse()
            .unwrap_or_else(|_| {
                tracing::warn!("failed to parse media field 'views'");
                0
            });
        let title = get_media_field(&group.children["title"][0].value, "title");
        let description = get_media_field(&group.children["description"][0].value, "description");
        Self {
            id: info.id,
            published: entry.published.unwrap().into(),
            updated: entry.updated.into(),
            title,
            description,
            duration: info.duration,
            likes,
            views,
        }
    }
}

fn get_media_field(option: &Option<String>, field_name: &str) -> String {
    option.as_ref().map(|s| s.to_string()).unwrap_or_else(|| {
        tracing::warn!("Failed to parse media field '{}'", field_name);
        format!("ytfeed: Failed to parse field '{}'", field_name)
    })
}
