//! Whether the terminal can report mouse events, and the mouse-capture
//! switch itself.
//!
//! Capture is process-wide terminal state, so the flag lives here rather
//! than on [`super::TerminalBackend`]: the panic hook and the code that
//! suspends the TUI to run a child process (which has no backend at hand)
//! must release it too.

use crate::config::settings::MouseMode;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use std::io::{self, IsTerminal};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Environment variable that forces detection: `0`/`off` or `1`/`on`.
pub const ENV_OVERRIDE: &str = "PAIREE_MOUSE";

static CAPTURED: AtomicBool = AtomicBool::new(false);
static CLI_DISABLED: OnceLock<bool> = OnceLock::new();

/// What detection looks at; built from the real process by [`EnvProbe::current`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvProbe {
    /// `TERM`, if set.
    pub term: Option<String>,
    /// stdin and stdout are both terminals.
    pub is_tty: bool,
    /// Running on the Windows console API (conhost or Windows Terminal).
    pub windows: bool,
    /// `PAIREE_MOUSE`, if set.
    pub env_override: Option<String>,
    /// `--no-mouse` was passed.
    pub cli_disabled: bool,
}

impl EnvProbe {
    pub fn current() -> Self {
        Self {
            term: std::env::var("TERM").ok(),
            is_tty: io::stdin().is_terminal() && io::stdout().is_terminal(),
            windows: cfg!(windows),
            env_override: std::env::var(ENV_OVERRIDE).ok(),
            cli_disabled: CLI_DISABLED.get().copied().unwrap_or(false),
        }
    }

    /// `true` when the terminal is expected to report mouse events.
    ///
    /// The Linux VT console (`TERM=linux`) only has a mouse through gpm,
    /// which crossterm does not speak, and `dumb` terminals have none.
    pub fn reports_mouse(&self) -> bool {
        if self.cli_disabled {
            return false;
        }
        if let Some(forced) = self.env_override.as_deref().and_then(parse_switch) {
            return forced;
        }
        if !self.is_tty {
            return false;
        }
        if self.windows {
            return true;
        }
        !matches!(
            self.term.as_deref(),
            Some("dumb" | "linux") | None | Some("")
        )
    }

    /// Whether `mode` captures here. `--no-mouse` turns off even `on`.
    pub fn captures(&self, mode: MouseMode) -> bool {
        !self.cli_disabled && mode.wants_capture(self.reports_mouse())
    }
}

fn parse_switch(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" => Some(true),
        "0" | "off" | "false" | "no" => Some(false),
        _ => None,
    }
}

/// Records `--no-mouse`; read by [`EnvProbe::current`].
pub fn disable_from_cli() {
    let _ = CLI_DISABLED.set(true);
}

/// `true` while the terminal is in mouse-capture mode.
pub fn is_captured() -> bool {
    CAPTURED.load(Ordering::Relaxed)
}

/// Turns capture on or off; a no-op when it is already in that state.
pub fn set_capture(on: bool) -> io::Result<()> {
    if is_captured() == on {
        return Ok(());
    }
    if on {
        execute!(io::stdout(), EnableMouseCapture)?;
    } else {
        execute!(io::stdout(), DisableMouseCapture)?;
    }
    CAPTURED.store(on, Ordering::Relaxed);
    Ok(())
}

/// Best-effort release for teardown paths that must not fail.
pub fn release() {
    let _ = set_capture(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unix(term: &str) -> EnvProbe {
        EnvProbe {
            term: Some(term.into()),
            is_tty: true,
            ..EnvProbe::default()
        }
    }

    #[test]
    fn graphical_terminals_report_the_mouse() {
        for term in [
            "xterm-256color",
            "tmux-256color",
            "screen",
            "xterm-kitty",
            "foot",
        ] {
            assert!(unix(term).reports_mouse(), "{term}");
        }
    }

    #[test]
    fn vt_console_and_dumb_terminals_do_not() {
        for term in ["linux", "dumb", ""] {
            assert!(!unix(term).reports_mouse(), "{term:?}");
        }
        let unset = EnvProbe {
            is_tty: true,
            ..EnvProbe::default()
        };
        assert!(!unset.reports_mouse());
    }

    #[test]
    fn no_terminal_means_no_mouse() {
        let piped = EnvProbe {
            is_tty: false,
            ..unix("xterm")
        };
        assert!(!piped.reports_mouse());
    }

    #[test]
    fn windows_console_always_reports() {
        let win = EnvProbe {
            windows: true,
            is_tty: true,
            ..EnvProbe::default()
        };
        assert!(win.reports_mouse());
    }

    #[test]
    fn env_override_wins_over_detection() {
        let forced_on = EnvProbe {
            env_override: Some("1".into()),
            ..unix("linux")
        };
        assert!(forced_on.reports_mouse());
        let forced_off = EnvProbe {
            env_override: Some("off".into()),
            ..unix("xterm")
        };
        assert!(!forced_off.reports_mouse());
        let garbage = EnvProbe {
            env_override: Some("maybe".into()),
            ..unix("xterm")
        };
        assert!(garbage.reports_mouse());
    }

    #[test]
    fn mode_decides_over_detection() {
        assert!(unix("xterm").captures(MouseMode::Auto));
        assert!(!unix("linux").captures(MouseMode::Auto));
        assert!(unix("linux").captures(MouseMode::On));
        assert!(!unix("xterm").captures(MouseMode::Off));
    }

    #[test]
    fn cli_flag_wins_over_everything() {
        let probe = EnvProbe {
            env_override: Some("1".into()),
            cli_disabled: true,
            ..unix("xterm")
        };
        assert!(!probe.reports_mouse());
        assert!(!probe.captures(MouseMode::On));
    }
}
