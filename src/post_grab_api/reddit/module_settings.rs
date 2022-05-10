use serde::{Deserialize, Serialize};

pub fn fuzzy_contains<T: PartialEq>(fc: &Option<T>, c: &T) -> bool {
    match fc {
        None => true,
        Some(x) => x == c,
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum ContentType {
    Text,
    Image,
    Gallery,
    Video,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum OriginType {
    Crossposted,
    NonCrossposted,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum NsfwType {
    Nsfw,
    Sfw,
}

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct PostClassification {
    pub content_type: ContentType,
    pub origin_type: OriginType,
    pub nsfw_type: NsfwType,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct FuzzyPostClassification {
    pub content_type: Option<ContentType>,
    pub origin_type: Option<OriginType>,
    pub nsfw_type: Option<NsfwType>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct EmbedSet(pub Vec<FuzzyPostClassification>);

impl EmbedSet {
    pub fn contains(&self, post_class: &PostClassification) -> bool {
        self.0.iter().any(|pc| {
            fuzzy_contains(&pc.content_type, &post_class.content_type)
                && fuzzy_contains(&pc.origin_type, &post_class.origin_type)
                && fuzzy_contains(&pc.nsfw_type, &post_class.nsfw_type)
        })
    }
}
