use crate::{
    error::Error,
    feed::{Feed, Video},
};
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Filter {
    #[serde(rename = "d", default)]
    pub duration: Option<u64>,
    #[serde(rename = "v", default)]
    pub views: Option<u64>,
    #[serde(rename = "l", default)]
    pub likes: Option<u64>,
    #[serde(rename = "lvr", default, skip_serializing_if = "std::ops::Not::not")]
    pub like_view_ratio: bool,
}

impl Filter {
    pub fn apply(&self, mut feed: Feed) -> Result<Feed, Error> {
        let orig_count = feed.videos.len();
        feed.videos.retain_mut(|v| self.filter_video(v));
        if orig_count != feed.videos.len() {
            tracing::debug!("filtered {} videos", orig_count - feed.videos.len());
        }
        Ok(feed)
    }

    fn filter_video(&self, video: &mut Video) -> bool {
        if let Some(min_duration) = &self.duration {
            if video.duration.as_secs() < *min_duration {
                return false;
            }
        }
        if let Some(min_views) = &self.views {
            if video.views < *min_views {
                return false;
            }
        }
        if let Some(min_likes) = &self.likes {
            if video.likes.unwrap_or(0) < *min_likes {
                return false;
            }
        }
        if self.like_view_ratio {
            if let Some(likes) = video.likes {
                let lvr = likes as f64 / video.views as f64 * 100.0;
                video.title = format!("{} [{:.1}]", video.title, lvr);
            }
        }
        self.filter_description(video);
        true
    }

    fn filter_description(&self, video: &mut Video) {
        let text = remove_ads(&video.description);
        let likes_text = video
            .likes
            .map(|l| format!(", 👍 {} likes", l.to_formatted_string(&Locale::en)))
            .unwrap_or_default();
        let info_text = format!(
            "👀 {} views{}, ⏲️  {}",
            video.views.to_formatted_string(&Locale::en),
            likes_text,
            format_duration(&video.duration)
        );
        video.description = info_text + "\n\n" + &text;
    }

    pub fn query_string(&self) -> Result<String, Error> {
        Ok(serde_html_form::to_string(self)?)
    }
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
    " affiliate",
    " affordable ",
    " check out ",
    " coupon code ",
    " discount ",
    " download ",
    " free for ",
    " for free",
    " limited offer ",
    " limited time ",
    " partnership ",
    " promo ",
    " promotion ",
    " purchase ",
    " save ",
    " sign up",
    " sponsor ",
    " sponsored by",
    " sponsoring ",
    " try out ",
    " upgrade at ",
    " upgrade to ",
    " use code",
    " use the code",
    " with code",
    " buy a ",
    " buy an ",
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
