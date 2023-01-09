use std::borrow::Cow;

use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders},
};

pub(crate) struct Styled {}

impl Styled {
    pub(super) fn new() -> Self {
        Self {}
    }

    pub(super) fn block(&self, focused: bool) -> Block {
        Block::default()
            .borders(Borders::ALL)
            .border_type(self.border_type())
            .border_style(Style::default().fg(self.border_color(focused)))
    }

    fn border_type(&self) -> BorderType {
        BorderType::Plain
    }

    fn border_color(&self, focused: bool) -> Color {
        if focused {
            self.highlight_color()
        } else {
            Color::White
        }
    }

    pub(super) fn box_border_height(&self) -> u16 {
        2
    }

    pub(super) fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.highlight_color())
            .add_modifier(Modifier::BOLD)
    }

    pub(super) fn highlight_color(&self) -> Color {
        Color::Yellow
    }

    pub(super) fn selected_item_modifier(&self, index: usize, selected: Option<usize>) -> Modifier {
        if Some(index) == selected {
            Modifier::BOLD | Modifier::UNDERLINED
        } else {
            Modifier::BOLD
        }
    }

    pub(super) fn key_value_spans<'a, K, V>(&self, key: K, value: V) -> Spans<'a>
    where
        K: Into<Cow<'a, str>>,
        V: ToString,
    {
        Spans::from(vec![
            Span::styled(
                format!("  {}", key.into()),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "=",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
        ])
    }
}
