//! Keys that plugins bind in their manifest (`[keybindings]`), looked up
//! when the core keymap does not resolve a key.

use super::keyspec::canonical_key;
use std::collections::{BTreeMap, HashMap};

/// Canonical key (resolver display form) → plugin name → action name.
///
/// The inner map is ordered, so when two plugins bind the same key the one
/// whose name sorts first wins, whatever order the plugins loaded in.
#[derive(Debug, Default)]
pub struct PluginKeymap {
    by_key: HashMap<String, BTreeMap<String, String>>,
}

impl PluginKeymap {
    /// Replaces the bindings of `plugin` with `bindings` (manifest key spec
    /// → action). Returns the specs that are not valid keys.
    pub fn register(&mut self, plugin: &str, bindings: &HashMap<String, String>) -> Vec<String> {
        self.unregister(plugin);
        let mut invalid = Vec::new();
        for (spec, action) in bindings {
            match canonical_key(spec) {
                Some(key) => {
                    self.by_key
                        .entry(key)
                        .or_default()
                        .insert(plugin.to_string(), action.clone());
                }
                None => invalid.push(spec.clone()),
            }
        }
        invalid.sort();
        invalid
    }

    /// Drops every binding of `plugin`.
    pub fn unregister(&mut self, plugin: &str) {
        self.by_key.retain(|_, plugins| {
            plugins.remove(plugin);
            !plugins.is_empty()
        });
    }

    /// `(plugin, action)` bound to `key` (resolver display form).
    pub fn resolve(&self, key: &str) -> Option<(String, String)> {
        let (plugin, action) = self.by_key.get(key)?.iter().next()?;
        Some((plugin.clone(), action.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::resolver::key_event_to_string;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn bindings(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, a)| (k.to_string(), a.to_string()))
            .collect()
    }

    fn ctrl(c: char) -> String {
        key_event_to_string(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
    }

    #[test]
    fn manifest_spellings_fire_on_the_pressed_key() {
        let mut map = PluginKeymap::default();
        let invalid = map.register(
            "demo",
            &bindings(&[("ctrl+h", "hello"), ("ctrl-p", "pick")]),
        );
        assert!(invalid.is_empty());
        assert_eq!(
            map.resolve(&ctrl('h')),
            Some(("demo".into(), "hello".into()))
        );
        assert_eq!(
            map.resolve(&ctrl('p')),
            Some(("demo".into(), "pick".into()))
        );
    }

    #[test]
    fn first_plugin_by_name_wins_regardless_of_load_order() {
        for order in [["zeta", "alpha"], ["alpha", "zeta"]] {
            let mut map = PluginKeymap::default();
            for plugin in order {
                map.register(plugin, &bindings(&[("ctrl+h", plugin)]));
            }
            assert_eq!(
                map.resolve(&ctrl('h')),
                Some(("alpha".into(), "alpha".into()))
            );
        }
    }

    #[test]
    fn unregister_hands_the_key_to_the_next_plugin() {
        let mut map = PluginKeymap::default();
        map.register("alpha", &bindings(&[("ctrl+h", "a")]));
        map.register("zeta", &bindings(&[("ctrl+h", "z")]));
        map.unregister("alpha");
        assert_eq!(map.resolve(&ctrl('h')), Some(("zeta".into(), "z".into())));
        map.unregister("zeta");
        assert_eq!(map.resolve(&ctrl('h')), None);
    }

    #[test]
    fn re_registering_drops_old_keys_and_reports_invalid_ones() {
        let mut map = PluginKeymap::default();
        map.register("demo", &bindings(&[("ctrl+h", "old")]));
        let invalid = map.register("demo", &bindings(&[("ctrl+j", "new"), ("ctrl+bogus", "x")]));
        assert_eq!(invalid, vec!["ctrl+bogus".to_string()]);
        assert_eq!(map.resolve(&ctrl('h')), None);
        assert_eq!(map.resolve(&ctrl('j')), Some(("demo".into(), "new".into())));
    }
}
