use crate::filter::Filter;
use atom_syndication::{
    extension::ExtensionBuilder, Entry, EntryBuilder, FeedBuilder, Generator, LinkBuilder,
    PersonBuilder, Text,
};
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use tracing::debug;
use std::collections::BTreeMap;
use ytextract::{Channel, Video};

pub fn convert_feed(videos: Vec<Video>, channel: Channel, filter: Filter) -> String {
    let entries: Vec<Entry> = videos
        .into_iter()
        .filter(|v| filter.filter_video(v))
        .take(filter.count.unwrap_or(usize::MAX))
        .map(map_video)
        .collect();
    debug!("Filtered to {} videos", entries.len());
    let last_update = if let Some(e) = entries.get(0) {
        e.updated
    } else {
        FixedOffset::west_opt(0)
            .unwrap()
            .from_utc_datetime(&NaiveDateTime::from_timestamp_opt(0, 0).unwrap())
    };
    let feed = FeedBuilder::default()
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
        .id(format!("ytfeed-{}-{}", channel.id(), filter.id()))
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
    feed.to_string()
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
