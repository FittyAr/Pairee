//! Changes made from the shortcuts modal. They are user overrides in
//! `keybindings.toml` (`[overrides.<preset>]`), saved at once; the live
//! keymap is rebuilt when the edited preset is the active one.

use super::model::{ShortcutRow, build_rows};
use super::state::ShortcutsState;
use crate::app::context::AppContext;
use crate::config::localization::t;
use crate::keybindings::embedded::{available_presets, normalize_preset_name};
use crate::keybindings::loader::{KeymapSpec, LoadedKeymap, load_keymap, split_sections};
use crate::keybindings::{KeybindingResolver, plugin_commands};
use std::path::PathBuf;

/// The keymap of `preset` with the user's overrides and loaded plugins.
fn load(context: &AppContext, preset: &str) -> LoadedKeymap {
    load_keymap(&KeymapSpec {
        preset,
        keybindings: &context.config.keybindings,
        yazi_letters: context.config.settings.enable_yazi_workflow,
        plugins: plugin_commands::active(),
    })
}

/// Rows of `preset` as the modal shows them.
pub fn rows_for(context: &AppContext, preset: &str) -> Vec<ShortcutRow> {
    build_rows(&load(context, preset), &context.config.keybindings, preset)
}

fn active_preset(context: &AppContext) -> String {
    normalize_preset_name(&context.config.keybindings.preset)
}

/// Saves the overrides, rebuilds the live keymap when `s` shows the active
/// preset, and reloads the rows.
fn apply(context: &mut AppContext, s: &mut ShortcutsState) {
    context.config.save_logging();
    if s.preset == active_preset(context) {
        context.resolver = KeybindingResolver::new(&context.config);
    }
    s.rows = rows_for(context, &s.preset);
    s.clamp_cursor();
}

/// Gives the cursor row exactly `chords` (none unbinds it).
pub fn assign(context: &mut AppContext, s: &mut ShortcutsState, chords: &[String]) {
    let Some(row) = s.current() else {
        return;
    };
    let id = row.override_id();
    context
        .config
        .keybindings
        .set_override(&s.preset, &id, &chords.join(", "));
    s.message = Some(t("shortcuts_saved"));
    apply(context, s);
}

/// Gives the cursor row back the keys of its preset.
pub fn restore(context: &mut AppContext, s: &mut ShortcutsState) {
    if let Some(row) = s.current() {
        let id = row.override_id();
        context.config.keybindings.restore(&s.preset, &id);
        s.message = Some(t("shortcuts_restored"));
        apply(context, s);
    }
}

/// Drops every override of the previewed preset.
pub fn restore_all(context: &mut AppContext, s: &mut ShortcutsState) {
    context.config.keybindings.restore_all(&s.preset);
    s.message = Some(t("shortcuts_restored"));
    apply(context, s);
}

/// Shows the next (`step` = 1) or previous (`-1`) preset.
pub fn preview(context: &AppContext, s: &mut ShortcutsState, step: isize) {
    let names = available_presets(&crate::config::paths::get_keymaps_dir());
    let i = names.iter().position(|n| *n == s.preset).unwrap_or(0) as isize;
    s.preset = names[(i + step).rem_euclid(names.len() as isize) as usize].clone();
    s.rows = rows_for(context, &s.preset);
    s.cursor = 0;
    s.message = (s.preset != active_preset(context)).then(|| t("shortcuts_preview"));
}

/// Makes the previewed preset the active one.
pub fn activate(context: &mut AppContext, s: &mut ShortcutsState) {
    crate::app::sys_helpers::config::change_preset(context, &s.preset);
    s.message = Some(t("shortcuts_activated").replace("{}", &s.preset));
}

/// Writes `keymaps/<name>.toml`: the previewed preset plus the user's
/// overrides, as a preset of its own.
pub fn export(context: &AppContext, s: &ShortcutsState, name: &str) -> Result<PathBuf, String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && crate::keybindings::embedded::preset_toml(name).is_none();
    if !valid {
        return Err(t("shortcuts_export_bad_name"));
    }
    let text = exported_preset(context, &s.preset, name);
    let dir = crate::config::paths::get_keymaps_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.toml"));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

fn exported_preset(context: &AppContext, preset: &str, name: &str) -> String {
    let overrides = context.config.keybindings.merged_overrides(preset);
    let mut table = toml::Table::new();
    table.insert("extends".into(), preset.into());
    for (section, keys) in split_sections(&overrides) {
        let keys: toml::Table = keys.into_iter().map(|(k, v)| (k, v.into())).collect();
        table.insert(section, toml::Value::Table(keys));
    }
    format!(
        "# Pairee keymap: {name} (made from '{preset}' and your changes)\n\n{}",
        toml::to_string(&table).unwrap_or_default()
    )
}
