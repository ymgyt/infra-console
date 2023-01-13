use std::{
    cmp,
    fmt::{self, Display},
};

use data::Data;
use tui::{
    layout::{
        Alignment, Constraint, Direction,
        Direction::{Horizontal, Vertical},
        Layout,
    },
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState},
};
use ElasticsearchComponentKind::*;
use ElasticsearchResourceKind::*;

use crate::{
    client::elasticsearch::response::{CatAlias, CatIndex},
    event::api::{
        elasticsearch::{ElasticsearchRequestEvent, ElasticsearchResponseEvent},
        RequestEvent,
    },
    view::{
        component::{
            elasticsearch::data::{
                health_color, humanize_str_bytes, parse_utc, ClusterHealthFormatter,
            },
            StringUtil,
        },
        ApplyNavigate, Navigate, Navigated, ViewContext,
    },
    ElasticsearchConfig,
};

mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElasticsearchComponentKind {
    ClusterList,
    ResourceList,
    AliasTable,
    IndexTable,
    IndexDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElasticsearchResourceKind {
    Cluster,
    Index,
    Alias,
}

impl Display for ElasticsearchResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ElasticsearchResourceKind::Cluster => "cluster",
            ElasticsearchResourceKind::Index => "index",
            ElasticsearchResourceKind::Alias => "alias",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TableFilter {
    Internal,
}

impl TableFilter {
    pub(super) fn apply(&self, item: &str) -> bool {
        match self {
            TableFilter::Internal => !item.starts_with('.'),
        }
    }
}

pub(crate) struct ElasticsearchComponent {
    configs: Vec<ElasticsearchConfig>,
    resources: &'static [ElasticsearchResourceKind],
    state: State,
    data: Data,
}

struct State {
    focused: Option<ElasticsearchComponentKind>,
    cluster_list_state: ListState,
    resource_list_state: ListState,
    index_table_state: TableState,
    alias_table_state: TableState,
    index_table_filter: Option<TableFilter>,
    alias_table_filter: Option<TableFilter>,
    detail_index_name: Option<String>,
}

impl ElasticsearchComponent {
    pub(crate) fn new(configs: Vec<ElasticsearchConfig>) -> Self {
        static RESOURCES: &[ElasticsearchResourceKind] = &[Cluster, Index, Alias];

        let mut cluster_list_state = ListState::default();
        cluster_list_state.select(Some(0));

        let mut resource_list_state = ListState::default();
        resource_list_state.select(Some(0));

        let mut index_table_state = TableState::default();
        index_table_state.select(Some(0));

        let mut alias_table_state = TableState::default();
        alias_table_state.select(Some(0));

        Self {
            configs,
            resources: RESOURCES,
            state: State {
                focused: None,
                cluster_list_state,
                resource_list_state,
                index_table_state,
                alias_table_state,
                index_table_filter: Some(TableFilter::Internal),
                alias_table_filter: Some(TableFilter::Internal),
                detail_index_name: None,
            },
            data: Data::new(),
        }
    }

    /// Initialize component data.
    pub(crate) fn init_data(&mut self) -> Option<impl Iterator<Item = RequestEvent>> {
        self.fetch_data()
            .map(|events| events.into_iter().map(RequestEvent::Elasticsearch))
    }

    fn fetch_data(&self) -> Option<Vec<ElasticsearchRequestEvent>> {
        self.selected_cluster_name()
            .zip(self.selected_resource())
            .map(|(cluster, r)| match r {
                Cluster => vec![ElasticsearchRequestEvent::FetchCluster {
                    cluster_name: cluster.to_owned(),
                }],
                Index => match self.state.detail_index_name.as_ref() {
                    Some(task) => vec![ElasticsearchRequestEvent::FetchIndex {
                        cluster_name: cluster.to_owned(),
                        index: task.to_owned(),
                    }],
                    None => vec![ElasticsearchRequestEvent::FetchIndices {
                        cluster_name: cluster.to_owned(),
                    }],
                },
                Alias => vec![ElasticsearchRequestEvent::FetchAliases {
                    cluster_name: cluster.to_owned(),
                }],
            })
    }

    pub(crate) fn update_api_response(&mut self, res: ElasticsearchResponseEvent) {
        match res {
            ElasticsearchResponseEvent::ClusterHealth {
                cluster_name,
                response,
            } => self.data.update_cluster_health(cluster_name, response),
            ElasticsearchResponseEvent::Indices {
                cluster_name,
                response,
            } => self.data.update_indices(cluster_name, response),
            ElasticsearchResponseEvent::Aliases {
                cluster_name,
                response,
            } => self.data.update_aliases(cluster_name, response),
            ElasticsearchResponseEvent::Index {
                cluster_name,
                index,
                response,
            } => self.data.update_index(cluster_name, index, response),
        };
    }

    pub(crate) fn focus(&mut self, component: ElasticsearchComponentKind) {
        self.state.focused = Some(component);
    }

    pub(crate) fn unfocus(&mut self) {
        self.state.focused = None;
    }

    pub(crate) fn enter(
        &mut self,
        component: ElasticsearchComponentKind,
    ) -> Option<impl Iterator<Item = RequestEvent>> {
        let fetch = match component {
            IndexDetail => {
                self.selected_cluster_name()
                    .zip(self.state.index_table_state.selected())
                    .and_then(|(cluster_name, idx)| {
                        self.data
                            .get_visible_indices(cluster_name, self.state.index_table_filter)
                            .and_then(|mut iter| iter.nth(idx))
                    })
                    .into_iter()
                    .for_each(|index| self.state.detail_index_name = Some(index.index.clone()));
                true
            }
            _ => false,
        };
        if fetch {
            self.fetch_data()
                .map(|events| events.into_iter().map(RequestEvent::Elasticsearch))
        } else {
            None
        }
    }

    pub(crate) fn leave(&mut self, component: ElasticsearchComponentKind) {
        if component == ElasticsearchComponentKind::IndexDetail {
            self.state.detail_index_name = None;
        }
    }

    pub(crate) fn navigate(
        &mut self,
        component: ElasticsearchComponentKind,
        navigate: Navigate,
    ) -> Option<impl Iterator<Item = RequestEvent>> {
        let fetch = match component {
            ClusterList => {
                self.state
                    .cluster_list_state
                    .apply(navigate, self.cluster_names().count())
                    == Navigated::Happen
            }
            ResourceList => {
                self.state
                    .resource_list_state
                    .apply(navigate, self.resources.len())
                    == Navigated::Happen
            }

            IndexTable => {
                self.state.index_table_state.apply(
                    navigate,
                    self.selected_cluster_name()
                        .and_then(|c| {
                            self.data
                                .get_visible_indices(c, self.state.index_table_filter)
                        })
                        .map(|iter| iter.count())
                        .unwrap_or(0),
                );
                false
            }
            AliasTable => {
                self.state.alias_table_state.apply(
                    navigate,
                    self.selected_cluster_name()
                        .and_then(|c| {
                            self.data
                                .get_visible_aliases(c, self.state.alias_table_filter)
                                .map(|iter| iter.count())
                        })
                        .unwrap_or(0),
                );
                false
            }
            IndexDetail => unreachable!(),
        };
        if fetch {
            self.fetch_data()
                .map(|events| events.into_iter().map(RequestEvent::Elasticsearch))
        } else {
            None
        }
    }

    fn cluster_names(&self) -> impl Iterator<Item = &str> {
        self.configs.iter().map(|c| c.name.as_str())
    }

    fn selected_cluster_name(&self) -> Option<&str> {
        self.state
            .cluster_list_state
            .selected()
            .and_then(|i| self.cluster_names().nth(i))
    }

    fn selected_resource(&self) -> Option<ElasticsearchResourceKind> {
        self.state
            .resource_list_state
            .selected()
            .and_then(|i| self.resources.get(i).copied())
    }

    pub(crate) fn render<B>(&mut self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        let (left_area, resource_area) = {
            let chunks = Layout::default()
                .direction(Horizontal)
                .margin(0)
                .constraints([Constraint::Length(20), Constraint::Percentage(100)].as_ref())
                .split(ctx.rect);
            (chunks[0], chunks[1])
        };

        self.render_left(ctx.with(left_area));

        match self.selected_resource() {
            Some(Cluster) => self.render_cluster(ctx.with(resource_area)),
            Some(Index) => match self.state.detail_index_name.as_deref() {
                Some(index) => self.render_index_detail(ctx.with(resource_area), index.to_owned()),
                None => self.render_indices(ctx.with(resource_area)),
            },
            Some(Alias) => self.render_aliases(ctx.with(resource_area)),
            None => (),
        }
    }

    fn render_left<B>(&mut self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        let (cluster_list_area, resource_list_area) = {
            let chunks = Layout::default()
                .direction(Vertical)
                .constraints([
                    Constraint::Min(self.cluster_names().count() as u16 + 2),
                    Constraint::Min(self.resources.len() as u16 + 2),
                    Constraint::Percentage(100),
                ])
                .split(ctx.rect);
            (chunks[0], chunks[1])
        };

        let cluster_list: Vec<ListItem> = self
            .cluster_names()
            .enumerate()
            .map(|(idx, name)| {
                ListItem::new(Text::styled(
                    name.to_owned(),
                    Style::default().add_modifier(
                        ctx.style
                            .selected_item_modifier(idx, self.state.cluster_list_state.selected()),
                    ),
                ))
            })
            .collect();
        let cluster_list = List::new(cluster_list)
            .block(
                ctx.style
                    .block(self.state.focused == Some(ClusterList))
                    .title(ctx.navigable_title("Cluster")),
            )
            .highlight_style(ctx.style.highlight_style())
            .highlight_symbol("> ");

        let resource_list: Vec<ListItem> = self
            .resources
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                ListItem::new(Text::styled(
                    r.capitalize(),
                    Style::default().add_modifier(
                        ctx.style
                            .selected_item_modifier(idx, self.state.resource_list_state.selected()),
                    ),
                ))
            })
            .collect();

        let resource_list = List::new(resource_list)
            .block(
                ctx.style
                    .block(self.state.focused == Some(ResourceList))
                    .title(ctx.navigable_title("Elasticsearch")),
            )
            .highlight_style(ctx.style.highlight_style())
            .highlight_symbol("> ");

        ctx.frame.render_stateful_widget(
            cluster_list,
            cluster_list_area,
            &mut self.state.cluster_list_state,
        );

        ctx.frame.render_stateful_widget(
            resource_list,
            resource_list_area,
            &mut self.state.resource_list_state,
        );
    }

    fn render_cluster<B>(&mut self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        if let Some(health) = self
            .selected_cluster_name()
            .and_then(|name| self.data.get_cluster_health(name))
        {
            let cluster_health: Text = ClusterHealthFormatter(health, ctx.style).into();
            let cluster_health_area = {
                let chunks = Layout::default()
                    .direction(Vertical)
                    .constraints([
                        Constraint::Length(
                            cluster_health.height() as u16 + ctx.style.box_border_height(),
                        ),
                        Constraint::Percentage(100),
                    ])
                    .split(ctx.rect);
                chunks[0]
            };

            let cluster_health = Paragraph::new(cluster_health)
                .block(ctx.style.block(false).title("Cluster Health"))
                .alignment(Alignment::Left);

            ctx.frame.render_widget(cluster_health, cluster_health_area);
        } else {
            let not_found = Paragraph::new(Text::raw("not found"));

            ctx.frame.render_widget(not_found, ctx.rect);
        }
    }

    fn render_indices<B>(&mut self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        if let Some(indices) = self.selected_cluster_name().and_then(|name| {
            self.data
                .get_visible_indices(name, self.state.index_table_filter)
        }) {
            let indices: Vec<&CatIndex> = indices.collect();

            let num_index = indices.len();
            let max_index_width = indices
                .iter()
                .map(|i| i.index.len() + 2)
                .max()
                .unwrap_or(10);

            let selected_index = self.state.index_table_state.selected().unwrap_or(0) + 1;
            let index_header = format!("  Index({selected_index}/{num_index})");
            let (header, column_constraints): (Vec<_>, Vec<_>) = [
                (
                    index_header.as_str(),
                    Constraint::Length(max_index_width as u16),
                ),
                ("Health", Constraint::Length(6)),
                ("Status", Constraint::Length(6)),
                ("Primary", Constraint::Length(7)),
                ("Replica", Constraint::Length(7)),
                ("DocsCount", Constraint::Length(10)),
                ("DocsDeleted", Constraint::Length(12)),
                ("StoreSize", Constraint::Length(10)),
                ("PrimaryStoreSize", Constraint::Length(18)),
                ("Uuid", Constraint::Length(22)),
            ]
            .into_iter()
            .map(|(h, c)| {
                (
                    Cell::from(h)
                        .style(Style::default().add_modifier(Modifier::DIM | Modifier::BOLD)),
                    c,
                )
            })
            .unzip();

            let header = Row::new(header).height(1).bottom_margin(0);

            let rows = indices.iter().map(|index| {
                let cells = vec![
                    Span::styled(
                        "  ".to_owned() + index.index.as_str(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        index.health.as_str(),
                        Style::default().fg(health_color(index.health.as_str())),
                    ),
                    Span::styled(index.status.as_str(), Style::default()),
                    Span::styled(index.pri.as_str(), Style::default()),
                    Span::styled(index.rep.as_str(), Style::default()),
                    Span::styled(index.docs_count.as_str(), Style::default().fg(Color::Cyan)),
                    Span::styled(index.docs_deleted.as_str(), Style::default()),
                    Span::styled(
                        humanize_str_bytes(index.store_size.as_str()),
                        Style::default(),
                    ),
                    Span::styled(
                        humanize_str_bytes(index.pri_store_size.as_str()),
                        Style::default(),
                    ),
                    Span::styled(index.uuid.as_str(), Style::default()),
                ]
                .into_iter()
                .map(Cell::from);
                Row::new(cells).height(1)
            });

            let indices_area = {
                Layout::default()
                    .direction(Vertical)
                    .constraints([
                        Constraint::Length(num_index as u16 + 1 + ctx.style.box_border_height()), // header
                        Constraint::Percentage(100),
                    ])
                    .split(ctx.rect)[0]
            };

            let indices = Table::new(rows)
                .header(header)
                .block(
                    ctx.style
                        .block(self.state.focused == Some(IndexTable))
                        .title(ctx.navigable_title("Index")),
                )
                .highlight_style(ctx.style.highlight_style())
                .highlight_symbol(">")
                .widths(column_constraints.as_slice());

            ctx.frame.render_stateful_widget(
                indices,
                indices_area,
                &mut self.state.index_table_state,
            );
        } else {
            let not_found = Paragraph::new(Text::raw("not found"));

            ctx.frame.render_widget(not_found, ctx.rect);
        }
    }

    fn render_aliases<B>(&mut self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        if let Some(aliases) = self.selected_cluster_name().and_then(|name| {
            self.data
                .get_visible_aliases(name, self.state.alias_table_filter)
        }) {
            let aliases: Vec<&CatAlias> = aliases.collect();

            let num_aliases = aliases.len();
            let (_max_alias_width, _max_index_width) =
                aliases.iter().fold((0, 0), |(max_alias, max_index), a| {
                    (
                        cmp::max(max_alias, a.alias.len()),
                        cmp::max(max_index, a.index.len()),
                    )
                });

            // TODO: handle too long alias name.
            let selected_index = self.state.alias_table_state.selected().unwrap_or(0) + 1;
            let alias_header = format!("  Alias({selected_index}/{num_aliases})");
            let (header, column_constraints): (Vec<_>, Vec<_>) = [
                (alias_header.as_str(), Constraint::Percentage(30)),
                ("Index", Constraint::Percentage(30)),
                ("IsWrite", Constraint::Length(7)),
                ("Filter", Constraint::Min(10)),
                ("RoutingIndex", Constraint::Min(12)),
                ("RoutingSearch", Constraint::Min(13)),
            ]
            .into_iter()
            .map(|(h, c)| {
                (
                    Cell::from(h)
                        .style(Style::default().add_modifier(Modifier::DIM | Modifier::BOLD)),
                    c,
                )
            })
            .unzip();

            let header = Row::new(header).height(1).bottom_margin(0);

            let rows = aliases.iter().map(|alias| {
                let cells = vec![
                    Span::styled(
                        format!("  {}", alias.alias.as_str()),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(alias.index.as_str(), Style::default()),
                    Span::styled(alias.is_write_index.as_str(), Style::default()),
                    Span::styled(alias.filter.as_str(), Style::default()),
                    Span::styled(alias.routing_index.as_str(), Style::default()),
                    Span::styled(alias.routing_search.as_str(), Style::default()),
                ]
                .into_iter()
                .map(Cell::from);
                Row::new(cells).height(1)
            });

            let aliases_area = {
                Layout::default()
                    .direction(Vertical)
                    .constraints([
                        Constraint::Length(num_aliases as u16 + 1 + ctx.style.box_border_height()),
                        Constraint::Percentage(100),
                    ])
                    .split(ctx.rect)[0]
            };

            let aliases = Table::new(rows)
                .header(header)
                .block(
                    ctx.style
                        .block(self.state.focused == Some(AliasTable))
                        .title(ctx.navigable_title("Alias")),
                )
                .highlight_style(ctx.style.highlight_style())
                .highlight_symbol(">")
                .widths(column_constraints.as_slice());

            ctx.frame.render_stateful_widget(
                aliases,
                aliases_area,
                &mut self.state.alias_table_state,
            );
        } else {
            let not_found = Paragraph::new(Text::raw("not found"));

            ctx.frame.render_widget(not_found, ctx.rect);
        }
    }

    fn render_index_detail<B>(&mut self, ctx: &mut ViewContext<B>, index: String)
    where
        B: tui::backend::Backend,
    {
        let index = match self
            .selected_cluster_name()
            .and_then(|cluster| self.data.get_index(cluster, index.as_str()))
        {
            Some(index) => index,
            None => {
                let not_found = Paragraph::new(Text::raw("not found"));

                ctx.frame.render_widget(not_found, ctx.rect);
                return;
            }
        };

        let (settings_area, _mappings_area, _aliases_area) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(5 + ctx.style.box_border_height()),
                        Constraint::Percentage(40),
                        Constraint::Percentage(40),
                    ]
                    .as_ref(),
                )
                .split(ctx.rect);
            (chunks[0], chunks[1], chunks[2])
        };

        if let Some(settings) = index.settings.as_ref().and_then(|s| s.index.as_ref()) {
            let creation_date = parse_utc(settings.creation_date.as_str())
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| settings.creation_date.clone());

            let settings = Text::from(vec![
                ctx.style
                    .key_value_spans("provided_name", settings.provided_name.as_str()),
                ctx.style.key_value_spans("uuid", settings.uuid.as_str()),
                ctx.style.key_value_spans("creation_date", creation_date),
                ctx.style
                    .key_value_spans("number_of_shards", settings.number_of_shards.as_str()),
                ctx.style
                    .key_value_spans("number_of_replicas", settings.number_of_replicas.as_str()),
            ]);

            let settings = Paragraph::new(settings)
                .block(ctx.style.block(false).title("Settings"))
                .alignment(Alignment::Left);

            ctx.frame.render_widget(settings, settings_area);
        }
    }
}
