//! Commands that plugins declare (`[[commands]]` in their manifest), so the
//! keymap, which-key, the palette and the shortcuts modal treat them like
//! built-in actions.
//!
//! A command is interned once as a [`PluginCommandId`] (stable for the whole
//! session, so a keymap built earlier never points at another command) and
//! is active while its plugin is loaded. Changes raise a flag the main loop
//! checks to rebuild the keymap.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Index of an interned plugin command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginCommandId(pub u32);

/// A command as a plugin declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCommand {
    pub plugin: String,
    /// Id inside the plugin (`toggle`).
    pub command: String,
    pub title: String,
    /// Preset name or `default` → chords (`"Alt+Shift+B"`, `"<leader>gb"`).
    pub keys: BTreeMap<String, String>,
}

impl PluginCommand {
    /// Keymap id: `plugin.<plugin>.<command>`.
    pub fn keymap_id(&self) -> String {
        format!("plugin.{}.{}", self.plugin, self.command)
    }

    /// The chords suggested for `preset`, else the default ones.
    pub fn keys_for(&self, preset: &str) -> Option<&str> {
        self.keys
            .get(preset)
            .or_else(|| self.keys.get("default"))
            .map(String::as_str)
    }
}

struct Entry {
    command: PluginCommand,
    /// Interned keymap id (leaked once per command for `&'static str`).
    id: &'static str,
    active: bool,
}

#[derive(Default)]
struct Store {
    entries: Vec<Entry>,
}

/// The command store: shared by the whole app; private to each thread in
/// tests, so tests running in parallel never see each other's plugins.
#[cfg(not(test))]
fn store() -> &'static RwLock<Store> {
    static STORE: std::sync::OnceLock<RwLock<Store>> = std::sync::OnceLock::new();
    STORE.get_or_init(Default::default)
}

#[cfg(test)]
fn store() -> &'static RwLock<Store> {
    thread_local! {
        static STORE: &'static RwLock<Store> = Box::leak(Box::default());
    }
    STORE.with(|s| *s)
}

/// Raised when commands change; see [`store`] for the test split.
#[cfg(not(test))]
fn dirty() -> &'static AtomicBool {
    static DIRTY: AtomicBool = AtomicBool::new(false);
    &DIRTY
}

#[cfg(test)]
fn dirty() -> &'static AtomicBool {
    thread_local! {
        static DIRTY: &'static AtomicBool = Box::leak(Box::default());
    }
    DIRTY.with(|d| *d)
}

/// Makes `commands` the active commands of `plugin` (replacing earlier ones).
pub fn register(plugin: &str, commands: Vec<PluginCommand>) {
    let mut store = store().write().expect("plugin command store");
    for entry in store
        .entries
        .iter_mut()
        .filter(|e| e.command.plugin == plugin)
    {
        entry.active = false;
    }
    for command in commands {
        let key = command.keymap_id();
        match store.entries.iter_mut().find(|e| e.id == key) {
            Some(entry) => {
                entry.command = command;
                entry.active = true;
            }
            None => store.entries.push(Entry {
                id: Box::leak(key.into_boxed_str()),
                command,
                active: true,
            }),
        }
    }
    dirty().store(true, Ordering::Release);
}

/// Deactivates the commands of an unloaded plugin.
pub fn unregister(plugin: &str) {
    let mut store = store().write().expect("plugin command store");
    for entry in store
        .entries
        .iter_mut()
        .filter(|e| e.command.plugin == plugin)
    {
        entry.active = false;
    }
    dirty().store(true, Ordering::Release);
}

/// `true` (once) when commands changed since the last call.
pub fn take_changed() -> bool {
    dirty().swap(false, Ordering::AcqRel)
}

/// The command behind `id` (also when its plugin was unloaded).
pub fn get(id: PluginCommandId) -> Option<PluginCommand> {
    let store = store().read().expect("plugin command store");
    store.entries.get(id.0 as usize).map(|e| e.command.clone())
}

/// Interned keymap id of `id` (`""` for an unknown id).
pub fn keymap_id(id: PluginCommandId) -> &'static str {
    let store = store().read().expect("plugin command store");
    store.entries.get(id.0 as usize).map_or("", |e| e.id)
}

/// The active command with keymap id `id` (`plugin.<name>.<command>`).
pub fn find(id: &str) -> Option<PluginCommandId> {
    let store = store().read().expect("plugin command store");
    store
        .entries
        .iter()
        .position(|e| e.active && e.id == id)
        .map(|i| PluginCommandId(i as u32))
}

/// Active commands, by plugin then command id.
pub fn active() -> Vec<(PluginCommandId, PluginCommand)> {
    let store = store().read().expect("plugin command store");
    let mut out: Vec<_> = store
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.active)
        .map(|(i, e)| (PluginCommandId(i as u32), e.command.clone()))
        .collect();
    out.sort_by_cached_key(|(_, c)| c.keymap_id());
    out
}

/// A command titled like its id, with `(preset, chords)` keys (tests).
#[cfg(test)]
pub fn test_command(plugin: &str, command: &str, keys: &[(&str, &str)]) -> PluginCommand {
    PluginCommand {
        plugin: plugin.into(),
        command: command.into(),
        title: command.into(),
        keys: keys
            .iter()
            .map(|(p, k)| (p.to_string(), k.to_string()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::test_command as cmd;
    use super::*;

    #[test]
    fn ids_survive_reloads_and_unloading_deactivates() {
        register(
            "pc-test-a",
            vec![cmd("pc-test-a", "one", &[("default", "Alt+1")])],
        );
        let id = find("plugin.pc-test-a.one").unwrap();
        register(
            "pc-test-a",
            vec![cmd("pc-test-a", "one", &[("default", "Alt+2")])],
        );
        assert_eq!(
            find("plugin.pc-test-a.one"),
            Some(id),
            "same id after reload"
        );
        assert_eq!(get(id).unwrap().keys_for("norton"), Some("Alt+2"));
        unregister("pc-test-a");
        assert_eq!(find("plugin.pc-test-a.one"), None);
        assert_eq!(
            keymap_id(id),
            "plugin.pc-test-a.one",
            "old keymaps keep a name"
        );
    }

    #[test]
    fn preset_keys_win_over_default() {
        let c = cmd("p", "c", &[("default", "Alt+b"), ("neovim", "<leader>b")]);
        assert_eq!(c.keys_for("neovim"), Some("<leader>b"));
        assert_eq!(c.keys_for("yazi"), Some("Alt+b"));
    }
}
