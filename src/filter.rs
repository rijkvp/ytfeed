use serde::Deserialize;
use ytextract::Video;

#[derive(Deserialize)]
pub struct Filter {
    #[serde(alias = "c")]
    pub count: Option<usize>,
    #[serde(alias = "l")]
    pub min_length: Option<u64>,
}

impl Filter {
    pub fn filter_video(&self, v: &Video) -> bool {
        if Some(v.duration().as_secs()) > self.min_length {
            return true;
        }
        false
    }

    pub fn id(&self) -> String {
        let mut str = String::new();
        if let Some(count) = self.count {
            str.push('c');
            str.push_str(&count.to_string());
        }
        if let Some(min_length) = self.min_length {
            str.push('l');
            str.push_str(&min_length.to_string());
        }
        str
    }
}

