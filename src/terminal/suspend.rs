//! Hands the terminal to a child process (a shell command, an external
//! program, `sudo`) and takes it back afterwards.

use super::mouse;
use crossterm::cursor::Show;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set while a child owns the terminal.
static SUSPENDED: AtomicBool = AtomicBool::new(false);

/// `true` while a child process owns the terminal. Ctrl+C (and Ctrl+backslash /
/// Ctrl+Break) then interrupts the child, which shares the foreground
/// process group / console: the signal handlers must not end Pairee too.
pub fn child_owns_terminal() -> bool {
    SUSPENDED.load(Ordering::SeqCst)
}

/// Modes released by [`suspend`]; [`Suspended::resume`] restores them.
#[must_use = "call `resume` to give the terminal back to the TUI"]
pub struct Suspended {
    mouse: bool,
}

/// Leaves raw mode and the alternate screen, and releases mouse capture so
/// the child does not receive mouse escape sequences.
pub fn suspend() -> io::Result<Suspended> {
    SUSPENDED.store(true, Ordering::SeqCst);
    let mouse = mouse::is_captured();
    mouse::set_capture(false)?;
    // Terminal-wide modes: the child would read `ESC[200~` around pastes
    // and `ESC[I` / `ESC[O` on focus changes (best effort: some consoles
    // do not support them).
    let _ = execute!(io::stdout(), DisableBracketedPaste, DisableFocusChange);
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    Ok(Suspended { mouse })
}

impl Suspended {
    /// Re-enters raw mode and the alternate screen, and captures the mouse
    /// again if it was captured before.
    pub fn resume(self) -> io::Result<()> {
        SUSPENDED.store(false, Ordering::SeqCst);
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        let _ = execute!(io::stdout(), EnableBracketedPaste, EnableFocusChange);
        mouse::set_capture(self.mouse)
    }
}
