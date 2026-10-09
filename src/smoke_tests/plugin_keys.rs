//! Plugin commands live in the keymap: their suggested keys bind when free,
//! they show in the palette, and the actions plugins emit run.

use crate::app::actions::command_palette::all_palette_items;
use crate::keybindings::{Action, plugin_commands};
use crate::plugin::loader::PluginManifest;
use crate::terminal::Event;
use crate::test_harness::Harness;
use std::path::PathBuf;

/// Registers the commands of `tests/plugin_acceptance/keymap_command`.
fn register_acceptance_plugin() -> plugin_commands::PluginCommandId {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/plugin_acceptance/keymap_command");
    let manifest = PluginManifest::read_from_dir(&dir).expect("manifest");
    plugin_commands::register(&manifest.name, manifest.keymap_commands());
    plugin_commands::find("plugin.acceptance-keymap.hop").expect("hop registered")
}

#[test]
fn loaded_plugin_commands_join_the_keymap_and_the_palette() {
    let mut h = Harness::new();
    let hop = register_acceptance_plugin();
    // Any event lets the loop pick up the change and rebuild the keymap.
    h.send(Event::Tick);
    assert_eq!(
        h.ctx.resolver.key_for_action(Action::Plugin(hop)),
        Some("Alt+H")
    );
    assert!(
        all_palette_items()
            .iter()
            .any(|(label, action)| *action == Action::Plugin(hop) && label.contains("Hop"))
    );
    let rows = &h.ctx.resolver.load_report().conflicts;
    assert!(rows.is_empty(), "{rows:?}");
}

#[test]
fn emitted_actions_run_like_key_presses() {
    let mut h = Harness::new();
    h.state
        .plugins
        .emitted_actions
        .push(Action::FocusRightPanel);
    h.send(Event::Tick);
    assert_eq!(h.state.panels.active, crate::app::state::ActivePanel::Right);
    assert!(h.state.plugins.emitted_actions.is_empty());
}
