use serde::Deserialize;

/// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-health.html#cluster-health-api-response-body
#[derive(Debug, Clone, Deserialize)]
pub struct ClusterHealth {
    pub active_primary_shards: i64,
    pub active_shards: i64,
    pub active_shards_percent_as_number: f64,
    pub cluster_name: String,
    pub delayed_unassigned_shards: i64,
    pub initializing_shards: i64,
    pub number_of_data_nodes: i64,
    pub number_of_in_flight_fetch: i64,
    pub number_of_nodes: i64,
    pub number_of_pending_tasks: i64,
    pub relocating_shards: i64,
    pub status: String,
    pub task_max_waiting_in_queue_millis: i64,
    pub timed_out: bool,
    pub unassigned_shards: i64,
}

/// https://www.elastic.co/guide/en/elasticsearch/reference/current/cat-indices.html
pub type CatIndices = Vec<CatIndex>;

#[derive(Debug, Clone, Deserialize)]
pub struct CatIndex {
    #[serde(rename = "docs.count")]
    pub docs_count: String,
    #[serde(rename = "docs.deleted")]
    pub docs_deleted: String,
    pub health: String,
    pub index: String,
    pub pri: String,
    #[serde(rename = "pri.store.size")]
    pub pri_store_size: String,
    pub rep: String,
    pub status: String,
    #[serde(rename = "store.size")]
    pub store_size: String,
    pub uuid: String,
}
