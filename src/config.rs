use serde::Deserialize;
use typed_builder::TypedBuilder;
use url::Url;

#[derive(Clone, Debug, Deserialize, TypedBuilder)]
pub struct Config {
    pub(crate) elasticsearch: Option<Vec<ElasticsearchConfig>>,
}

#[derive(Clone, Debug, Deserialize, TypedBuilder)]
pub struct ElasticsearchConfig {
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) endpoint: Url,
    pub(crate) credential: ElasticsearchCredential,
}

#[derive(Clone, Debug, Deserialize, TypedBuilder)]
pub struct ElasticsearchCredential {
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) cloud_id: Option<String>,
}
