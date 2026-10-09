//! A read-only copy of the live panel keymap for code outside the main loop
//! (the Lua `pairee.keymap` API), refreshed whenever a keymap is built.

use super::actions::Action;
use super::loader::assign::Row;
use super::registry::Bindable;
use std::sync::{OnceLock, RwLock};

/// One live binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedBinding {
    pub chord: String,
    /// Keymap id (`copy`, `plugin.blame.toggle`).
    pub id: String,
    pub label: String,
    /// Where the binding comes from (`preset 'norton'`, `plugin 'blame'`...).
    pub origin: String,
}

fn cell() -> &'static RwLock<Vec<PublishedBinding>> {
    static CELL: OnceLock<RwLock<Vec<PublishedBinding>>> = OnceLock::new();
    CELL.get_or_init(Default::default)
}

/// Replaces the published copy with `rows`.
pub fn publish(rows: &[Row<Action>]) {
    let copy = rows
        .iter()
        .map(|r| PublishedBinding {
            chord: r.seq.to_string(),
            id: r.command.id().to_string(),
            label: r.command.label(),
            origin: r.origin.to_string(),
        })
        .collect();
    *cell().write().expect("published keymap") = copy;
}

/// Every live binding, sorted by chord.
pub fn bindings() -> Vec<PublishedBinding> {
    cell().read().expect("published keymap").clone()
}

/// The first chord bound to keymap id `id`.
pub fn chord_for(id: &str) -> Option<String> {
    cell()
        .read()
        .expect("published keymap")
        .iter()
        .find(|b| b.id == id)
        .map(|b| b.chord.clone())
}
