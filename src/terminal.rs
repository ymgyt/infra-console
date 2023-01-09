use std::{
    io,
    ops::{Deref, DerefMut},
};

use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use error_stack::{IntoReport, ResultExt};
use thiserror::Error;
use tui::backend::CrosstermBackend;

#[derive(Debug, Error)]
#[error("terminal error")]
pub struct TerminalError {}

pub type Terminal = tui::Terminal<CrosstermBackend<io::Stdout>>;

pub struct TerminalGuard {
    inner: Terminal,
}

pub fn init() -> error_stack::Result<TerminalGuard, TerminalError> {
    enable_raw_mode()
        .into_report()
        .change_context(TerminalError {})?;

    crossterm::execute!(io::stdout(), EnterAlternateScreen)
        .into_report()
        .change_context(TerminalError {})?;

    let backend = CrosstermBackend::new(io::stdout());
    let inner = Terminal::new(backend)
        .into_report()
        .change_context(TerminalError {})?;

    // configure panic hook to display panic message to user.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        reset_terminal().ok();
        original_hook(panic);
    }));

    Ok(TerminalGuard { inner })
}

impl Deref for TerminalGuard {
    type Target = Terminal;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Err(error) = reset_terminal() {
            tracing::error!(%error, "running terminal cleanup");
        }
    }
}

fn reset_terminal() -> error_stack::Result<(), TerminalError> {
    disable_raw_mode()
        .into_report()
        .change_context(TerminalError {})?;

    crossterm::execute!(io::stdout(), LeaveAlternateScreen)
        .into_report()
        .change_context(TerminalError {})
}
