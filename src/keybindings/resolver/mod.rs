//! Keybinding resolver backed by the industry `keybinds` crate.
//!
//! Crossterm still delivers raw `KeyEvent`s; **mapping** is owned by `keybinds`
//! (parse + dispatch + sequences). Invalid chords never enter the map.
//! The panels keymap answers through the methods below; the editor, viewer
//! and list keymaps are public fields.

use super::actions::Action;
use super::keymap::ContextKeymap;
use super::loader::assign::Row;
use super::loader::{KeymapLoadReport, KeymapSpec, LoadedKeymap, load_keymap};
use super::options::KeymapOptions;
use super::registry::def_for;
use super::screens::{EditorAction, ListAction, ViewerAction};
use crate::config::AppConfig;
use crossterm::event::KeyEvent;
use keybinds::KeyInput;
use std::time::Instant;

pub struct KeybindingResolver {
    panels: ContextKeymap<Action>,
    pub editor: ContextKeymap<EditorAction>,
    pub viewer: ContextKeymap<ViewerAction>,
    pub list: ContextKeymap<ListAction>,
    options: KeymapOptions,
    load_report: KeymapLoadReport,
    /// When the pending panels sequence received its last key.
    last_input: Option<Instant>,
}

impl KeybindingResolver {
    pub fn new(config: &AppConfig) -> Self {
        let loaded = load_keymap(&KeymapSpec::from_config(config));
        log_report(&config.keybindings.preset, &loaded.report);
        Self::from_loaded(loaded)
    }

    pub fn from_loaded(loaded: LoadedKeymap) -> Self {
        Self {
            panels: loaded.panels,
            editor: loaded.editor,
            viewer: loaded.viewer,
            list: loaded.list,
            options: loaded.options,
            load_report: loaded.report,
            last_input: None,
        }
    }

    /// Options of the active preset (typing mode, leader, timeout).
    pub fn options(&self) -> &KeymapOptions {
        &self.options
    }

    /// Live panel bindings with their origin, sorted by chord.
    pub fn rows(&self) -> &[Row<Action>] {
        self.panels.rows()
    }

    /// Validation result of the last keymap load (preset + custom overlays).
    pub fn load_report(&self) -> &KeymapLoadReport {
        &self.load_report
    }

    /// Resolve a key press into an action (may complete a multi-key sequence).
    pub fn resolve(&mut self, key_event: KeyEvent) -> Option<Action> {
        let action = self.panels.dispatch(key_event);
        self.last_input = self.panels.is_ongoing().then(Instant::now);
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
        self.panels.would_trigger(key_event)
    }

    /// The panel action bound to the single chord `key` when that action
    /// also works over the editor and viewer screens (screens list, help,
    /// palette...).
    pub fn global_action(&self, key: KeyEvent) -> Option<Action> {
        self.panels
            .peek_single(key)
            .filter(|action| def_for(*action).global)
    }

    /// The chord to show for `action` (a terminal-robust one when there is
    /// one), or `None` if unbound.
    pub fn key_for_action(&self, action: Action) -> Option<&str> {
        self.panels.key_for(action)
    }

    /// Resolve a config-style key string (e.g. `"F7"`, `"Alt+F5"`) to its action.
    pub fn resolve_for_key_string(&self, key: &str) -> Option<Action> {
        self.panels.resolve_key_string(key)
    }

    /// True while a multi-key sequence is waiting for the next chord.
    pub fn is_ongoing(&self) -> bool {
        self.panels.is_ongoing()
    }

    /// Drop an in-progress sequence (Esc / timeout UX).
    pub fn reset(&mut self) {
        self.panels.reset();
        self.last_input = None;
    }

    /// Human-readable prefix currently being matched (`"Alt+q"`).
    pub fn ongoing_prefix_display(&self) -> String {
        self.panels.ongoing_prefix_display()
    }

    /// Remaining suffix + label + action for bindings that continue the prefix.
    pub fn prefix_completions(&self) -> Vec<(String, String, Action)> {
        self.panels.prefix_completions()
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
