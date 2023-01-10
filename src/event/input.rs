pub use crossterm::event::EventStream;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use Event::*;
use KeyCode::*;

use crate::view::{
    component::{
        elasticsearch::{ElasticsearchComponentKind, ElasticsearchComponentKind::ResourceList},
        ComponentKind, ResourceKind,
    },
    Navigate, ViewState,
};

pub(crate) trait InputQuery {
    fn should_quit(&self) -> bool;
    fn key_code(&self) -> Option<&KeyCode>;
    fn navigate(&self) -> Option<Navigate>;
}

impl InputQuery for Event {
    fn should_quit(&self) -> bool {
        match self {
            Key(KeyEvent {
                code: Char('q'), ..
            }) => true,
            Key(KeyEvent {
                code: Char('c'),
                modifiers,
                ..
            })
            | Key(KeyEvent {
                code: Char('d'),
                modifiers,
                ..
            }) if modifiers.contains(KeyModifiers::CONTROL) => true,
            _ => false,
        }
    }

    fn key_code(&self) -> Option<&KeyCode> {
        #[allow(clippy::single_match)]
        match self {
            Key(KeyEvent { code, .. }) => Some(code),
            _ => None,
        }
    }

    fn navigate(&self) -> Option<Navigate> {
        match self {
            Key(KeyEvent {
                code: Char('h'), ..
            })
            | Key(KeyEvent { code: Left, .. }) => Some(Navigate::Left),
            Key(KeyEvent {
                code: Char('l'), ..
            })
            | Key(KeyEvent { code: Right, .. }) => Some(Navigate::Right),
            Key(KeyEvent {
                code: Char('k'), ..
            })
            | Key(KeyEvent { code: Up, .. }) => Some(Navigate::Up),
            Key(KeyEvent {
                code: Char('j'), ..
            })
            | Key(KeyEvent { code: Down, .. }) => Some(Navigate::Down),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum Command {
    QuitApp,
    UnforcusComponent,
    ForcusComponent(ComponentKind),
    NavigateComponent(ComponentKind, Navigate),
}

pub(crate) struct InputHandler {
    event_stream: EventStream,
}

impl InputHandler {
    pub(crate) fn new(event_stream: EventStream) -> Self {
        Self { event_stream }
    }

    pub(crate) async fn read(&mut self, state: &ViewState) -> Command {
        use futures::StreamExt;

        loop {
            let input = self
                .event_stream
                .next()
                .await
                .transpose()
                .unwrap()
                .expect("Keyboard input stream closed unexpectedly");

            tracing::trace!(?input, "Read input");

            if let Key(ref event) = input {
                state.last_input_key.set(Some(*event));
            }

            if let Some(command) = self.handle(input, state) {
                tracing::debug!(?command, "Handle");

                return command;
            }
        }
    }

    fn handle(&self, input: Event, state: &ViewState) -> Option<Command> {
        use Command::*;
        use ResourceKind::*;
        if input.should_quit() {
            return Some(QuitApp);
        }

        #[allow(clippy::single_match)]
        match input.key_code() {
            Some(KeyCode::Esc) => return Some(UnforcusComponent),
            _ => (),
        }

        match state.forcused_component {
            None => match (state.selected_resource, input.key_code()) {
                (Some(Elasticsearch), Some(KeyCode::Char('c'))) => {
                    return Some(ForcusComponent(ComponentKind::Elasticsearch(
                        ElasticsearchComponentKind::ClusterList,
                    )))
                }
                (Some(Elasticsearch), Some(KeyCode::Char('e'))) => {
                    return Some(ForcusComponent(ComponentKind::Elasticsearch(ResourceList)))
                }
                (_, Some(KeyCode::Char('r'))) => {
                    return Some(ForcusComponent(ComponentKind::ResourceTab))
                }
                _ => (),
            },
            Some(component) => {
                if let Some(navigate) = input.navigate() {
                    return Some(NavigateComponent(component, navigate));
                }
            }
        }
        None
    }
}
