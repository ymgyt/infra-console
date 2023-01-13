use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use tui::{style::Color, text::Text};

use crate::{
    client::elasticsearch::response::{
        CatAlias, CatAliases, CatIndex, CatIndices, ClusterHealth, Index,
    },
    view::{component::elasticsearch::TableFilter, style::Styled},
};

#[derive(Debug)]
pub(super) struct Data {
    clusters: HashMap<String, ClusterData>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct ClusterData {
    health: Option<ClusterHealth>,
    indices: Option<CatIndices>,
    aliases: Option<CatAliases>,
    index: HashMap<String, Index>,
}

impl Data {
    pub(super) fn new() -> Self {
        Self {
            clusters: HashMap::new(),
        }
    }
}

impl Data {
    pub(super) fn update_cluster_health(&mut self, cluster_name: String, health: ClusterHealth) {
        self.cluster_data_mut(cluster_name).health = Some(health);
    }

    pub(super) fn get_cluster_health(&self, cluster_name: &str) -> Option<&ClusterHealth> {
        self.clusters
            .get(cluster_name)
            .and_then(|c| c.health.as_ref())
    }

    pub(super) fn update_indices(&mut self, cluster_name: String, mut indices: CatIndices) {
        indices.sort_unstable_by(|a, b| a.index.cmp(&b.index));
        self.cluster_data_mut(cluster_name).indices = Some(indices);
    }

    pub(super) fn get_visible_indices(
        &self,
        cluster_name: &str,
        filter: Option<TableFilter>,
    ) -> Option<impl Iterator<Item = &CatIndex>> {
        self.clusters
            .get(cluster_name)
            .and_then(|c| c.indices.as_ref())
            .map(|indices| {
                // need more idiomatic way.
                indices.iter().filter(move |index| match filter {
                    Some(filter) => filter.apply(index.index.as_str()),
                    None => true,
                })
            })
    }

    pub(super) fn update_aliases(&mut self, cluster_name: String, mut aliases: CatAliases) {
        aliases.sort_unstable_by(|a, b| a.alias.cmp(&b.alias));
        self.cluster_data_mut(cluster_name).aliases = Some(aliases);
    }

    pub(super) fn get_visible_aliases(
        &self,
        cluster_name: &str,
        filter: Option<TableFilter>,
    ) -> Option<impl Iterator<Item = &CatAlias>> {
        self.clusters
            .get(cluster_name)
            .and_then(|c| c.aliases.as_ref())
            .map(|aliases| {
                // need more idiomatic way.
                aliases.iter().filter(move |alias| match filter {
                    Some(filter) => filter.apply(alias.alias.as_str()),
                    None => true,
                })
            })
    }

    pub(super) fn update_index(&mut self, cluster_name: String, name: String, index: Index) {
        self.cluster_data_mut(cluster_name)
            .index
            .insert(name, index);
    }

    pub(super) fn get_index(&self, cluster_name: &str, index: &str) -> Option<&Index> {
        self.clusters
            .get(cluster_name)
            .and_then(|c| c.index.get(index))
    }

    fn cluster_data_mut(&mut self, cluster_name: String) -> &mut ClusterData {
        self.clusters
            .entry(cluster_name)
            .or_insert(ClusterData::default())
    }
}

pub(super) struct ClusterHealthFormatter<'a>(pub(super) &'a ClusterHealth, pub(super) &'a Styled);

impl<'a> From<ClusterHealthFormatter<'a>> for tui::text::Text<'a> {
    fn from(this: ClusterHealthFormatter<'a>) -> Self {
        let v = vec![
            this.1.key_value_spans("cluster_name", &this.0.cluster_name),
            this.1.key_value_spans("status", &this.0.status),
            this.1.key_value_spans("nodes", this.0.number_of_nodes),
            this.1
                .key_value_spans("data_nodes", this.0.number_of_data_nodes),
            this.1
                .key_value_spans("active_shards", this.0.active_shards),
            this.1
                .key_value_spans("active_primary_shards", this.0.active_primary_shards),
            this.1
                .key_value_spans("initializing_shards", this.0.initializing_shards),
            this.1.key_value_spans(
                "delayed_unassigned_shards",
                this.0.delayed_unassigned_shards,
            ),
            this.1
                .key_value_spans("relocating_shards", this.0.relocating_shards),
            this.1
                .key_value_spans("in_flight_fetch", this.0.number_of_in_flight_fetch),
            this.1
                .key_value_spans("pending_tasks", this.0.number_of_pending_tasks),
            this.1.key_value_spans(
                "task_max_waiting_in_queue_millis",
                this.0.task_max_waiting_in_queue_millis,
            ), // TODO: humanize duration
        ];

        Text::from(v)
    }
}

pub(super) fn health_color(health: &str) -> Color {
    match health {
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "red" => Color::Red,
        _ => Color::White,
    }
}

pub(super) fn humanize_str_bytes(s: &str) -> String {
    s.parse::<u64>()
        .map(|n| humansize::format_size(n, humansize::BINARY))
        .unwrap_or_else(|_| "unknown".to_owned())
}

pub(super) fn parse_utc(timestamp: &str) -> Option<DateTime<Utc>> {
    timestamp
        .parse()
        .ok()
        .and_then(|t| Utc.timestamp_opt(t, 0).single())
}
