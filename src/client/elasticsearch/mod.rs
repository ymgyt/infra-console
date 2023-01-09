use std::time::Duration;

use elasticsearch::{
    auth::Credentials,
    cat::CatIndicesParts,
    cluster::ClusterHealthParts,
    http::transport::Transport,
    indices::IndicesGetParts,
    params::{Bytes, ExpandWildcards, Level},
};
use error_stack::{IntoReport, ResultExt};
use thiserror::Error;

use crate::ElasticsearchConfig;

pub(crate) mod response;

#[derive(Debug)]
pub struct ElasticsearchClient {
    name: String,
    inner: elasticsearch::Elasticsearch,
    default_timeout: Duration,
}

#[derive(Debug, Error)]
pub(crate) enum ElasticsearchClientError {
    #[error("build client error")]
    BuildClient,
    #[error("api request error")]
    ApiRequest,
    #[error("deserialize response")]
    DeserializeResponse,
}

impl ElasticsearchClient {
    pub(crate) fn new(
        c: ElasticsearchConfig,
    ) -> error_stack::Result<Self, ElasticsearchClientError> {
        let transport = match c.credential.cloud_id {
            Some(cloud_id) => Transport::cloud(
                cloud_id.as_str(),
                Credentials::Basic(c.credential.username, c.credential.password),
            )
            .into_report()
            .change_context(ElasticsearchClientError::BuildClient)?,
            None => {
                return Err(error_stack::report!(ElasticsearchClientError::BuildClient))
                    .attach_printable("currently only cloud id credential supported")
            }
        };

        Ok(ElasticsearchClient {
            name: c.name,
            inner: elasticsearch::Elasticsearch::new(transport),
            default_timeout: Duration::from_secs(20),
        })
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    // https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-health.html
    pub(crate) async fn get_cluster_health(
        &self,
    ) -> error_stack::Result<response::ClusterHealth, ElasticsearchClientError> {
        self.inner
            .cluster()
            .health(ClusterHealthParts::None)
            .level(Level::Cluster)
            .local(false)
            .request_timeout(self.default_timeout)
            .send()
            .await
            .into_report()
            .change_context(ElasticsearchClientError::ApiRequest)?
            .json::<response::ClusterHealth>()
            .await
            .into_report()
            .change_context(ElasticsearchClientError::DeserializeResponse)
    }

    pub(crate) async fn cat_indices(
        &self,
    ) -> error_stack::Result<response::CatIndices, ElasticsearchClientError> {
        self.inner
            .cat()
            .indices(CatIndicesParts::None)
            .bytes(Bytes::B)
            .format("json")
            .include_unloaded_segments(false) // should true ?
            .v(false) // ignored in case of json.
            .human(false) // ignored in case of json.
            .request_timeout(self.default_timeout)
            .send()
            .await
            .into_report()
            .change_context(ElasticsearchClientError::ApiRequest)?
            .json::<response::CatIndices>()
            .await
            .into_report()
            .change_context(ElasticsearchClientError::DeserializeResponse)
    }

    // https://www.elastic.co/guide/en/elasticsearch/reference/8.5/indices-get-index.html
    pub async fn get_all_indices(&self) {
        //   features()の引数に複数のFeatureを渡せないので、default値のaliases,mappings,settingsを暗黙的に利用する
        let response = self
            .inner
            .indices()
            .get(IndicesGetParts::Index(&["*"]))
            .allow_no_indices(true)
            .expand_wildcards(&[ExpandWildcards::Open])
            .flat_settings(false)
            .include_defaults(false) // 挙動どうなる?
            .ignore_unavailable(false)
            .local(false)
            .master_timeout(Duration::from_secs(10).into_time_unit().as_str())
            .send()
            .await
            .unwrap();

        let body = response.json::<serde_json::Value>().await.unwrap();
        let pretty = serde_json::to_string_pretty(&body).unwrap();
        println!("{pretty}");

        // let body = response.json::<HashMap<String,serde_json::Value>>().await.unwrap();
        // let v = body.keys().collect::<Vec<_>>();
        // println!("{v:#?}");
    }
}

// Elasticsearch apiの時間の指定方法。
// https://www.elastic.co/guide/en/elasticsearch/reference/8.5/api-conventions.html#time-units
trait TimeUnit {
    fn into_time_unit(self) -> String;
}

impl TimeUnit for std::time::Duration {
    fn into_time_unit(self) -> String {
        format!("{}s", self.as_secs())
    }
}
