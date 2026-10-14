pub mod backend;
pub mod events;
pub mod mouse;
pub mod panic_hook;
pub mod pty;
pub mod pty_cmd;
pub mod pty_control;
#[cfg(unix)]
pub mod signals;
#[cfg(windows)]
pub mod signals_windows;
pub mod standalone;
pub mod suspend;
#[cfg(target_os = "linux")]
pub mod x11_poll;

pub use backend::TerminalBackend;
pub use events::{Event, EventHandler};

/// Pairee saved what it keeps and restored the terminal: on Windows, a
/// console close, logoff or shutdown held until now may end the process.
pub fn cleanup_finished() {
    #[cfg(windows)]
    signals_windows::cleanup_finished();
}
