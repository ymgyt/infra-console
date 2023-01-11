use std::fmt::{self, Display};

use data::Data;
use tui::{
    layout::{
        Alignment, Constraint,
        Direction::{Horizontal, Vertical},
        Layout,
    },
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Cell, List, ListItem, ListState, Paragraph, Row, Table},
};
use ElasticsearchComponentKind::*;
use ElasticsearchResourceKind::*;

use crate::{
    client::elasticsearch::response::CatIndex,
    event::api::{
        elasticsearch::{ElasticsearchRequestEvent, ElasticsearchResponseEvent},
        RequestEvent,
    },
    view::{
        component::{
            elasticsearch::data::{health_color, humanize_str_bytes, ClusterHealthFormatter},
            StringUtil,
        },
        ApplyNavigate, Navigate, ViewContext,
    },
    ElasticsearchConfig,
};

mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElasticsearchComponentKind {
    ClusterList,
    ResourceList,
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
}

impl ElasticsearchComponent {
    pub(crate) fn new(configs: Vec<ElasticsearchConfig>) -> Self {
        static RESOURCES: &[ElasticsearchResourceKind] = &[Cluster, Index, Alias];

        let mut cluster_list_state = ListState::default();
        cluster_list_state.select(Some(0));

        let mut resource_list_state = ListState::default();
        resource_list_state.select(Some(0));

        Self {
            configs,
            resources: RESOURCES,
            state: State {
                focused: None,
                cluster_list_state,
                resource_list_state,
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
        if let Some(cluster) = self.selected_cluster_name() {
            match self.selected_resource() {
                Some(Cluster) => Some(vec![ElasticsearchRequestEvent::FetchCluster {
                    cluster_name: cluster.to_owned(),
                }]),
                Some(Index) => Some(vec![ElasticsearchRequestEvent::FetchIndices {
                    cluster_name: cluster.to_owned(),
                }]),
                _ => None,
            }
        } else {
            None
        }
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
        };
    }

    pub(crate) fn focus(&mut self, component: ElasticsearchComponentKind) {
        self.state.focused = Some(component);
    }

    pub(crate) fn unfocus(&mut self) {
        self.state.focused = None;
    }

    pub(crate) fn navigate(
        &mut self,
        component: ElasticsearchComponentKind,
        navigate: Navigate,
    ) -> Option<impl Iterator<Item = RequestEvent>> {
        match component {
            ClusterList => self
                .state
                .cluster_list_state
                .apply(navigate, self.cluster_names().count()),
            ResourceList => self
                .state
                .resource_list_state
                .apply(navigate, self.resources.len()),
        }
        self.fetch_data()
            .map(|events| events.into_iter().map(RequestEvent::Elasticsearch))
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
            Some(Index) => self.render_index(ctx.with(resource_area)),
            _ => (),
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
                    .title(ctx.navigatable_title("Cluster")),
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
                    .title(ctx.navigatable_title("Elasticsearch")),
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

    fn render_index<B>(&mut self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        if let Some(indices) = self
            .selected_cluster_name()
            .and_then(|name| self.data.get_indices(name))
        {
            let mut indices: Vec<&CatIndex> = indices
                .iter()
                .filter(|index| !index.index.starts_with('.'))
                .collect();
            indices.sort_unstable_by_key(|index| &index.index);

            let num_index = indices.len();
            let max_index_width = indices
                .iter()
                .map(|i| i.index.len() + 2)
                .max()
                .unwrap_or(10);

            let (header, column_constraints): (Vec<_>, Vec<_>) = [
                ("  Index", Constraint::Length(max_index_width as u16)),
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
                        Constraint::Length(num_index as u16 + ctx.style.box_border_height()),
                        Constraint::Percentage(100),
                    ])
                    .split(ctx.rect)[0]
            };

            let indices = Table::new(rows)
                .header(header)
                .block(ctx.style.block(false).title("Indices"))
                .highlight_style(ctx.style.highlight_style())
                .highlight_symbol(">")
                .widths(column_constraints.as_slice());

            // TODO: stateful
            ctx.frame.render_widget(indices, indices_area);
        } else {
            let not_found = Paragraph::new(Text::raw("not found"));

            ctx.frame.render_widget(not_found, ctx.rect);
        }
    }
}
