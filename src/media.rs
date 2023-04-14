use atom_syndication::Entry;

/// The media namespace used in YouTube's Atom feeds
pub struct Media {
    pub likes: Option<u64>,
    pub views: u64,
    pub title: String,
    pub description: String,
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
            .unwrap();
        let title = group.children["title"][0]
            .value
            .as_ref()
            .unwrap()
            .to_string();
        let description = group.children["description"][0]
            .value
            .as_ref()
            .unwrap()
            .to_string();
        Self {
            likes,
            views,
            title,
            description,
        }
    }
}
