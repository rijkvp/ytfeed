use crate::{
    error::Error,
    extractor::VideoInfo,
    media::Media,
    proxy::FeedInfo,
    range::{range_format_opt, RangeExt},
};
use atom_syndication::Entry;
use num_format::{Locale, ToFormattedString};
use serde::Deserialize;
use std::{ops::Range, time::Duration};
use tracing::debug;

#[derive(Debug, Clone, Deserialize)]
pub struct Filter {
    #[serde(alias = "c")]
    pub count: Option<usize>,
    #[serde(alias = "d", with = "range_format_opt", default)]
    pub duration: Option<Range<u64>>,
    #[serde(alias = "v", with = "range_format_opt", default)]
    pub views: Option<Range<u64>>,
    #[serde(alias = "l", with = "range_format_opt", default)]
    pub likes: Option<Range<u64>>,
}

impl Filter {
    pub fn filter(&self, i: &VideoInfo, m: &Media) -> bool {
        if let Some(duration_range) = &self.duration {
            if !duration_range.contains(&i.duration.as_secs()) {
                return false;
            }
        }
        if let Some(views_range) = &self.views {
            if !views_range.contains(&m.views) {
                return false;
            }
        }
        if let Some(likes_range) = &self.likes {
            if let Some(likes) = m.likes {
                if !likes_range.contains(&likes) {
                    return false;
                }
            }
        }
        true
    }

    pub fn id(&self) -> String {
        format! {
            "{}{}{}{}",
            self.count.map(|c| format!("c{c}")).unwrap_or_default(),
            self.duration.as_ref().map(|d| format!("d{}", d.display())).unwrap_or_default(),
            self.likes.as_ref().map(|l| format!("l{}", l.display())).unwrap_or_default(),
            self.views.as_ref().map(|v| format!("v{}", v.display())).unwrap_or_default()
        }
    }
}

pub fn filter_feed(chached_feed: FeedInfo, filter: Filter) -> Result<String, Error> {
    let mut feed = chached_feed.feed;
    let extraction = chached_feed.extraction;
    let orig_count = feed.entries.len();
    feed.entries = feed
        .entries
        .into_iter()
        .filter_map(|mut e| {
            if let Some(video_info) = extraction
                .videos
                .iter()
                .find(|v| Some(v.id.clone()) == e.extensions["yt"]["videoId"][0].value)
            {
                let media = Media::from(&e);
                if filter.filter(video_info, &media) {
                    update_description(&mut e, video_info, &media);
                    return Some(e);
                }
            }
            None
        })
        .take(filter.count.unwrap_or(usize::MAX))
        .collect();

    feed.set_id(format!("{}#{}", extraction.channel.id, filter.id()));
    debug!(
        "Filtered to {} videos (from {})",
        feed.entries.len(),
        orig_count
    );
    Ok(feed.to_string())
}

// Adds video information to description and tries to remove ads/sponsors based on keywords
fn update_description(e: &mut Entry, i: &VideoInfo, m: &Media) {
    let description = &mut e
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
    let text = remove_ads(&m.description);
    let likes_text = m
        .likes
        .map(|l| format!(", 👍 {} likes", l.to_formatted_string(&Locale::en)))
        .unwrap_or_default();
    let info_text = format!(
        "👀 {} views{}, ⏲️  {}",
        m.views.to_formatted_string(&Locale::en),
        likes_text,
        format_duration(&i.duration)
    );
    description.value = Some(info_text + "\n\n" + &text);
}

fn format_duration(d: &Duration) -> String {
    let total_secs = d.as_secs();
    let h = total_secs / 3600;
    let m = (total_secs / 60) % 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

const AD_KEYWORDS: &[&str] = &[
    " affiliate ",
    " affordable ",
    " check out ",
    " coupon code ",
    " discount ",
    " download ",
    " free for ",
    " limited offer ",
    " limited time ",
    " partnership ",
    " promo ",
    " promotion ",
    " purchase ",
    " save ",
    " sign up ",
    " sponsor ",
    " sponsored by ",
    " sponsoring ",
    " try out ",
    " upgrade at ",
    " upgrade to ",
    " use code ",
    " use the code ",
    " with code ",
    "% off ",
];

fn remove_ads(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let normalized = " ".to_string()
                + &line
                    .trim()
                    .to_lowercase()
                    .replace(|c: char| !c.is_ascii(), "")
                + " ";
            for kw in AD_KEYWORDS {
                if normalized.contains(kw) {
                    return false;
                }
            }
            true
        })
        .map(|l| l.to_string() + "\n")
        .collect::<String>()
        .trim()
        .to_string()
}
