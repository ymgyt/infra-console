use std::sync::atomic::Ordering;

use crossterm::event::KeyCode;
use itertools::Itertools;
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    app::{TransportResult, TransportStats},
    event::api::{elasticsearch::ElasticsearchResponseEvent, ResponseEvent},
    view::{component::ResourceKind, ViewContext},
};

pub(crate) struct HelpComponent {
    common_input_keys: Vec<(KeyCode, Span<'static>)>,
    elasticsearch_input_keys: Vec<(KeyCode, Span<'static>)>,
}
impl HelpComponent {
    pub(crate) fn new() -> Self {
        Self {
            common_input_keys: Self::common_key_spans(),
            elasticsearch_input_keys: Self::elasticsearch_key_spans(),
        }
    }

    fn common_key_spans() -> Vec<(KeyCode, Span<'static>)> {
        let s = Style::default().add_modifier(Modifier::DIM);
        vec![
            (KeyCode::Char('q'), Span::styled("q: Quit", s)),
            (KeyCode::Esc, Span::styled("esc: UnforcusTab", s)),
            (KeyCode::Char('r'), Span::styled("r: Resource", s)),
            (KeyCode::Char('j'), Span::styled("j: ↓", s)),
            (KeyCode::Char('k'), Span::styled("k: ↑", s)),
            (KeyCode::Char('h'), Span::styled("h: ←", s)),
            (KeyCode::Char('l'), Span::styled("l: →", s)),
        ]
    }

    fn elasticsearch_key_spans() -> Vec<(KeyCode, Span<'static>)> {
        let s = Style::default().add_modifier(Modifier::DIM);
        vec![
            (KeyCode::Char('c'), Span::styled("c: Cluster", s)),
            (KeyCode::Char('e'), Span::styled("e: Elasticsearch", s)),
            (KeyCode::Char('i'), Span::styled("i: Index", s)),
            (KeyCode::Char('a'), Span::styled("a: Alias", s)),
        ]
    }

    /// Highlight key help according to input entered.
    fn highlight_key_spans<'a>(
        &self,
        iter: impl Iterator<Item = &'a (KeyCode, Span<'a>)>,
        last_input_key_code: Option<KeyCode>,
    ) -> Spans<'a> {
        #[allow(unstable_name_collisions)] // Itertools::intersperse collide with std
        let spans: Vec<Span<'_>> = iter
            .map(|(key, span)| {
                if Some(*key) == last_input_key_code {
                    Span::styled(
                        span.content.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    )
                } else {
                    span.clone()
                }
            })
            .intersperse(Span::raw("  "))
            .collect();

        Spans::from(spans)
    }

    fn format_transport_stats(&self, stats: &TransportStats) -> Spans {
        let in_flight = stats.in_flight_requests.load(Ordering::Relaxed);

        let mut s = Spans::from(vec![
            Span::styled(
                "in flight req: ",
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::styled(
                format!("{in_flight}"),
                Style::default().add_modifier(if in_flight > 0 {
                    Modifier::BOLD
                } else {
                    Modifier::DIM
                }),
            ),
            Span::raw("  "),
        ]);

        if let Some(t) = stats.latest_transport() {
            s.0.extend(format_transport(t).0.into_iter());
        }
        s
    }

    pub(crate) fn render<B>(
        &mut self,
        ctx: &mut ViewContext<B>,
        transport_stats: Option<&TransportStats>,
    ) where
        B: tui::backend::Backend,
    {
        let last_input_key_code = ctx.state.last_input_key.get().map(|event| event.code);

        let mut lines = Vec::new();

        lines.push(self.highlight_key_spans(self.common_input_keys.iter(), last_input_key_code));

        #[allow(clippy::single_match)]
        match ctx.state.selected_resource {
            Some(ResourceKind::Elasticsearch) => {
                lines.push(self.highlight_key_spans(
                    self.elasticsearch_input_keys.iter(),
                    last_input_key_code,
                ));
            }
            _ => (),
        }

        if let Some(stats) = transport_stats {
            lines.push(self.format_transport_stats(stats));
        }

        let help = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: false });

        ctx.frame.render_widget(help, ctx.rect)
    }
}

fn format_transport(t: TransportResult) -> Spans<'static> {
    // need more improvement.
    let elapsed = t.elapsed();
    match t.response {
        Ok(event) => {
            let ok = Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::DIM);
            let style = Style::default().add_modifier(Modifier::DIM);
            let mut spans = Spans::from(vec![Span::styled("OK", ok), Span::raw(" ")]);
            let s = match event {
                ResponseEvent::Elasticsearch(e) => match e {
                    ElasticsearchResponseEvent::ClusterHealth { cluster_name, .. } => Span::styled(
                        format!("elasticsearch {cluster_name} /_cluster/health"),
                        style,
                    ),
                    ElasticsearchResponseEvent::Indices { cluster_name, .. } => {
                        Span::styled(format!("elasticsearch {cluster_name} /_cat/indices"), style)
                    }
                    ElasticsearchResponseEvent::Aliases { cluster_name, .. } => {
                        Span::styled(format!("elasticsearch {cluster_name} /_cat/aliases"), style)
                    }
                },
            };
            spans.0.push(s);
            spans
                .0
                .push(Span::styled(format!(" {}ms", elapsed.as_millis()), style));
            spans
        }
        Err(err) => {
            let err_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
            Spans::from(vec![
                Span::styled("ERROR", err_style),
                Span::raw("  "),
                Span::styled(
                    format!("{err}"),
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ])
        }
    }
}
