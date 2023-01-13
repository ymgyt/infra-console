use std::collections::HashMap;

use error_stack::{Report, ResultExt};

use crate::{
    client::elasticsearch::{
        response::{CatAliases, CatIndices, ClusterHealth, Index},
        ElasticsearchClient, ElasticsearchClientError,
    },
    config::ElasticsearchConfig,
    event::api::ApiHandleError,
};

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum ElasticsearchRequestEvent {
    FetchCluster { cluster_name: String },
    FetchIndices { cluster_name: String },
    FetchAliases { cluster_name: String },
    FetchIndex { cluster_name: String, index: String },
}

#[derive(Debug, Clone)]
pub(crate) enum ElasticsearchResponseEvent {
    ClusterHealth {
        cluster_name: String,
        response: ClusterHealth,
    },
    Indices {
        cluster_name: String,
        response: CatIndices,
    },
    Aliases {
        cluster_name: String,
        response: CatAliases,
    },
    Index {
        cluster_name: String,
        index: String,
        response: Index,
    },
}

pub(crate) struct ElasticsearchApiHandler {
    clients: HashMap<String, ElasticsearchClient>,
}

impl ElasticsearchApiHandler {
    pub(crate) fn new(
        configs: Vec<ElasticsearchConfig>,
    ) -> error_stack::Result<Self, ElasticsearchClientError> {
        let clients = configs
            .into_iter()
            .map(ElasticsearchClient::new)
            .collect::<Result<Vec<ElasticsearchClient>, _>>()?
            .into_iter()
            .fold(HashMap::new(), |mut h, client| {
                h.insert(client.name().to_owned(), client);
                h
            });

        Ok(ElasticsearchApiHandler { clients })
    }

    pub(crate) async fn handle(
        &self,
        req: ElasticsearchRequestEvent,
    ) -> error_stack::Result<ElasticsearchResponseEvent, ApiHandleError> {
        use ElasticsearchRequestEvent::*;
        match req {
            FetchCluster { cluster_name } => {
                let client = self.lookup_cluster(&cluster_name)?;

                tracing::info!("Fetch cluster info...");

                client
                    .get_cluster_health()
                    .await
                    .map(|health| ElasticsearchResponseEvent::ClusterHealth {
                        cluster_name,
                        response: health,
                    })
                    .change_context(ApiHandleError::Elasticsearch)
            }
            FetchIndices { cluster_name } => {
                let client = self.lookup_cluster(&cluster_name)?;

                tracing::info!("Fetch indices...");

                client
                    .cat_indices()
                    .await
                    .map(|indices| ElasticsearchResponseEvent::Indices {
                        cluster_name,
                        response: indices,
                    })
                    .change_context(ApiHandleError::Elasticsearch)
            }
            FetchAliases { cluster_name } => {
                let client = self.lookup_cluster(&cluster_name)?;

                tracing::info!("Fetch aliases...");

                client
                    .cat_aliases()
                    .await
                    .map(|aliases| ElasticsearchResponseEvent::Aliases {
                        cluster_name,
                        response: aliases,
                    })
                    .change_context(ApiHandleError::Elasticsearch)
            }
            FetchIndex {
                cluster_name,
                index,
            } => {
                let client = self.lookup_cluster(&cluster_name)?;

                tracing::info!("Fetch index...");

                // let dump = client.dump_index(index.as_str()).await.unwrap();
                //
                // tracing::info!("{dump}");

                client
                    .get_index(index.as_str())
                    .await
                    .map(|response| ElasticsearchResponseEvent::Index {
                        cluster_name,
                        index: index.to_owned(),
                        response,
                    })
                    .change_context(ApiHandleError::Elasticsearch)
            }
        }
    }

    fn lookup_cluster(
        &self,
        name: &str,
    ) -> error_stack::Result<&ElasticsearchClient, ApiHandleError> {
        self.clients
            .get(name)
            .ok_or_else(|| Report::new(ApiHandleError::Elasticsearch))
            .attach_printable("client not found by name: {name}")
    }
}
