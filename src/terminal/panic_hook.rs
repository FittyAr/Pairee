//! Panic hook that restores the terminal before the panic message is printed.
//!
//! Without it, a panic inside the TUI leaves the user's shell in raw mode on
//! the alternate screen with mouse capture on, and the panic message is drawn
//! into the (soon discarded) alternate buffer where nobody can read it.
//! `TerminalBackend`'s `Drop` also restores the terminal, but only after the
//! default hook has already printed — and not at all with `panic = "abort"`.

use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, DisableFocusChange, DisableMouseCapture},
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};
use std::io;

/// Best-effort terminal restoration; every step ignores errors so a broken
/// stdout cannot cause a second panic inside the hook.
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        DisableMouseCapture,
        DisableBracketedPaste,
        DisableFocusChange,
        LeaveAlternateScreen,
        Show
    );
}

/// Installs the hook: restore the terminal, log the panic, then run the
/// previously installed hook (normally the default one that prints to stderr).
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!("Pairee panicked: {info}\n{backtrace}");
        previous(info);
    }));
}
