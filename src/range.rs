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

pub mod range_format_opt {
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

    pub fn serialize<S, T>(range: &Option<Range<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: Display,
    {
        match range {
            Some(range) => range_format::serialize(range, serializer),
            None => serializer.serialize_none(),
        }
    }
}

pub mod range_format {
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

    pub fn serialize<S, T>(range: &Range<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: Display,
    {
        serializer.serialize_str(&format!("{}-{}", range.start, range.end))
    }
}
