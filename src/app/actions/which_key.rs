//! Which-key overlay: live chords from the `keybinds` map (not a second keymap).

use crate::app::state::{AppState, PopupType};
use crate::keybindings::registry;
use crate::keybindings::{Action, KeybindingResolver};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// One overlay row: display chord, human label, action to run.
pub type WhichKeyItem = (String, String, Action);

/// Snapshot of the live keymap, sorted by chord then label.
pub fn all_items(resolver: &KeybindingResolver) -> Vec<WhichKeyItem> {
    let mut items: Vec<WhichKeyItem> = resolver
        .bindings()
        .map(|(chord, action)| (chord, registry::label_for(action), action))
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    items
}

/// Fuzzy-filter `items` by chord + label (nucleo, same matcher as the palette).
pub fn filter_items(query: &str, items: &[WhichKeyItem]) -> Vec<WhichKeyItem> {
    let q = query.trim();
    if q.is_empty() {
        return items.to_vec();
    }
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(q, CaseMatching::Ignore, Normalization::Smart);
    let mut buf = Vec::new();
    let mut scored: Vec<(u32, WhichKeyItem)> = Vec::new();
    for item in items {
        let snake = item.1.replace(' ', "_");
        let hay = format!("{} {} {snake}", item.0, item.1);
        let utf = Utf32Str::new(&hay, &mut buf);
        if let Some(score) = pattern.score(utf, &mut matcher) {
            scored.push((score, item.clone()));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.0.cmp(&b.1.0)));
    scored.into_iter().map(|(_, item)| item).collect()
}

/// Open the which-key overlay on `state` from the current resolver map.
pub fn open_which_key(state: &mut AppState, resolver: &KeybindingResolver) {
    let items = all_items(resolver);
    state.dialogs.replace(PopupType::WhichKey {
        query: String::new(),
        cursor_idx: 0,
        items,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AppConfig, keybindings::KeybindingsConfig, settings::Settings, theme::Theme,
    };

    fn resolver_with(extra: &[(&str, &str)]) -> KeybindingResolver {
        let mut config = AppConfig {
            settings: Settings::default(),
            theme: Theme::default(),
            keybindings: KeybindingsConfig::default(),
        };
        for (action, chord) in extra {
            config
                .keybindings
                .custom_bindings
                .insert((*action).into(), (*chord).into());
        }
        KeybindingResolver::new(&config)
    }

    #[test]
    fn items_include_injected_which_key_and_copy() {
        let resolver = resolver_with(&[("which_key", "Ctrl+Alt+Shift+F11"), ("copy", "F5")]);
        let items = all_items(&resolver);
        assert!(
            items
                .iter()
                .any(|(_, _, action)| *action == Action::WhichKey),
            "which_key must appear as a live binding, got {items:?}"
        );
        assert!(
            items
                .iter()
                .any(|(chord, _, action)| chord.eq_ignore_ascii_case("F5")
                    && *action == Action::Copy),
            "expected F5 copy in {items:?}"
        );
    }

    #[test]
    fn filter_by_chord_finds_copy() {
        let items = all_items(&resolver_with(&[("copy", "F5")]));
        let filtered = filter_items("F5", &items);
        assert!(
            filtered.iter().any(|(_, _, a)| *a == Action::Copy),
            "F5 should match copy, got {filtered:?}"
        );
    }

    #[test]
    fn filter_by_label_finds_copy_path() {
        let items = all_items(&resolver_with(&[("copy_path", "Ctrl+Alt+Shift+F12")]));
        let filtered = filter_items("copy path", &items);
        assert!(
            filtered.iter().any(|(_, _, a)| *a == Action::CopyPath),
            "label query should match copy path, got {filtered:?}"
        );
    }

    #[test]
    fn filter_unknown_query_is_empty() {
        let items = all_items(&resolver_with(&[]));
        assert!(filter_items("zzzz-no-such-chord", &items).is_empty());
    }

    #[test]
    fn open_which_key_replaces_dialog() {
        let resolver = resolver_with(&[("which_key", "Ctrl+Alt+Shift+F11")]);
        let mut state = AppState::new(std::path::PathBuf::from("."), std::path::PathBuf::from("."));
        open_which_key(&mut state, &resolver);
        match state.dialogs.top() {
            Some(PopupType::WhichKey { items, query, .. }) => {
                assert!(query.is_empty());
                assert!(!items.is_empty());
            }
            other => panic!("expected WhichKey, got {other:?}"),
        }
    }
}
