//! Hands the terminal to a child process (a shell command, an external
//! program, `sudo`) and takes it back afterwards.

use super::mouse;
use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use std::io;

/// Modes released by [`suspend`]; [`Suspended::resume`] restores them.
#[must_use = "call `resume` to give the terminal back to the TUI"]
pub struct Suspended {
    mouse: bool,
}

/// Leaves raw mode and the alternate screen, and releases mouse capture so
/// the child does not receive mouse escape sequences.
pub fn suspend() -> io::Result<Suspended> {
    let mouse = mouse::is_captured();
    mouse::set_capture(false)?;
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    Ok(Suspended { mouse })
}

impl Suspended {
    /// Re-enters raw mode and the alternate screen, and captures the mouse
    /// again if it was captured before.
    pub fn resume(self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        mouse::set_capture(self.mouse)
    }
}
