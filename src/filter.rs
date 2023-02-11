use std::{fmt::Display, ops::Range};

use serde::Deserialize;
use ytextract::Video;

#[derive(Deserialize)]
pub struct Filter {
    #[serde(alias = "c")]
    pub count: Option<usize>,
    #[serde(alias = "d", with = "range_format_opt", default)]
    pub duration: Option<Range<u64>>,
    #[serde(alias = "l", with = "range_format_opt", default)]
    pub likes: Option<Range<u64>>,
    #[serde(alias = "v", with = "range_format_opt", default)]
    pub views: Option<Range<u64>>,
    #[serde(alias = "e")]
    pub live: Option<bool>,
    #[serde(alias = "t")]
    pub tag: Option<String>,
}

pub trait RangeExt<T> {
    fn to_string(&self) -> String;
}

impl<T: Display> RangeExt<T> for Range<T> {
    fn to_string(&self) -> String {
        format!("{}-{}", self.start, self.end)
    }
}

pub trait RangeNum {
    fn start() -> Self;
    fn end() -> Self;
}

impl RangeNum for u64 {
    fn start() -> Self {
        Self::MIN
    }
    fn end() -> Self {
        Self::MAX
    }
}

impl Filter {
    pub fn filter_video(&self, v: &Video) -> bool {
        if let Some(duration_range) = &self.duration {
            if !duration_range.contains(&v.duration().as_secs()) {
                return false;
            }
        }
        if let Some(likes_range) = &self.likes {
            if let Some(likes) = v.likes() {
                if !likes_range.contains(&likes) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(views_range) = &self.views {
            if !views_range.contains(&v.views()) {
                return false;
            }
        }
        if let Some(live) = self.live {
            if live != v.live() {
                return false;
            }
        }
        if let Some(tag_match) = &self.tag {
            let tags: Vec<String> = v
                .hashtags()
                .filter_map(|t| {
                    let s = t
                        .trim()
                        .to_lowercase()
                        .replace(|c: char| c == '#' || !c.is_ascii(), "");
                    if !s.is_empty() {
                        Some(s)
                    } else {
                        None
                    }
                })
                .collect();
            if !tags.contains(&tag_match) {
                return false;
            }
        }
        true
    }

    pub fn id(&self) -> String {
        let mut str = String::new();
        if let Some(count) = self.count {
            str.push('c');
            str.push_str(&count.to_string());
        }
        if let Some(duration) = &self.duration {
            str.push('d');
            str.push_str(&duration.to_string());
        }
        if let Some(likes) = &self.likes {
            str.push('l');
            str.push_str(&likes.to_string());
        }
        if let Some(views) = &self.views {
            str.push('v');
            str.push_str(&views.to_string());
        }
        if let Some(live) = &self.live {
            str.push('e');
            str.push_str(&live.to_string());
        }
        if let Some(tag) = &self.tag {
            str.push('t');
            str.push_str(tag);
        }
        str
    }
}

mod range_format_opt {
    use super::{range_format, RangeNum};
    use serde::{de::Error, Deserializer};
    use std::{fmt::Display, ops::Range, str::FromStr};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Range<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr + RangeNum,
        <T as FromStr>::Err: Display,
    {
        match range_format::deserialize(deserializer) {
            Ok(dur) => Ok(Some(dur)),
            Err(err) => Err(Error::custom(err)),
        }
    }
}

mod range_format {
    use super::RangeNum;
    use serde::{de::Error, Deserialize, Deserializer};
    use std::{fmt::Display, ops::Range, str::FromStr};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Range<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr + RangeNum,
        <T as FromStr>::Err: Display,
    {
        let str = String::deserialize(deserializer)?;
        let center = str
            .find("-")
            .ok_or(Error::custom("missing '-' split on range"))?;
        let start_slice = &str[..center];
        let end_slice = &str[center + 1..];
        if start_slice.is_empty() && end_slice.is_empty() {
            return Err(Error::custom("specify at least a start or end range"));
        }
        let start = if start_slice.is_empty() {
            T::start()
        } else {
            start_slice
                .parse::<T>()
                .map_err(|e| Error::custom(format!("failed to parse start: {}", e)))?
        };
        let end = if end_slice.is_empty() {
            T::end()
        } else {
            end_slice
                .parse::<T>()
                .map_err(|e| Error::custom(format!("failed to parse end: {}", e)))?
        };
        Ok(Range { start, end })
    }
}
