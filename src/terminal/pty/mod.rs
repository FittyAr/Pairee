//! Pairee's own pseudo-terminal layer: a child process whose stdin, stdout
//! and stderr are a terminal (Unix `posix_openpt` / Windows ConPTY).
//!
//! [`PtySystem`] is the seam: [`NativePty`] talks to the OS, tests can hand
//! [`super::pty_cmd`] a fake one. A spawn yields its parts separately
//! ([`SpawnedPty`]) because they live on different threads: the output is
//! read on one, the exit is waited for on another, and the UI types into,
//! resizes and stops the child from a third.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io::{self, Read, Write};
use std::path::PathBuf;

/// Terminal size the child sees, in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

impl PtySize {
    /// Size used when the real terminal's is unknown (no console, tests).
    pub const DEFAULT: Self = Self { rows: 24, cols: 80 };

    /// Size of the terminal Pairee runs in, or [`Self::DEFAULT`].
    pub fn of_terminal() -> Self {
        match crossterm::terminal::size() {
            Ok((cols, rows)) if cols > 0 && rows > 0 => Self { rows, cols },
            _ => Self::DEFAULT,
        }
    }
}

/// What to start on a new PTY. The environment is Pairee's own plus `env`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
    pub size: PtySize,
}

/// The terminal side Pairee keeps; dropping it closes the PTY.
pub trait PtyMaster: Send {
    fn resize(&self, size: PtySize) -> io::Result<()>;
}

/// Stops the child from any thread while another one waits for it.
pub trait ChildKiller: Send {
    fn kill(&mut self) -> io::Result<()>;
}

/// Blocks until the child exits; `true` when it succeeded.
pub trait ChildWaiter: Send {
    fn wait(self: Box<Self>) -> io::Result<bool>;
}

/// A child running on a fresh PTY, split by the thread that uses each part.
pub struct SpawnedPty {
    pub pid: Option<u32>,
    /// Everything the child writes (stdout and stderr share the terminal).
    pub reader: Box<dyn Read + Send>,
    /// The child's keyboard.
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn PtyMaster>,
    pub killer: Box<dyn ChildKiller>,
    pub waiter: Box<dyn ChildWaiter>,
}

/// Starts commands on pseudo terminals.
pub trait PtySystem {
    fn spawn(&self, cmd: &PtyCommand) -> io::Result<SpawnedPty>;
}

/// The operating system's PTY.
pub struct NativePty;

impl PtySystem for NativePty {
    fn spawn(&self, cmd: &PtyCommand) -> io::Result<SpawnedPty> {
        #[cfg(unix)]
        {
            unix::spawn(cmd)
        }
        #[cfg(windows)]
        {
            windows::spawn(cmd)
        }
    }
}
