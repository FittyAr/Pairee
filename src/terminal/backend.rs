use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

/// Helper struct that handles terminal raw mode initialization and cleanup.
/// Using the Drop trait, it ensures standard terminal properties are restored
/// even if the application panics or crashes.
pub struct TerminalBackend {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalBackend {
    /// Enables raw mode, switches to alternate screen, and returns the Terminal instance.
    pub fn init() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Restores the original terminal state by disabling raw mode and leaving alternate screen.
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, Show)?;
        Ok(())
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
