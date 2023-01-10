use crossterm::event::KeyCode;
use itertools::Itertools;
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::view::{component::ResourceKind, ViewContext};

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

    pub(crate) fn render<B>(&mut self, ctx: &mut ViewContext<B>)
    where
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

        let help = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::Gray))
                    .title("Help"),
            )
            .wrap(Wrap { trim: false });

        ctx.frame.render_widget(help, ctx.rect)
    }
}
