use std::{cell::Cell, sync::Arc};

use ascii::AsAsciiStr;
use component::resource_tab::ResourceTab;
use crossterm::event::KeyEvent;
use tui::{
    layout::{Constraint, Direction::Vertical, Layout, Rect},
    text::Spans,
    Frame,
};

use crate::{
    app::TransportStats,
    event::api::{RequestEvent, ResponseEvent},
    view::{
        component::{
            elasticsearch::ElasticsearchComponent, help::HelpComponent, ComponentKind, ResourceKind,
        },
        style::Styled,
    },
    Config,
};

pub(crate) mod component;
pub(super) mod style;

pub(crate) struct View {
    resource_tab: ResourceTab,
    elasticsearch: ElasticsearchComponent,
    help: HelpComponent,
    state: ViewState,
    style: Styled,
    transport_stats: Option<Arc<TransportStats>>,
}

pub(crate) struct ViewState {
    pub(crate) focused_component: Option<ComponentKind>,
    pub(crate) entered_component: Option<ComponentKind>,
    pub(crate) selected_resource: Option<ResourceKind>,
    pub(crate) last_input_key: Cell<Option<KeyEvent>>,
}

impl ViewState {
    fn new() -> Self {
        Self {
            focused_component: None,
            entered_component: None,
            selected_resource: Some(ResourceKind::variants()[0]), // should query
            last_input_key: Cell::new(None),
        }
    }
}

impl View {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            resource_tab: ResourceTab::new(),
            elasticsearch: ElasticsearchComponent::new(config.elasticsearch.unwrap_or_default()),
            help: HelpComponent::new(),
            state: ViewState::new(),
            style: Styled::new(),
            transport_stats: None,
        }
    }

    pub(crate) fn with_transport_stats(mut self, stats: Arc<TransportStats>) -> Self {
        self.transport_stats = Some(stats);
        self
    }

    /// Init view before into render loop.
    pub(crate) fn pre_render_loop(&mut self) -> Option<impl Iterator<Item = RequestEvent>> {
        #[allow(clippy::single_match)]
        match self.resource_tab.selected_resource() {
            ResourceKind::Elasticsearch => self.elasticsearch.init_data(),
            _ => None,
        }
    }

    pub(crate) fn state(&self) -> &ViewState {
        &self.state
    }

    pub(crate) fn unfocus(&mut self) {
        if let Some(focused) = self.state.focused_component {
            match focused {
                ComponentKind::ResourceTab => self.resource_tab.toggle_focus(false),
                ComponentKind::Elasticsearch(_) => self.elasticsearch.unfocus(),
            }
        }
        self.state.focused_component = None;
    }

    pub(crate) fn focus(&mut self, component: ComponentKind) {
        // disable current focus.
        self.unfocus();

        match component {
            ComponentKind::ResourceTab => self.resource_tab.toggle_focus(true),
            ComponentKind::Elasticsearch(component) => self.elasticsearch.focus(component),
        }

        self.state.focused_component = Some(component);
    }

    pub(crate) fn navigate_component(
        &mut self,
        component: ComponentKind,
        navigate: Navigate,
    ) -> Option<impl Iterator<Item = RequestEvent>> {
        match component {
            ComponentKind::ResourceTab => {
                self.resource_tab.navigate(navigate);
                self.state.selected_resource = Some(self.resource_tab.selected_resource());
                None
            }
            ComponentKind::Elasticsearch(component) => {
                self.elasticsearch.navigate(component, navigate)
            }
        }
    }

    pub(crate) fn enter_component(
        &mut self,
        component: ComponentKind,
    ) -> Option<impl Iterator<Item = RequestEvent>> {
        self.state.entered_component = Some(component);
        match component {
            ComponentKind::ResourceTab => unreachable!(),
            ComponentKind::Elasticsearch(component) => self.elasticsearch.enter(component),
        }
    }

    pub(crate) fn leave_component(&mut self, component: ComponentKind) {
        match component {
            ComponentKind::ResourceTab => unreachable!(),
            ComponentKind::Elasticsearch(component) => self.elasticsearch.leave(component),
        }
        self.state.entered_component = None;
    }

    pub(crate) fn update_api_response(&mut self, res: ResponseEvent) {
        match res {
            ResponseEvent::Elasticsearch(res) => self.elasticsearch.update_api_response(res),
        }
    }

    pub(crate) fn render<B>(&mut self, frame: &mut Frame<B>, rect: Rect)
    where
        B: tui::backend::Backend,
    {
        let (resource_tab_area, resource_area, help_area) = {
            let chunks = Layout::default()
                .direction(Vertical)
                .margin(0)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Percentage(88),
                        Constraint::Max(3 + self.style.box_border_height()),
                    ]
                    .as_ref(),
                )
                .split(rect);
            (chunks[0], chunks[1], chunks[2])
        };

        let mut ctx = ViewContext::new(frame, resource_tab_area, &self.style, &self.state);

        self.resource_tab.render(&mut ctx);

        #[allow(clippy::single_match)]
        match self.resource_tab.selected_resource() {
            ResourceKind::Elasticsearch => self.elasticsearch.render(ctx.with(resource_area)),
            _ => (),
        }

        self.help
            .render(ctx.with(help_area), self.transport_stats.as_deref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Navigate {
    Left,
    Right,
    Up,
    Down,
}

impl Navigate {
    fn inc(current: usize, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            (current + 1) % len
        }
    }
    fn inc_opt(current: Option<usize>, len: usize) -> usize {
        match current {
            Some(current) => Navigate::inc(current, len),
            None => 0,
        }
    }
    fn dec(current: usize, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        if current == 0 {
            len - 1
        } else {
            current - 1
        }
    }
    fn dec_opt(current: Option<usize>, len: usize) -> usize {
        match current {
            Some(current) => Navigate::dec(current, len),
            None => {
                if len == 0 {
                    0
                } else {
                    len - 1
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Navigated {
    Happen,
    DoesNotHappen,
}

trait ApplyNavigate {
    fn apply(&mut self, navigate: Navigate, len: usize) -> Navigated;
}

impl ApplyNavigate for tui::widgets::ListState {
    fn apply(&mut self, navigate: Navigate, len: usize) -> Navigated {
        match navigate {
            Navigate::Up => {
                self.select(Some(Navigate::dec_opt(self.selected(), len)));
                Navigated::Happen
            }
            Navigate::Down => {
                self.select(Some(Navigate::inc_opt(self.selected(), len)));
                Navigated::Happen
            }
            _ => Navigated::DoesNotHappen,
        }
    }
}

impl ApplyNavigate for tui::widgets::TableState {
    fn apply(&mut self, navigate: Navigate, len: usize) -> Navigated {
        match navigate {
            Navigate::Up => {
                self.select(Some(Navigate::dec_opt(self.selected(), len)));
                Navigated::Happen
            }
            Navigate::Down => {
                self.select(Some(Navigate::inc_opt(self.selected(), len)));
                Navigated::DoesNotHappen
            }
            _ => Navigated::DoesNotHappen,
        }
    }
}

pub(crate) struct ViewContext<'f, 'b, 's, B>
where
    B: tui::backend::Backend,
{
    frame: &'f mut Frame<'b, B>,
    rect: Rect,
    style: &'s Styled,
    state: &'s ViewState,
}

impl<'f, 'b, 's, B> ViewContext<'f, 'b, 's, B>
where
    B: tui::backend::Backend,
{
    fn new(
        frame: &'f mut Frame<'b, B>,
        rect: Rect,
        style: &'s Styled,
        state: &'s ViewState,
    ) -> Self {
        Self {
            frame,
            rect,
            style,
            state,
        }
    }

    fn with(&mut self, rect: Rect) -> &mut Self {
        self.rect = rect;
        self
    }

    fn navigable_title<'a>(&self, title: &'a str) -> Spans<'a> {
        if self.state.focused_component.is_some() {
            Spans::from(title)
        } else {
            match title.as_ascii_str().ok().and_then(|s| s.get_ascii(0)) {
                Some(first) => Spans::from(format!("{title}({})", first.to_ascii_lowercase())),
                None => Spans::from(title),
            }
        }
    }
}
