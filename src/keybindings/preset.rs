//! Keymap action-id parsing.
//!
//! Chord validation and dispatch live in [`super::loader`] / [`super::resolver`]
//! via the `keybinds` crate — do not reintroduce string-hash key maps here.

use super::registry::Bindable;

/// Legacy suffixes (`move_up_arrow`, `view_fkey`…) from the time one TOML key
/// could hold a single chord. A key now takes a list (`"k, Up"`), so these
/// are accepted with a deprecation warning only.
const LEGACY_SUFFIXES: &[&str] = &[
    "_arrow", "_pgkey", "_home", "_end", "_enter", "_bs", "_insert", "_fkey", "_alt", "_shift",
    "_rename", "_f10",
];

/// A parsed keymap id: the command, and whether it was spelled with a
/// legacy alias suffix.
pub struct ParsedId<B> {
    pub command: B,
    pub legacy_alias: bool,
}

/// Parses a keymap id (`"copy"`, `"go_to_tab_3"`, legacy `"copy_fkey"`).
pub fn parse_id<B: Bindable>(name: &str) -> Option<ParsedId<B>> {
    let name = name.to_lowercase();
    if let Some(command) = B::from_id(&name) {
        return Some(ParsedId {
            command,
            legacy_alias: false,
        });
    }
    LEGACY_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .and_then(B::from_id)
        .map(|command| ParsedId {
            command,
            legacy_alias: true,
        })
}
