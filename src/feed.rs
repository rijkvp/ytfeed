use crate::extractor::VideoInfo;
use atom_syndication::{
    Entry, EntryBuilder, Feed as AtomFeed, FeedBuilder, LinkBuilder, PersonBuilder, Text,
};
use chrono::{DateTime, FixedOffset, Utc};
use linkify::{LinkFinder, LinkKind};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Feed {
    pub channel: Channel,
    pub videos: Vec<Video>,
}

impl Feed {
    pub fn into_atom(self, base_url: &str, handle: &str, query_string: &str) -> AtomFeed {
        let mut self_link = base_url.to_string() + "@" + handle;
        if !query_string.is_empty() {
            self_link.push('?');
            self_link.push_str(query_string);
        };

        FeedBuilder::default()
            .title(self.channel.title.clone())
            .updated(
                self.videos
                    .iter()
                    .map(|v| v.updated)
                    .max()
                    .unwrap_or_else(|| Utc::now().fixed_offset()),
            )
            .author(
                PersonBuilder::default()
                    .name(self.channel.title)
                    .uri(Some(self.channel.url))
                    .build(),
            )
            .id(self_link.clone())
            .link(LinkBuilder::default().href(self_link).rel("self").build())
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
                            .summary(Text::html(text_to_html(&v.description)))
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
                tracing::warn!("Failed to parse media field 'views'");
                0
            });
        let title = get_media_field(&group.children["title"][0].value, "title");
        let description = get_media_field(&group.children["description"][0].value, "description");
        Self {
            id: info.id,
            published: entry.published.unwrap(),
            updated: entry.updated,
            title,
            description,
            duration: info.duration,
            likes,
            views,
        }
    }
}

/// Convert plain text to HTML: escape it, preserve line breaks and make links clickable.
fn text_to_html(text: &str) -> String {
    let mut finder = LinkFinder::new();
    finder.url_must_have_scheme(false);
    let mut out = String::with_capacity(text.len());
    for span in finder.spans(text) {
        let escaped = escape_html(span.as_str());
        match span.kind() {
            Some(LinkKind::Url) => {
                let s = span.as_str();
                let lower = s.to_ascii_lowercase();
                let href = if lower.starts_with("http://") || lower.starts_with("https://") {
                    escaped.clone()
                } else if !s.contains("://") {
                    format!("https://{escaped}")
                } else {
                    // Only link http(s) to avoid e.g. javascript:// hrefs
                    out.push_str(&escaped);
                    continue;
                };
                out.push_str(&format!("<a href=\"{href}\">{escaped}</a>"));
            }
            Some(LinkKind::Email) => {
                out.push_str(&format!("<a href=\"mailto:{escaped}\">{escaped}</a>"));
            }
            _ => out.push_str(&escaped),
        }
    }
    out.replace('\n', "<br>\n")
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn get_media_field(option: &Option<String>, field_name: &str) -> String {
    option.as_ref().map(|s| s.to_string()).unwrap_or_else(|| {
        tracing::warn!("Failed to parse media field '{}'", field_name);
        format!("ytfeed: Failed to parse field '{}'", field_name)
    })
}

#[cfg(test)]
mod tests {
    use super::text_to_html;

    #[test]
    fn escapes_and_keeps_line_breaks() {
        assert_eq!(text_to_html("a & b\n<x>"), "a &amp; b<br>\n&lt;x&gt;");
    }

    #[test]
    fn links_urls() {
        assert_eq!(
            text_to_html("See https://example.com/a?b=1&c=2."),
            "See <a href=\"https://example.com/a?b=1&amp;c=2\">https://example.com/a?b=1&amp;c=2</a>."
        );
        assert_eq!(
            text_to_html("(http://x.org/wiki/Foo_(bar))"),
            "(<a href=\"http://x.org/wiki/Foo_(bar)\">http://x.org/wiki/Foo_(bar)</a>)"
        );
        assert_eq!(
            text_to_html("1. https://a.com\n2. https://b.com"),
            "1. <a href=\"https://a.com\">https://a.com</a><br>\n2. <a href=\"https://b.com\">https://b.com</a>"
        );
        assert_eq!(text_to_html("just https:// here"), "just https:// here");
        assert_eq!(
            text_to_html("Visit www.example.com or mail a@b.com"),
            "Visit <a href=\"https://www.example.com\">www.example.com</a> or mail <a href=\"mailto:a@b.com\">a@b.com</a>"
        );
        assert_eq!(
            text_to_html("javascript://x%0aalert(1) ftp://a.com/f"),
            "javascript://x%0aalert(1) ftp://a.com/f"
        );
    }
}
