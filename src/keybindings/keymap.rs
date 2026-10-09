//! A validated keymap of one context (panels, editor, viewer, lists):
//! the dispatcher, the rows behind it and the chords of each command.

use super::chord::fragility;
use super::loader::assign::Row;
use super::registry::Bindable;
use crossterm::event::{KeyEvent, KeyEventKind};
use keybinds::{KeyInput, KeySeq, Keybinds, Match};
use std::collections::HashMap;
use std::time::Duration;

pub struct ContextKeymap<B> {
    keybinds: Keybinds<B>,
    rows: Vec<Row<B>>,
    /// Command → its chords, terminal-robust ones first.
    inverse: HashMap<B, Vec<String>>,
}

impl<B> Default for ContextKeymap<B> {
    fn default() -> Self {
        Self {
            keybinds: Keybinds::default(),
            rows: Vec::new(),
            inverse: HashMap::new(),
        }
    }
}

fn is_press(key: &KeyEvent) -> bool {
    matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

impl<B: Bindable> ContextKeymap<B> {
    pub fn new(mut keybinds: Keybinds<B>, rows: Vec<Row<B>>, timeout: Duration) -> Self {
        keybinds.set_timeout(timeout);
        let mut inverse: HashMap<B, Vec<String>> = HashMap::new();
        let mut ordered: Vec<&Row<B>> = rows.iter().collect();
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
        }
    }

    /// Feeds a key press; returns the command a completed chord or
    /// sequence names.
    pub fn dispatch(&mut self, key: KeyEvent) -> Option<B> {
        if !is_press(&key) {
            return None;
        }
        self.keybinds.dispatch(key).copied()
    }

    /// True if `key` is a complete binding or starts a sequence (no state
    /// change).
    pub fn would_trigger(&self, key: KeyEvent) -> bool {
        if !is_press(&key) {
            return false;
        }
        if self.keybinds.is_ongoing() {
            return true;
        }
        let single = [KeyInput::from(&key)];
        self.keybinds
            .as_slice()
            .iter()
            .any(|b| b.seq.match_to(&single) != Match::Unmatch)
    }

    /// The command bound to exactly the single chord `key`, without feeding
    /// it to the dispatcher.
    pub fn peek_single(&self, key: KeyEvent) -> Option<B> {
        let single = [KeyInput::from(&key)];
        self.keybinds
            .as_slice()
            .iter()
            .find(|b| b.seq.match_to(&single) == Match::Matched)
            .map(|b| b.action)
    }

    pub fn is_ongoing(&self) -> bool {
        self.keybinds.is_ongoing()
    }

    pub fn reset(&mut self) {
        self.keybinds.reset();
    }

    /// Live bindings with their origin, sorted by chord.
    pub fn rows(&self) -> &[Row<B>] {
        &self.rows
    }

    /// Every chord bound to `command`, terminal-robust ones first.
    pub fn keys_for(&self, command: B) -> &[String] {
        self.inverse.get(&command).map_or(&[], Vec::as_slice)
    }

    /// The chord to show for `command`, if bound.
    pub fn key_for(&self, command: B) -> Option<&str> {
        self.keys_for(command).first().map(String::as_str)
    }

    /// The command bound to a config-style chord (`"F7"`, `"Shift+F2"`).
    pub fn resolve_key_string(&self, key: &str) -> Option<B> {
        let seq: KeySeq = key.parse().ok()?;
        self.keybinds
            .as_slice()
            .iter()
            .find(|b| b.seq == seq)
            .map(|b| b.action)
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

    /// Remaining keys + label + command of the bindings that continue the
    /// pending prefix.
    pub fn prefix_completions(&self) -> Vec<(String, String, B)> {
        let prefix = self.keybinds.ongoing_inputs();
        if prefix.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<(String, String, B)> = self
            .keybinds
            .as_slice()
            .iter()
            .filter(|b| b.seq.match_to(prefix) == Match::Prefix)
            .map(|b| {
                let rest = b.seq.as_slice()[prefix.len()..]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                (rest, b.action.label(), b.action)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        out
    }
}
