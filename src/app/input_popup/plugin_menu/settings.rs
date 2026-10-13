//! The plugin settings form: opening it from the Plugin Manager (`c` or
//! Enter on an installed plugin) and its keys. Space toggles a checkbox,
//! Tab / arrows move, Enter saves (or presses the focused button), Esc
//! cancels. Saving checks every value against its schema type, stores the
//! values that differ from the defaults in `plugin_settings.<name>` of
//! `config.toml` and reloads the plugins so the change applies at once.

use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::state::popup::{
    BUTTON_DEFAULTS, BUTTON_SAVE, Dialog, KeyHandler, KeyOutcome, PluginSettingsDialog,
};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::plugin::runtime::settings::{self, Kind};
use crossterm::event::KeyEvent;

/// Opens the settings form of the installed plugin `plugin` over the
/// Plugin Manager, or says that it has none.
pub fn open(state: &mut AppState, context: &AppContext, plugin: &str) {
    let dir = crate::config::paths::get_installed_plugin_dir(plugin);
    let specs = crate::plugin::loader::PluginManifest::read_from_dir(&dir)
        .map(|manifest| settings::specs(&manifest))
        .unwrap_or_default();
    if specs.is_empty() {
        let text = t("plugin_settings_none").replacen("{}", plugin, 1);
        state.dialogs.push(PopupType::info(text));
        return;
    }
    let current = context
        .config
        .settings
        .plugin_settings
        .get(plugin)
        .cloned()
        .unwrap_or_default();
    state
        .dialogs
        .push(PluginSettingsDialog::new(plugin, specs, &current));
}

/// What a valid value of `kind` looks like, for the error line.
fn kind_label(kind: Kind) -> String {
    t(match kind {
        Kind::Bool => "plugin_settings_kind_bool",
        Kind::Integer => "plugin_settings_kind_integer",
        Kind::Number => "plugin_settings_kind_number",
        Kind::Text => "plugin_settings_kind_text",
    })
}

/// Saves the form (and closes it), or shows why a value is refused.
fn save(state: &mut AppState, context: &mut AppContext) {
    let Some(dialog) = state.dialogs.top_mut_as::<PluginSettingsDialog>() else {
        return;
    };
    match dialog.values() {
        Err(row) => {
            let spec = &dialog.rows[row].spec;
            let message = t("plugin_settings_invalid")
                .replacen("{}", &spec.key, 1)
                .replacen("{}", &kind_label(spec.kind), 1);
            dialog.error = Some((row, message));
            dialog.focus = row;
        }
        Ok(values) => {
            let plugin = dialog.plugin.clone();
            crate::plugin::manager::toggles::set_settings(&mut context.config, &plugin, values);
            state.dialogs.pop();
        }
    }
}

fn handle(state: &mut AppState, key: KeyEvent, context: &mut AppContext) -> KeyOutcome {
    let Some(dialog) = state.dialogs.top_mut_as::<PluginSettingsDialog>() else {
        return Err(());
    };
    let layout = dialog.layout();
    let mut focus = dialog.focus;
    let outcome = layout.handle(&mut focus, dialog.focused_field(), &key);
    dialog.focus = focus;
    match outcome {
        FormKey::Toggle(row) => dialog.toggle(row),
        FormKey::Activate(row) => match layout.focused_button(row) {
            Some(BUTTON_DEFAULTS) => dialog.reset(),
            Some(BUTTON_SAVE) | None => save(state, context),
            Some(_) => {
                state.dialogs.pop();
            }
        },
        FormKey::Cancel => {
            state.dialogs.pop();
        }
        FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}

impl Dialog for PluginSettingsDialog {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        self.focused_field()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crossterm::event::KeyCode;
    use std::path::PathBuf;

    const MANIFEST: &str = r#"
        name = "demo"
        version = "1.0.0"
        [settings_schema]
        depth = { type = "integer", default = 3, description = "How deep" }
        flag = { type = "bool", default = false }
    "#;

    fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
        let _ = handle(state, KeyEvent::from(code), context);
    }

    #[test]
    fn the_form_saves_changed_values_and_refuses_bad_ones() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let plugin_dir = crate::config::paths::get_installed_plugin_dir("demo");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("manifest.toml"), MANIFEST).unwrap();
        let mut context = AppContext::new(AppConfig::default());
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        open(&mut state, &context, "demo");
        assert!(state.dialogs.top_as::<PluginSettingsDialog>().is_some());

        // depth: "3" -> "3x" is refused.
        press(&mut state, &mut context, KeyCode::End);
        press(&mut state, &mut context, KeyCode::Char('x'));
        press(&mut state, &mut context, KeyCode::Enter);
        let dialog = state.dialogs.top_as::<PluginSettingsDialog>().unwrap();
        assert_eq!(dialog.error.as_ref().map(|e| e.0), Some(0));

        // "3" -> "4", flag on, Enter saves.
        press(&mut state, &mut context, KeyCode::Backspace);
        press(&mut state, &mut context, KeyCode::Backspace);
        press(&mut state, &mut context, KeyCode::Char('4'));
        press(&mut state, &mut context, KeyCode::Down);
        press(&mut state, &mut context, KeyCode::Char(' '));
        press(&mut state, &mut context, KeyCode::Enter);
        assert!(state.dialogs.top_as::<PluginSettingsDialog>().is_none());
        let saved = &context.config.settings.plugin_settings["demo"];
        assert_eq!(saved.get("depth").map(String::as_str), Some("4"));
        assert_eq!(saved.get("flag").map(String::as_str), Some("true"));
    }

    #[test]
    fn a_plugin_without_settings_gets_a_message() {
        let dir = tempfile::tempdir().unwrap();
        let _root = crate::config::paths::test_root::redirect(dir.path());
        let context = AppContext::new(AppConfig::default());
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        open(&mut state, &context, "missing");
        assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
    }
}
