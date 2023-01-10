use tui::{
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::Tabs,
};

use crate::view::{
    component::{ResourceKind, StringUtil},
    Navigate, ViewContext,
};

pub(crate) struct ResourceTab {
    state: State,
    resoureces: &'static [ResourceKind],
}

struct State {
    is_focused: bool,
    selected: usize,
}

impl State {
    fn new() -> Self {
        Self {
            is_focused: false,
            selected: 0,
        }
    }
}

impl ResourceTab {
    pub(crate) fn new() -> Self {
        Self {
            state: State::new(),
            resoureces: ResourceKind::variants(),
        }
    }

    pub(crate) fn toggle_focus(&mut self, focused: bool) {
        self.state.is_focused = focused;
    }

    pub(crate) fn navigate(&mut self, navigate: Navigate) {
        let current = self.state.selected;
        match navigate {
            Navigate::Left => {
                self.state.selected = if (current as i64) - 1 < 0 {
                    self.resoureces.len() - 1
                } else {
                    current - 1
                }
            }
            Navigate::Right => self.state.selected = (current + 1) % self.resoureces.len(),
            _ => (),
        }
    }

    pub(crate) fn selected_resource(&self) -> ResourceKind {
        self.resoureces[self.state.selected]
    }

    pub(crate) fn render<B>(&self, ctx: &mut ViewContext<B>)
    where
        B: tui::backend::Backend,
    {
        let tabs = self
            .resoureces
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let modifier = if idx == self.state.selected {
                    Modifier::BOLD | Modifier::UNDERLINED
                } else {
                    Modifier::BOLD
                };
                Spans::from(vec![Span::styled(
                    r.capitalize(),
                    Style::default().add_modifier(modifier),
                )])
            })
            .collect();

        let tab = Tabs::new(tabs)
            .block(
                ctx.style
                    .block(self.state.is_focused)
                    .title(ctx.navigatable_title("Resource")),
            )
            .highlight_style(ctx.style.highlight_style())
            .select(self.state.selected);

        ctx.frame.render_widget(tab, ctx.rect)
    }
}
