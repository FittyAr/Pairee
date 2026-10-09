//! Applying keymap layers: the later layer wins.
//!
//! Naming an action in a layer **replaces** its chords; a chord that another
//! action owned moves to the new one and the move is recorded in
//! [`KeymapLoadReport::displaced`]. Two actions claiming one chord inside the
//! same layer is an error. `action = ""` unbinds the action.

use super::layers::{Layer, Origin};
use super::report::KeymapLoadReport;
use crate::keybindings::chord::{normalize_chord, seqs_overlap};
use crate::keybindings::preset::parse_id;
use crate::keybindings::registry::Bindable;
use keybinds::{KeySeq, Keybind, Keybinds};
use std::collections::{HashMap, HashSet};

/// One live binding.
#[derive(Debug, Clone)]
pub struct Row<B> {
    pub seq: KeySeq,
    pub command: B,
    pub origin: Origin,
}

/// The bindings built so far, in application order.
#[derive(Debug)]
pub struct Assignments<B> {
    rows: Vec<Row<B>>,
    /// Commands some layer named (bound or unbound).
    mentioned: HashSet<B>,
}

impl<B> Default for Assignments<B> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            mentioned: HashSet::new(),
        }
    }
}

/// A layer entry after id parsing: the command, its id and raw chords
/// (several TOML keys may name one command through legacy aliases).
struct Entry<B> {
    command: B,
    id: String,
    chords: Vec<String>,
}

impl<B: Bindable> Assignments<B> {
    pub fn rows(&self) -> &[Row<B>] {
        &self.rows
    }

    /// Applies `layer` over the current bindings.
    pub fn apply(&mut self, layer: &Layer, leader: Option<&str>, report: &mut KeymapLoadReport) {
        let mut claimed: HashMap<KeySeq, B> = HashMap::new();
        for entry in entries::<B>(layer, report) {
            self.mentioned.insert(entry.command);
            self.rows.retain(|r| r.command != entry.command);
            for raw in &entry.chords {
                let Some(seq) = parse_seq(raw, leader, &entry.id, report) else {
                    continue;
                };
                match claimed.get(&seq) {
                    Some(owner) if *owner != entry.command => {
                        report.errors.push(format!(
                            "Duplicate key chord '{seq}' in {}: already bound to '{}', cannot also bind '{}'",
                            layer.origin,
                            owner.id(),
                            entry.id
                        ));
                        continue;
                    }
                    Some(_) => continue,
                    None => {}
                }
                if let Some(pos) = self.rows.iter().position(|r| r.seq == seq) {
                    let prev = self.rows.remove(pos);
                    report.displaced.push(format!(
                        "'{seq}': {} → {} ({})",
                        prev.command.id(),
                        entry.id,
                        layer.origin
                    ));
                }
                claimed.insert(seq.clone(), entry.command);
                self.rows.push(Row {
                    seq,
                    command: entry.command,
                    origin: layer.origin.clone(),
                });
            }
        }
    }

    /// Applies suggested bindings (a plugin's keys) that never take a chord:
    /// a chord already bound, or the start or extension of a bound sequence,
    /// is left unassigned and recorded in [`KeymapLoadReport::conflicts`].
    pub fn suggest(&mut self, layer: &Layer, leader: Option<&str>, report: &mut KeymapLoadReport) {
        for entry in entries::<B>(layer, report) {
            for raw in &entry.chords {
                let Some(seq) = parse_seq(raw, leader, &entry.id, report) else {
                    continue;
                };
                if let Some(owner) = self.rows.iter().find(|r| seqs_overlap(&r.seq, &seq)) {
                    report.conflicts.push(format!(
                        "'{seq}' for {} ({}) is taken by {}",
                        entry.id,
                        layer.origin,
                        owner.command.id()
                    ));
                    continue;
                }
                self.rows.push(Row {
                    seq,
                    command: entry.command,
                    origin: layer.origin.clone(),
                });
            }
        }
    }

    /// Adds the bindings of `defaults` for commands no layer named, on
    /// chords still free: a user's older copy of a built-in preset keeps
    /// getting the actions added since it was written.
    pub fn fill_missing_from(&mut self, defaults: &Assignments<B>) {
        for row in &defaults.rows {
            if !self.mentioned.contains(&row.command)
                && !self.rows.iter().any(|r| seqs_overlap(&r.seq, &row.seq))
            {
                self.rows.push(row.clone());
            }
        }
    }

    /// The dispatcher plus the rows behind it, sorted by chord.
    pub fn finish(mut self) -> (Keybinds<B>, Vec<Row<B>>) {
        self.rows
            .sort_by_cached_key(|r| (r.seq.to_string(), r.command.id()));
        let binds = self
            .rows
            .iter()
            .map(|r| Keybind::new(r.seq.clone(), r.command))
            .collect();
        (Keybinds::new(binds), self.rows)
    }
}

/// Parses the ids of `layer`, merging legacy aliases of one command.
fn entries<B: Bindable>(layer: &Layer, report: &mut KeymapLoadReport) -> Vec<Entry<B>> {
    let mut out: Vec<Entry<B>> = Vec::new();
    for (id, keys) in &layer.bindings {
        let Some(parsed) = parse_id::<B>(id) else {
            report.warnings.push(format!(
                "Unknown action '{id}' in {} — skipped",
                layer.origin
            ));
            continue;
        };
        if parsed.legacy_alias {
            report.warnings.push(format!(
                "'{id}' in {} is a deprecated alias of '{}': list every chord in one key (\"a, b\")",
                layer.origin,
                parsed.command.id()
            ));
        }
        let chords = keys
            .split(',')
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_string);
        match out.iter_mut().find(|e| e.command == parsed.command) {
            Some(entry) => entry.chords.extend(chords),
            None => out.push(Entry {
                command: parsed.command,
                id: parsed.command.id().to_string(),
                chords: chords.collect(),
            }),
        }
    }
    out
}

fn parse_seq(
    raw: &str,
    leader: Option<&str>,
    id: &str,
    report: &mut KeymapLoadReport,
) -> Option<KeySeq> {
    let chord = normalize_chord(raw, leader);
    match chord.parse::<KeySeq>() {
        Ok(seq) => Some(seq),
        Err(e) => {
            report
                .errors
                .push(format!("Invalid key chord '{raw}' for action '{id}': {e}"));
            None
        }
    }
}
