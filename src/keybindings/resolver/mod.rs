//! Keybinding resolver backed by the industry `keybinds` crate.
//!
//! Crossterm still delivers raw `KeyEvent`s; **mapping** is owned by `keybinds`
//! (parse + dispatch + sequences). Invalid chords never enter the map.

use super::actions::Action;
use super::chord::fragility;
use super::loader::assign::Row;
use super::loader::{KeymapLoadReport, KeymapSpec, LoadedKeymap, load_keymap};
use super::options::KeymapOptions;
use crate::config::AppConfig;
use crossterm::event::{KeyEvent, KeyEventKind};
use keybinds::{KeyInput, KeySeq, Keybinds, Match};
use std::collections::HashMap;
use std::time::Instant;

pub struct KeybindingResolver {
    keybinds: Keybinds<Action>,
    rows: Vec<Row<Action>>,
    /// Action → its chords, terminal-robust ones first.
    inverse: HashMap<Action, Vec<String>>,
    options: KeymapOptions,
    load_report: KeymapLoadReport,
    /// When the pending sequence received its last key.
    last_input: Option<Instant>,
}

impl KeybindingResolver {
    pub fn new(config: &AppConfig) -> Self {
        let loaded = load_keymap(&KeymapSpec::from_config(config));
        log_report(&config.keybindings.preset, &loaded.report);
        Self::from_loaded(loaded)
    }

    pub fn from_loaded(loaded: LoadedKeymap) -> Self {
        let LoadedKeymap {
            mut keybinds,
            rows,
            options,
            report,
        } = loaded;
        keybinds.set_timeout(options.sequence_timeout());
        let mut inverse: HashMap<Action, Vec<String>> = HashMap::new();
        let mut ordered: Vec<&Row<Action>> = rows.iter().collect();
        ordered.sort_by_key(|r| fragility(&r.seq).is_some());
        for row in ordered {
            inverse
                .entry(row.command)
                .or_default()
                .push(row.seq.to_string());
        }
        Self {
            keybinds,
            rows,
            inverse,
            options,
            load_report: report,
            last_input: None,
        }
    }

    /// Options of the active preset (typing mode, leader, timeout).
    pub fn options(&self) -> &KeymapOptions {
        &self.options
    }

    /// Live bindings with their origin, sorted by chord.
    pub fn rows(&self) -> &[Row<Action>] {
        &self.rows
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
        let action = self.keybinds.dispatch(key_event).copied();
        self.last_input = self.keybinds.is_ongoing().then(Instant::now);
        action
    }

    /// Drops a pending sequence whose next key did not arrive in time, so
    /// the prefix HUD closes without another key press. `true` if dropped.
    pub fn expire_pending(&mut self, now: Instant) -> bool {
        let stale = self
            .last_input
            .is_some_and(|t| now.saturating_duration_since(t) > self.options.sequence_timeout());
        if stale {
            self.reset();
        }
        stale
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

    /// The chord to show for `action` (a terminal-robust one when there is
    /// one), or `None` if unbound.
    pub fn key_for_action(&self, action: Action) -> Option<&str> {
        self.keys_for_action(action).first().map(String::as_str)
    }

    /// Every chord bound to `action`, terminal-robust ones first.
    pub fn keys_for_action(&self, action: Action) -> &[String] {
        self.inverse.get(&action).map_or(&[], Vec::as_slice)
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

    /// True while a multi-key sequence is waiting for the next chord.
    pub fn is_ongoing(&self) -> bool {
        self.keybinds.is_ongoing()
    }

    /// Drop an in-progress sequence (Esc / timeout UX).
    pub fn reset(&mut self) {
        self.keybinds.reset();
        self.last_input = None;
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

fn log_report(preset: &str, report: &KeymapLoadReport) {
    for w in report.warnings.iter().chain(&report.robustness) {
        log::warn!("keymap: {w}");
    }
    for e in &report.errors {
        log::error!("keymap: {e}");
    }
    if report.ok() {
        log::info!(
            "keymap preset='{preset}' loaded ({} bindings)",
            report.bound_count
        );
    } else {
        log::error!(
            "keymap loaded with {} error(s); {} binding(s) active",
            report.errors.len(),
            report.bound_count
        );
    }
}

/// Human-readable key for plugins / logging (best-effort; not the source of truth).
pub fn key_event_to_string(key: KeyEvent) -> String {
    let input = KeyInput::from(&key);
    input.to_string()
}

#[cfg(test)]
mod tests;
