use std::{collections::HashMap, time::Duration};

use elasticsearch::{
    auth::Credentials,
    cat::{CatAliasesParts, CatIndicesParts},
    cluster::ClusterHealthParts,
    http::transport::Transport,
    indices::IndicesGetParts,
    params::{Bytes, Level},
};
use error_stack::{IntoReport, Report, ResultExt};
use futures::{FutureExt, TryFutureExt};
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
            .map(|result| result.and_then(|res| res.error_for_status_code()))
            .and_then(|res| res.json::<response::CatIndices>())
            .await
            .into_report()
            .change_context(ElasticsearchClientError::ApiRequest)
    }

    /// https://www.elastic.co/guide/en/elasticsearch/reference/current/cat-alias.html
    pub(crate) async fn cat_aliases(
        &self,
    ) -> error_stack::Result<response::CatAliases, ElasticsearchClientError> {
        self.inner
            .cat()
            .aliases(CatAliasesParts::None)
            .format("json")
            .local(false)
            .v(true)
            .human(false)
            .request_timeout(self.default_timeout)
            .send()
            .map(|result| result.and_then(|res| res.error_for_status_code()))
            .and_then(|res| res.json::<response::CatAliases>())
            .await
            .into_report()
            .change_context(ElasticsearchClientError::ApiRequest)
    }

    pub(crate) async fn get_index(
        &self,
        index: &str,
    ) -> error_stack::Result<response::Index, ElasticsearchClientError> {
        type Payload = HashMap<String, response::Index>;

        let mut payload = self
            .inner
            .indices()
            .get(IndicesGetParts::Index(&[index]))
            .include_defaults(false) // should true ?
            .local(false)
            .request_timeout(self.default_timeout)
            .send()
            .map(|result| result.and_then(|res| res.error_for_status_code()))
            .and_then(|res| res.json::<Payload>())
            .await
            .into_report()
            .change_context(ElasticsearchClientError::ApiRequest)?;

        match payload.remove(index) {
            Some(res) => Ok(res),
            None => Err(Report::new(ElasticsearchClientError::ApiRequest))
                .attach_printable_lazy(|| "response does not contain expected index"),
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn dump_index(
        &self,
        index: &str,
    ) -> error_stack::Result<String, ElasticsearchClientError> {
        let r = self
            .inner
            .indices()
            .get(IndicesGetParts::Index(&[index]))
            .include_defaults(false) // should true ?
            .local(false)
            .flat_settings(false)
            .request_timeout(self.default_timeout)
            .send()
            .map(|result| result.and_then(|res| res.error_for_status_code()))
            .and_then(|res| res.json::<serde_json::Value>())
            .await
            .into_report()
            .change_context(ElasticsearchClientError::ApiRequest)?;

        Ok(serde_json::to_string_pretty(&r).unwrap())
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
