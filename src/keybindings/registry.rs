//! Lookups over the action [`catalog`](super::catalog): id ↔ action,
//! localized labels and the [`Bindable`] trait the loader is generic over.

use super::actions::Action;
pub use super::catalog::{ActionDef, Category};
use super::catalog::{CATALOG, all_defs};
use super::plugin_commands;
use crate::config::localization::t;
use std::fmt::Debug;
use std::hash::Hash;

/// Anything a keymap layer can bind a chord to. Implemented by [`Action`]
/// (panels); editor / viewer / plugin commands plug into the same loader.
pub trait Bindable: Copy + Eq + Hash + Debug + 'static {
    /// Preset section (`panels`, `editor`, ...) and override prefix.
    const SECTION: &'static str;
    /// The command for a keymap id (`"copy_path"`, `"go_to_tab_3"`).
    fn from_id(id: &str) -> Option<Self>;
    /// Stable keymap id.
    fn id(self) -> &'static str;
    /// Localized human label.
    fn label(self) -> String;
    fn category(self) -> Category;
    /// Must keep a terminal-robust chord in every preset.
    fn essential(self) -> bool;
    /// Every command of this kind.
    fn all() -> Vec<Self>;
}

/// Catalogue row of `action`; every built-in action has one (tested),
/// plugin commands have none.
pub fn def_for(action: Action) -> Option<&'static ActionDef> {
    all_defs().find(|d| d.action == action)
}

/// `action` also works over the editor and viewer screens.
pub fn is_global(action: Action) -> bool {
    def_for(action).is_some_and(|d| d.global)
}

impl Bindable for Action {
    const SECTION: &'static str = "panels";

    fn from_id(id: &str) -> Option<Self> {
        all_defs()
            .find(|d| d.id == id)
            .map(|d| d.action)
            .or_else(|| plugin_commands::find(id).map(Action::Plugin))
    }

    fn id(self) -> &'static str {
        match self {
            Action::Plugin(id) => plugin_commands::keymap_id(id),
            _ => def_for(self).map_or("", |d| d.id),
        }
    }

    fn label(self) -> String {
        match self {
            Action::GoFolderShortcut(n) | Action::GoToTab(n) => {
                let key = if matches!(self, Action::GoToTab(_)) {
                    "action_go_to_tab"
                } else {
                    "action_go_folder_shortcut"
                };
                t(key).replace("{n}", &n.to_string())
            }
            Action::Plugin(id) => {
                plugin_commands::get(id).map_or_else(|| self.id().to_string(), |c| c.title)
            }
            _ => t(&format!("action_{}", self.id())),
        }
    }

    fn category(self) -> Category {
        def_for(self).map_or(Category::Plugins, |d| d.category)
    }

    fn essential(self) -> bool {
        def_for(self).is_some_and(|d| d.essential)
    }

    fn all() -> Vec<Self> {
        all_defs().map(|d| d.action).collect()
    }
}

/// Palette-visible catalogue rows.
pub fn palette_defs() -> impl Iterator<Item = &'static ActionDef> {
    CATALOG.iter().filter(|def| def.in_palette)
}

/// Localized label for an action.
pub fn label_for(action: Action) -> String {
    action.label()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::localization::translator::get_default_english_translation;

    /// Variant names declared in `actions.rs`, read from the source so a new
    /// variant cannot be added without a catalogue row.
    fn declared_variants() -> Vec<String> {
        include_str!("actions.rs")
            .lines()
            .map(str::trim)
            .filter(|l| l.chars().next().is_some_and(char::is_uppercase))
            // Plugin commands are declared by plugins, not catalogued.
            .filter(|l| !l.starts_with("Plugin("))
            .filter_map(|l| l.split(['(', ',']).next())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn every_action_variant_is_catalogued() {
        let catalogued: std::collections::HashSet<String> = all_defs()
            .map(|d| format!("{:?}", d.action))
            .map(|s| s.split('(').next().unwrap().to_string())
            .collect();
        let variants = declared_variants();
        assert!(variants.len() > 100, "parsed {} variants", variants.len());
        for v in variants {
            assert!(catalogued.contains(&v), "{v} has no catalogue row");
        }
    }

    #[test]
    fn ids_and_actions_are_unique_and_round_trip() {
        let mut ids: Vec<&str> = all_defs().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all_defs().count());
        for d in all_defs() {
            assert_eq!(Action::from_id(d.id), Some(d.action), "{}", d.id);
            assert_eq!(d.action.id(), d.id);
        }
    }

    #[test]
    fn every_action_has_an_english_label() {
        for d in CATALOG {
            let key = format!("action_{}", d.id);
            assert_ne!(get_default_english_translation(&key), key, "missing {key}");
        }
        for key in ["action_go_to_tab", "action_go_folder_shortcut"] {
            assert!(
                get_default_english_translation(key).contains("{n}"),
                "{key}"
            );
        }
        for d in all_defs() {
            let key = d.category.label_key();
            assert_ne!(get_default_english_translation(key), key, "missing {key}");
        }
    }

    #[test]
    fn numbered_labels_fill_in_the_slot() {
        assert!(label_for(Action::GoToTab(3)).contains('3'));
        assert!(label_for(Action::GoFolderShortcut(7)).contains('7'));
    }

    #[test]
    fn palette_lists_plain_actions_only() {
        assert!(palette_defs().any(|d| d.id == "copy_path"));
        assert!(palette_defs().all(|d| !d.id.starts_with("go_to_tab_")));
    }
}
