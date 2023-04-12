use crate::extractor::VideoInfo;
use atom_syndication::Entry;
use serde::Deserialize;
use std::{fmt::Display, ops::Range};

#[derive(Deserialize)]
pub struct Filter {
    #[serde(alias = "c")]
    pub count: Option<usize>,
    #[serde(alias = "l", with = "range_format_opt", default)]
    pub duration: Option<Range<u64>>,
    #[serde(alias = "v", with = "range_format_opt", default)]
    pub views: Option<Range<u64>>,
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
    pub fn filter_entry(&self, _e: &Entry, i: &VideoInfo) -> bool {
        // TODO: Add filters back
        if let Some(duration_range) = &self.duration {
            if !duration_range.contains(&i.length.as_secs()) {
                return false;
            }
        }
        if let Some(views_range) = &self.views {
            if !views_range.contains(&i.views) {
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
        if let Some(views) = &self.views {
            str.push('v');
            str.push_str(&views.to_string());
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
            .find('-')
            .ok_or_else(|| Error::custom("missing '-' split on range"))?;
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
