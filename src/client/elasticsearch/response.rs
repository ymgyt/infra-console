use std::collections::HashMap;

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

pub type CatAliases = Vec<CatAlias>;

#[derive(Debug, Clone, Deserialize)]
pub struct CatAlias {
    pub alias: String,
    pub filter: String,
    pub index: String,
    /// "true" / "false"
    pub is_write_index: String,
    #[serde(rename = "routing.index")]
    pub routing_index: String,
    #[serde(rename = "routing.search")]
    pub routing_search: String,
}

/*
{
    "index_name": {  index data ... }
 */
#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub aliases: Option<HashMap<String, IndexAlias>>,
    pub mappings: Option<IndexMappings>,
    pub settings: Option<Settings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexAlias {
    pub is_write_index: Option<bool>,
    pub filter: Option<Filter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Filter {
    term: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexMappings {
    pub dynamic: Option<String>,
    // TODO: care object(nested property)
    pub properties: HashMap<String, Property>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Property {
    r#type: Option<String>,
    format: Option<String>,
    analyzer: Option<String>,
    search_analyzer: Option<String>,
    term_vector: Option<String>,
    fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub index: Option<IndexSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexSettings {
    pub creation_date: String,
    pub number_of_shards: String,
    pub number_of_replicas: String,
    pub uuid: String,
    pub provided_name: String,
}
