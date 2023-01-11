use error_stack::{IntoReport, ResultExt};
use futures::future::OptionFuture;
use thiserror::Error;

use crate::{
    app::transport::TransportController,
    config::Config,
    event::input::{self, Command, InputHandler},
    terminal::TerminalGuard,
    view::View,
};

mod transport;
pub(crate) use transport::TransportStats;

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
        let mut transport = TransportController::init(config.clone())?;
        let mut view = View::new(config).with_transport_stats(transport.stats());

        OptionFuture::from(
            view.pre_render_loop()
                .map(|events| transport.send_requests(events)),
        )
        .await;

        loop {
            terminal
                .draw(|f| view.render(f, f.size()))
                .into_report()
                .change_context_lazy(|| AppError::TerminalIo)?;

            tokio::select! {
                biased; // tokio::select macro feature.

                command = input.read(view.state()) => match command {
                    Command::QuitApp => break,
                    Command::UnfocusComponent => view.unfocus(),
                    Command::FocusComponent(component) => view.focus(component),
                    Command::NavigateComponent(component, navigate) => {
                        OptionFuture::from(view.navigate_component(component,navigate).map(|events| transport.send_requests(events))).await;
                    }
                },

                Some(res) = transport.recv_response() => {
                    tracing::debug!(?res, "Receive api response");
                    view.update_api_response(res);
                }
            }
        }

        Ok(())
    }
}
