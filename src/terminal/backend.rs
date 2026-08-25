use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        supports_keyboard_enhancement,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

/// Helper struct that handles terminal raw mode initialization and cleanup.
/// Using the Drop trait, it ensures standard terminal properties are restored
/// even if the application panics or crashes.
pub struct TerminalBackend {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhancement: bool,
    focus_change: bool,
    bracketed_paste: bool,
}

/// Maps the kitty-protocol probe to a push/pop decision.
/// Only `Ok(true)` enables enhancement; errors and `false` keep the legacy input path.
pub(crate) fn keyboard_enhancement_from_query(supports: io::Result<bool>) -> bool {
    matches!(supports, Ok(true))
}

fn push_keyboard_enhancement(stdout: &mut Stdout) -> bool {
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        ),
    )
    .is_ok()
}

/// Enable kitty-protocol flags only when the terminal actually supports them.
///
/// crossterm's [`supports_keyboard_enhancement`] is a real (blocking) query on
/// Unix. On Windows it is a stub that always returns `false`, including Windows
/// Terminal which does speak the protocol — so we try the CSI push there.
fn enable_keyboard_enhancement(stdout: &mut Stdout) -> bool {
    let probe_ok = if cfg!(windows) {
        true
    } else {
        keyboard_enhancement_from_query(supports_keyboard_enhancement())
    };
    probe_ok && push_keyboard_enhancement(stdout)
}

impl TerminalBackend {
    /// Enables raw mode, switches to alternate screen, and returns the Terminal instance.
    pub fn init() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide, EnableMouseCapture)?;

        let keyboard_enhancement = enable_keyboard_enhancement(&mut stdout);
        if !keyboard_enhancement {
            log::debug!("Keyboard enhancement not enabled; using legacy key events");
        }

        let focus_change = execute!(stdout, EnableFocusChange).is_ok();
        if !focus_change {
            log::debug!("EnableFocusChange not supported; continuing without focus events");
        }

        let bracketed_paste = execute!(stdout, EnableBracketedPaste).is_ok();
        if !bracketed_paste {
            log::debug!("Bracketed paste not supported; paste may arrive as key events");
        }

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            keyboard_enhancement,
            focus_change,
            bracketed_paste,
        })
    }

    /// Restores the original terminal state by disabling raw mode and leaving alternate screen.
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode()?;
        if self.keyboard_enhancement {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
            self.keyboard_enhancement = false;
        }
        if self.focus_change {
            let _ = execute!(io::stdout(), DisableFocusChange);
            self.focus_change = false;
        }
        if self.bracketed_paste {
            let _ = execute!(io::stdout(), DisableBracketedPaste);
            self.bracketed_paste = false;
        }

        execute!(
            io::stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            Show
        )?;
        Ok(())
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::keyboard_enhancement_from_query;
    use std::io;

    #[test]
    fn enhancement_enabled_only_when_query_is_ok_true() {
        assert!(keyboard_enhancement_from_query(Ok(true)));
        assert!(!keyboard_enhancement_from_query(Ok(false)));
        assert!(!keyboard_enhancement_from_query(Err(io::Error::other(
            "no tty"
        ))));
    }

    #[cfg(windows)]
    #[test]
    fn windows_crossterm_probe_is_a_stub() {
        // Guard the Windows workaround: do not skip CSI push just because the stub says false.
        assert!(!keyboard_enhancement_from_query(
            crossterm::terminal::supports_keyboard_enhancement()
        ));
    }
}
