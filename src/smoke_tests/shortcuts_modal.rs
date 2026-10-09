//! The keyboard-shortcuts modal, driven by keys as a user would.

use crate::app::shortcuts::state::ShortcutsState;
use crate::app::state::{ActivePanel, PopupType};
use crate::keybindings::Action;
use crate::keybindings::screens::EditorAction;
use crate::test_harness::Harness;

fn open() -> Harness {
    let mut h = Harness::new();
    h.keys("@which_key");
    assert!(modal(&h).is_some(), "modal open");
    h
}

fn modal(h: &Harness) -> Option<&ShortcutsState> {
    match h.state.dialogs.top() {
        Some(PopupType::Shortcuts(s)) => Some(s),
        _ => None,
    }
}

fn current_id(h: &Harness) -> String {
    modal(h)
        .and_then(|s| s.current())
        .map(|r| r.id.clone())
        .unwrap_or_default()
}

fn copy_keys(h: &Harness) -> Vec<String> {
    h.ctx
        .resolver
        .key_for_action(Action::Copy)
        .map(str::to_string)
        .into_iter()
        .collect()
}

#[test]
fn filter_rebind_with_conflict_add_remove_and_restore() {
    let mut h = open();
    h.text("copy to other");
    assert_eq!(current_id(&h), "copy");
    // F11 belongs to the plugins menu: confirm taking it.
    h.keys("F2 F11 Enter");
    assert!(matches!(
        modal(&h).unwrap().mode,
        crate::app::shortcuts::state::Mode::ConfirmReplace { .. }
    ));
    h.keys("Enter");
    assert_eq!(copy_keys(&h), ["F11"]);
    assert_eq!(h.ctx.resolver.key_for_action(Action::PluginMenu), None);
    assert_eq!(
        h.ctx.config.keybindings.merged_overrides("norton")["copy"],
        "F11"
    );
    // Add a second key, then remove them all, then restore the preset's.
    h.keys("Insert Ctrl+Alt+y Enter");
    assert_eq!(modal(&h).unwrap().current().unwrap().chords.len(), 2);
    h.keys("Delete");
    assert!(copy_keys(&h).is_empty());
    h.keys("F8");
    assert_eq!(copy_keys(&h), ["F5"]);
    assert!(h.ctx.config.keybindings.overrides.is_empty());
}

#[test]
fn find_a_key_and_rebind_an_editor_command() {
    let mut h = open();
    h.keys("F3 Ctrl+s");
    let s = modal(&h).unwrap();
    assert_eq!(s.tab, crate::app::shortcuts::model::ContextTab::Editor);
    assert_eq!(current_id(&h), "save");
    h.keys("F2 Ctrl+w Enter");
    assert_eq!(
        h.ctx.resolver.editor.key_for(EditorAction::Save),
        Some("Ctrl+w")
    );
}

#[test]
fn preview_activate_and_export_a_preset() {
    let mut h = open();
    h.keys("Ctrl+Right");
    assert_eq!(modal(&h).unwrap().preset, "standard");
    h.keys("F5");
    assert_eq!(h.ctx.config.keybindings.preset, "standard");
    h.keys("F9");
    for _ in 0.."standard-mine".len() {
        h.keys("Backspace");
    }
    h.text("mine").keys("Enter");
    let path = h.config_dir().join("keymaps/mine.toml");
    let text = std::fs::read_to_string(&path).expect("exported preset");
    assert!(text.contains("extends = \"standard\""), "{text}");
}

#[test]
fn enter_runs_the_highlighted_action() {
    let mut h = open();
    h.text("focus right");
    h.keys("Enter");
    assert!(h.state.dialogs.is_none());
    assert_eq!(h.state.panels.active, ActivePanel::Right);
}

#[test]
fn escape_returns_to_the_settings_dialog_it_was_opened_from() {
    let mut h = Harness::new();
    h.keys("@system_settings");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ConfigurationDialog(_))
    ));
    // Settings → Interface → "Keyboard shortcuts…" pushes the modal.
    let rows = crate::app::shortcuts::edit::rows_for(&h.ctx, "norton");
    h.state
        .dialogs
        .push(PopupType::Shortcuts(Box::new(ShortcutsState::new(
            "norton".into(),
            rows,
        ))));
    h.keys("Esc");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ConfigurationDialog(_))
    ));
}
