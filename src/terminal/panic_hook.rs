//! Panic hook that restores the terminal before the panic message is printed.
//!
//! Without it, a panic inside the TUI leaves the user's shell in raw mode on
//! the alternate screen with mouse capture on, and the panic message is drawn
//! into the (soon discarded) alternate buffer where nobody can read it.
//! `TerminalBackend`'s `Drop` also restores the terminal, but only after the
//! default hook has already printed — and not at all with `panic = "abort"`.

use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, DisableFocusChange},
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};
use std::io;

/// Best-effort terminal restoration; every step ignores errors so a broken
/// stdout cannot cause a second panic inside the hook.
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    super::mouse::release();
    let _ = execute!(
        io::stdout(),
        DisableBracketedPaste,
        DisableFocusChange,
        LeaveAlternateScreen,
        Show
    );
}

/// Installs the hook on the UI thread (the caller). A panic there restores
/// the terminal, is logged, then reaches the previously installed hook
/// (normally the default one that prints to stderr). A panic on any other
/// thread (a background job, which is caught and reported, or a worker
/// whose lock `lock_safe` recovers) is only logged: the TUI keeps running,
/// so restoring the terminal or printing over it would break it.
pub fn install() {
    let ui_thread = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let on_ui = std::thread::current().id() == ui_thread;
        if on_ui {
            restore_terminal();
        }
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!("Pairee panicked: {info}\n{backtrace}");
        if on_ui {
            previous(info);
        }
    }));
}
