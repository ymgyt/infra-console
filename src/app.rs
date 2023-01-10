use error_stack::{IntoReport, ResultExt};
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    config::Config,
    event::{
        api::{ApiHandler, RequestEvent, ResponseEvent},
        input::{self, Command, InputHandler},
    },
    terminal::TerminalGuard,
    view::View,
};

pub struct App {
    config: Config,
    terminal: TerminalGuard,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("terminal io error")]
    TerminalIo,
    #[error("configure client error")]
    ConfigureClient,
}

impl App {
    pub fn new(config: Config, terminal: TerminalGuard) -> Self {
        Self { config, terminal }
    }

    pub async fn run(self) -> error_stack::Result<(), AppError> {
        let App {
            config,
            mut terminal,
        } = self;

        terminal
            .clear()
            .into_report()
            .change_context(AppError::TerminalIo)?;

        let mut input = InputHandler::new(input::EventStream::new());
        let (req_tx, mut res_rx) = Self::init_api_handler(config.clone())?;
        let mut view = View::new(config, req_tx);

        view.pre_render_loop().await;

        loop {
            terminal
                .draw(|f| view.render(f, f.size()))
                .into_report()
                .change_context_lazy(|| AppError::TerminalIo)?;

            tokio::select! {
                biased; // tokio::select macro feature.

                command = input.read(view.state()) => match command {
                    Command::QuitApp => break,
                    Command::UnforcusComponent => view.unforcus(),
                    Command::ForcusComponent(component) => view.forcus(component),
                    Command::NavigateComponent(component, navigate) => view.navigate_component(component, navigate).await,
                },

                Some(res) = res_rx.recv() => {
                    tracing::debug!(?res, "Receive api response");
                    view.update_api_response(res);
                }
            }
        }

        Ok(())
    }

    // Span api handler task, then return channels to communicate.
    fn init_api_handler(
        config: Config,
    ) -> error_stack::Result<(Sender<RequestEvent>, Receiver<ResponseEvent>), AppError> {
        let (req_tx, req_rx) = mpsc::channel::<RequestEvent>(10);
        let (res_tx, res_rx) = mpsc::channel::<ResponseEvent>(10);

        let api_handler = ApiHandler::new(config.elasticsearch.unwrap_or_default())
            .change_context_lazy(|| AppError::ConfigureClient)?;

        tokio::spawn(api_handler.run(req_rx, res_tx));

        Ok((req_tx, res_rx))
    }
}
