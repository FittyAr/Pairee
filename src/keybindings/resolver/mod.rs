//! Keybinding resolver backed by the industry `keybinds` crate.
//!
//! Crossterm still delivers raw `KeyEvent`s; **mapping** is owned by `keybinds`
//! (parse + dispatch + sequences). Invalid chords never enter the map.

use super::actions::Action;
use super::loader::{KeymapLoadReport, load_keybinds};
use crate::config::AppConfig;
use crossterm::event::{KeyEvent, KeyEventKind};
use keybinds::{KeyInput, KeySeq, Keybinds, Match};
use std::collections::HashMap;

pub struct KeybindingResolver {
    keybinds: Keybinds<Action>,
    /// Action → first bound chord display (for F-key bar / help).
    inverse: HashMap<Action, String>,
    load_report: KeymapLoadReport,
}

impl KeybindingResolver {
    pub fn new(config: &AppConfig) -> Self {
        let (keybinds, report) = load_keybinds(
            &config.keybindings.preset,
            &config.keybindings.custom_bindings,
        );

        for w in &report.warnings {
            log::warn!("keymap: {w}");
        }
        for e in &report.errors {
            log::error!("keymap: {e}");
        }
        if !report.ok() {
            log::error!(
                "keymap loaded with {} error(s); {} binding(s) active",
                report.errors.len(),
                report.bound_count
            );
        } else {
            log::info!(
                "keymap preset='{}' loaded ({} bindings)",
                config.keybindings.preset,
                report.bound_count
            );
        }

        let mut inverse: HashMap<Action, String> = HashMap::new();
        for bind in keybinds.as_slice() {
            inverse
                .entry(bind.action)
                .or_insert_with(|| bind.seq.to_string());
        }

        Self {
            keybinds,
            inverse,
            load_report: report,
        }
    }

    /// Validation result of the last keymap load (preset + custom overlays).
    pub fn load_report(&self) -> &KeymapLoadReport {
        &self.load_report
    }

    /// Resolve a key press into an action (may complete a multi-key sequence).
    pub fn resolve(&mut self, key_event: KeyEvent) -> Option<Action> {
        // Ignore key-release / non-press noise from enhancement flags.
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return None;
        }
        self.keybinds.dispatch(key_event).copied()
    }

    /// True if this key is a complete binding or starts a multi-key sequence.
    /// Used to keep CLI capture from eating shortcuts (immutable, no dispatch).
    pub fn would_trigger(&self, key_event: KeyEvent) -> bool {
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return false;
        }
        if self.keybinds.is_ongoing() {
            return true;
        }
        let input = KeyInput::from(&key_event);
        let single = [input];
        for bind in self.keybinds.as_slice() {
            match bind.seq.match_to(&single) {
                Match::Matched | Match::Prefix => return true,
                Match::Unmatch => {}
            }
        }
        false
    }

    /// Returns the key string bound to `action`, or `None` if unbound.
    pub fn key_for_action(&self, action: Action) -> Option<&str> {
        self.inverse.get(&action).map(|s| s.as_str())
    }

    /// Resolve a config-style key string (e.g. `"F7"`, `"Alt+F5"`) to its action.
    pub fn resolve_for_key_string(&self, key: &str) -> Option<Action> {
        let seq: KeySeq = key.parse().ok()?;
        self.keybinds
            .as_slice()
            .iter()
            .find(|b| b.seq == seq)
            .map(|b| b.action)
    }

    /// Live keymap rows: display chord + action (one row per bound sequence).
    pub fn bindings(&self) -> impl Iterator<Item = (String, Action)> + '_ {
        self.keybinds
            .as_slice()
            .iter()
            .map(|b| (b.seq.to_string(), b.action))
    }

    /// True while a multi-key sequence is waiting for the next chord.
    pub fn is_ongoing(&self) -> bool {
        self.keybinds.is_ongoing()
    }

    /// Drop an in-progress sequence (Esc / timeout UX).
    pub fn reset(&mut self) {
        self.keybinds.reset();
    }

    /// Human-readable prefix currently being matched (`"Alt+q"`).
    pub fn ongoing_prefix_display(&self) -> String {
        self.keybinds
            .ongoing_inputs()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Remaining suffix + label + action for bindings that continue the prefix.
    pub fn prefix_completions(&self) -> Vec<(String, String, Action)> {
        let prefix = self.keybinds.ongoing_inputs();
        if prefix.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for bind in self.keybinds.as_slice() {
            if bind.seq.match_to(prefix) != Match::Prefix {
                continue;
            }
            let rest = bind.seq.as_slice()[prefix.len()..]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            out.push((rest, super::registry::label_for(bind.action), bind.action));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        out
    }
}

/// Human-readable key for plugins / logging (best-effort; not the source of truth).
pub fn key_event_to_string(key: KeyEvent) -> String {
    let input = KeyInput::from(&key);
    input.to_string()
}

#[cfg(test)]
mod tests;
