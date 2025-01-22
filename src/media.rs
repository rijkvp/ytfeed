use atom_syndication::Entry;

/// The media namespace used in YouTube's Atom feeds
pub struct Media {
    pub likes: Option<u64>,
    pub views: u64,
    pub title: String,
    pub description: String,
}

fn get_field(option: &Option<String>, field_name: &str) -> String {
    option.as_ref().map(|s| s.to_string()).unwrap_or_else(|| {
        tracing::warn!("Failed to parse media field '{}'", field_name);
        format!("ytfeed: Failed to parse field '{}'", field_name)
    })
}

impl From<&Entry> for Media {
    fn from(value: &Entry) -> Self {
        let group = &value.extensions["media"]["group"][0];
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
        let title = get_field(&group.children["title"][0].value, "title");
        let description = get_field(&group.children["description"][0].value, "description");
        Self {
            likes,
            views,
            title,
            description,
        }
    }
}
