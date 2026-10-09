//! `[options]` of a keymap preset: how bare letters behave, the leader key
//! and how long a key sequence waits for its next key.

use serde::Deserialize;
use std::time::Duration;

/// What a printable key without a binding does in the file panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypingMode {
    /// Far / Norton Commander: letters go to the command line.
    #[default]
    Cli,
    /// Explorer / Total Commander: letters jump to the matching file.
    TypeAhead,
    /// Vim / yazi: letters are commands; unbound ones are ignored and the
    /// command line opens only through `focus_cli`.
    Commands,
}

/// Default wait between the keys of a sequence (same as Vim's `timeoutlen`).
pub const DEFAULT_SEQUENCE_TIMEOUT_MS: u64 = 1000;

/// Resolved options of the active preset chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapOptions {
    pub typing: TypingMode,
    /// Key that `<leader>` expands to.
    pub leader: Option<String>,
    /// `0` waits for the next key indefinitely (yazi).
    pub sequence_timeout_ms: u64,
}

impl Default for KeymapOptions {
    fn default() -> Self {
        Self {
            typing: TypingMode::default(),
            leader: None,
            sequence_timeout_ms: DEFAULT_SEQUENCE_TIMEOUT_MS,
        }
    }
}

impl KeymapOptions {
    /// Applies the fields a preset file sets on top of its parent's options.
    pub fn overlay(&mut self, file: &OptionsTable) {
        if let Some(typing) = file.typing {
            self.typing = typing;
        }
        if let Some(leader) = &file.leader {
            self.leader = Some(leader.clone()).filter(|l| !l.trim().is_empty());
        }
        if let Some(ms) = file.sequence_timeout {
            self.sequence_timeout_ms = ms;
        }
    }

    /// Wait between sequence keys; "forever" when the timeout is `0`.
    pub fn sequence_timeout(&self) -> Duration {
        match self.sequence_timeout_ms {
            0 => Duration::MAX,
            ms => Duration::from_millis(ms),
        }
    }
}

/// `[options]` as written in a preset file (every field optional, so a
/// child preset only overrides what it names).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionsTable {
    pub typing: Option<TypingMode>,
    pub leader: Option<String>,
    pub sequence_timeout: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_options_override_only_what_they_name() {
        let mut opts = KeymapOptions::default();
        opts.overlay(&toml::from_str("typing = \"commands\"\nleader = \"Space\"").unwrap());
        opts.overlay(&toml::from_str("sequence_timeout = 0").unwrap());
        assert_eq!(opts.typing, TypingMode::Commands);
        assert_eq!(opts.leader.as_deref(), Some("Space"));
        assert_eq!(opts.sequence_timeout(), Duration::MAX);
    }

    #[test]
    fn unknown_option_is_rejected() {
        assert!(toml::from_str::<OptionsTable>("typo = 1").is_err());
    }
}
