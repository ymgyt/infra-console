use std::cell::Cell;

use ascii::AsAsciiStr;
use component::resource_tab::ResourceTab;
use crossterm::event::KeyEvent;
use tokio::sync::mpsc::Sender;
use tui::{
    layout::{Constraint, Direction::Vertical, Layout, Rect},
    text::Spans,
    Frame,
};

use crate::{
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
}

pub(crate) struct ViewState {
    pub(crate) forcused_component: Option<ComponentKind>,
    pub(crate) selected_resource: Option<ResourceKind>,
    pub(crate) last_input_key: Cell<Option<KeyEvent>>,
}

impl ViewState {
    fn new() -> Self {
        Self {
            forcused_component: None,
            selected_resource: Some(ResourceKind::variants()[0]), // should query
            last_input_key: Cell::new(None),
        }
    }
}

impl View {
    pub(crate) fn new(config: Config, tx: Sender<RequestEvent>) -> Self {
        Self {
            resource_tab: ResourceTab::new(),
            elasticsearch: ElasticsearchComponent::new(
                config.elasticsearch.unwrap_or_default(),
                tx,
            ),
            help: HelpComponent::new(),
            state: ViewState::new(),
            style: Styled::new(),
        }
    }

    /// Init view before into render loop.
    pub(crate) async fn pre_render_loop(&mut self) {
        #[allow(clippy::single_match)]
        match self.resource_tab.selected_resource() {
            ResourceKind::Elasticsearch => self.elasticsearch.init_data().await,
            _ => (),
        }
    }

    pub(crate) fn state(&self) -> &ViewState {
        &self.state
    }

    pub(crate) fn unforcus(&mut self) {
        if let Some(focused) = self.state.forcused_component {
            match focused {
                ComponentKind::ResourceTab => self.resource_tab.toggle_focus(false),
                ComponentKind::Elasticsearch(_) => self.elasticsearch.unforcus(),
            }
        }
        self.state.forcused_component = None;
    }

    pub(crate) fn forcus(&mut self, component: ComponentKind) {
        // disable current focus.
        self.unforcus();

        match component {
            ComponentKind::ResourceTab => self.resource_tab.toggle_focus(true),
            ComponentKind::Elasticsearch(component) => self.elasticsearch.focus(component),
        }

        self.state.forcused_component = Some(component);
    }

    pub(crate) async fn navigate_component(
        &mut self,
        component: ComponentKind,
        navigate: Navigate,
    ) {
        match component {
            ComponentKind::ResourceTab => {
                self.resource_tab.navigate(navigate);
                self.state.selected_resource = Some(self.resource_tab.selected_resource());
            }
            ComponentKind::Elasticsearch(component) => {
                self.elasticsearch.navigate(component, navigate).await;
            }
        }
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
                        Constraint::Percentage(85),
                        Constraint::Min(2 + self.style.box_border_height()),
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

        self.help.render(ctx.with(help_area))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Navigate {
    Left,
    Right,
    Up,
    Down,
}

trait ApplyNavigate {
    fn apply(&mut self, navigate: Navigate, len: usize);
}

impl ApplyNavigate for tui::widgets::ListState {
    fn apply(&mut self, navigate: Navigate, len: usize) {
        match navigate {
            Navigate::Up => {
                let i = match self.selected() {
                    Some(n) => {
                        if n == 0 {
                            len - 1
                        } else {
                            n - 1
                        }
                    }
                    None => len - 1,
                };
                self.select(Some(i));
            }
            Navigate::Down => {
                let i = match self.selected() {
                    Some(n) => (n + 1) % len,
                    None => 0,
                };
                self.select(Some(i));
            }
            _ => (),
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

    fn navigatable_title<'a>(&self, title: &'a str) -> Spans<'a> {
        if self.state.forcused_component.is_some() {
            Spans::from(title)
        } else {
            match title.as_ascii_str().ok().and_then(|s| s.get_ascii(0)) {
                Some(first) => Spans::from(format!("{title}({})", first.to_ascii_lowercase())),
                None => Spans::from(title),
            }
        }
    }
}
