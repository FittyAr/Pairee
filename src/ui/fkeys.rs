use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::keybindings::keymap::ContextKeymap;
use crate::keybindings::registry::def_for;
use crate::keybindings::screens::ScreenCommand;
use crate::ui::theme_apply::parse_color;
use crossterm::event::KeyModifiers;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// F-key bar label (localization key) of each action that can sit on an
/// F-key. The panel rows are built from the live keymap through this table,
/// so the bar always shows what `F<n>`, `Shift+F<n>`, `Ctrl+F<n>` and
/// `Alt+F<n>` really do.
const ACTION_LABELS: &[(Action, &str)] = &[
    (Action::Help, "fkey_help"),
    (Action::UserMenu, "fkey_user"),
    (Action::View, "fkey_view"),
    (Action::ViewAlt, "fkey_alt_view"),
    (Action::Edit, "fkey_edit"),
    (Action::Copy, "fkey_copy"),
    (Action::Move, "fkey_move"),
    (Action::Rename, "fkey_rename"),
    (Action::MkDir, "fkey_mkdir"),
    (Action::Delete, "fkey_delete"),
    (Action::Menu, "fkey_menu"),
    (Action::Quit, "fkey_quit"),
    (Action::PluginMenu, "fkey_plugin"),
    (Action::ScreensList, "fkey_screen"),
    (Action::CompressFiles, "fkey_sh_pack"),
    (Action::ExtractArchive, "fkey_sh_unpack"),
    (Action::ArchiveCommands, "fkey_sh_arccmd"),
    (Action::MultiRename, "fkey_sh_mrename"),
    (Action::NewFile, "fkey_sh_new"),
    (Action::SaveSetup, "fkey_sh_save"),
    (Action::ContextMenu, "fkey_sh_context"),
    (Action::TogglePanelLeft, "fkey_ctrl_left"),
    (Action::TogglePanelRight, "fkey_ctrl_right"),
    (Action::SortByName, "fkey_ctrl_name"),
    (Action::SortByExtension, "fkey_ctrl_extens"),
    (Action::SortByWriteTime, "fkey_ctrl_time"),
    (Action::SortBySize, "fkey_ctrl_size"),
    (Action::SortUnsorted, "fkey_ctrl_unsort"),
    (Action::SortByCreationTime, "fkey_ctrl_creatn"),
    (Action::SortByAccessTime, "fkey_ctrl_access"),
    (Action::SortByDescription, "fkey_ctrl_descr"),
    (Action::SortByOwner, "fkey_ctrl_owner"),
    (Action::SortModes, "fkey_ctrl_sort"),
    (Action::DriveSelectLeft, "fkey_alt_left"),
    (Action::DriveSelectRight, "fkey_alt_right"),
    (Action::PrintFile, "fkey_alt_print"),
    (Action::CreateLink, "fkey_alt_mklink"),
    (Action::FindFile, "fkey_alt_find"),
    (Action::CommandHistory, "fkey_alt_history"),
    (Action::VideoMode, "fkey_alt_video"),
    (Action::TreeView, "fkey_alt_tree"),
    (Action::FileViewHistory, "fkey_alt_viewhs"),
    (Action::FoldersHistory, "fkey_alt_foldhs"),
];

/// Localization key of the F-key bar label of `action`, if it has one.
fn action_label(action: Action) -> Option<&'static str> {
    ACTION_LABELS
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, key)| *key)
}

/// Chord prefix of the held modifiers, in `keybinds` order (`Ctrl+Alt+Shift+`).
fn modifier_prefix(modifiers: KeyModifiers) -> String {
    [
        (KeyModifiers::CONTROL, "Ctrl+"),
        (KeyModifiers::ALT, "Alt+"),
        (KeyModifiers::SHIFT, "Shift+"),
    ]
    .iter()
    .filter(|(m, _)| modifiers.contains(*m))
    .map(|(_, name)| *name)
    .collect()
}

/// Panel row for the held modifiers, read from the keymap: the label of the
/// action bound to `<prefix>F<n>`, empty when the key is unbound.
fn keymap_cells(context: &AppContext, prefix: &str) -> Vec<(String, String)> {
    cells(|slot| {
        context
            .resolver
            .resolve_for_key_string(&format!("{prefix}F{}", slot + 1))
            .and_then(action_label)
            .map(t)
            .unwrap_or_default()
    })
}

/// Editor / viewer row for the held modifiers: the F-key label of the
/// command `keymap` binds to `<prefix>F<n>`, else of a global panel action
/// (help, screens) bound there.
fn screen_cells<B: ScreenCommand>(
    keymap: &ContextKeymap<B>,
    context: &AppContext,
    prefix: &str,
) -> Vec<(String, String)> {
    cells(|slot| {
        let chord = format!("{prefix}F{}", slot + 1);
        keymap
            .resolve_key_string(&chord)
            .and_then(|command| command.def().fkey)
            .or_else(|| {
                context
                    .resolver
                    .resolve_for_key_string(&chord)
                    .filter(|action| def_for(*action).global)
                    .and_then(action_label)
            })
            .map(t)
            .unwrap_or_default()
    })
}

/// `(key number, label)` cells of the twelve F-keys, labels produced by
/// `label` from the slot index (empty for an unlabeled key).
fn cells(label: impl Fn(usize) -> String) -> Vec<(String, String)> {
    (0..12).map(|i| ((i + 1).to_string(), label(i))).collect()
}

/// The cells shown for the active screen and held modifier keys.
fn bar_cells(context: &AppContext, state: &AppState) -> Vec<(String, String)> {
    let modifiers = state
        .fkeys_modifier_override
        .unwrap_or(state.current_modifiers);
    let prefix = modifier_prefix(modifiers);
    match state.screens.get(state.active_screen_idx) {
        Some(Screen::Editor(_)) => screen_cells(&context.resolver.editor, context, &prefix),
        Some(Screen::Viewer(_)) => screen_cells(&context.resolver.viewer, context, &prefix),
        _ => {
            let mut row = keymap_cells(context, &prefix);
            if modifiers == KeyModifiers::SHIFT && is_dev_plugin_dir(context, state) {
                row[10].1 = t("plugin_install_dev");
            }
            row
        }
    }
}

/// `Shift+F11` installs the plugin under the cursor in developer mode.
fn is_dev_plugin_dir(context: &AppContext, state: &AppState) -> bool {
    context.config.settings.plugins_developer_mode && {
        let active_panel = state.get_active_panel();
        let current_dir = &active_panel.current_path;
        current_dir.join("manifest.toml").exists()
            || active_panel
                .entries
                .get(active_panel.cursor_index)
                .map(|e| e.path.is_dir() && e.path.join("manifest.toml").exists())
                .unwrap_or(false)
    }
}

pub fn render_fkeys(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;
    let fkeys = bar_cells(context, state);

    // Divide the row into 12 equal columns
    let constraints = vec![Constraint::Ratio(1, 12); 12];
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let num_style = Style::default()
        .bg(parse_color(&theme.fkey_bg))
        .fg(parse_color(&theme.fkey_num_fg));

    let text_style = Style::default()
        .bg(parse_color("DarkGray"))
        .fg(parse_color(&theme.fkey_text_fg));

    for (i, (num, text)) in fkeys.iter().enumerate() {
        let block_area = columns[i];

        // Compose block as " 1 Help   "
        let line = Line::from(vec![
            Span::styled(format!(" {:>2}", num), num_style),
            Span::styled(format!(" {:<6}", text), text_style),
        ]);

        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, block_area);
    }

    // Update-available badge: render on top of the last fkey cell
    if state.update.available.is_some() {
        let last_col = columns[11];
        // Build the badge text (fits in the 9-char fkey cell)
        let badge = Span::styled(
            " ▲ UPDATE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        let badge_line = Line::from(badge);
        let badge_paragraph = Paragraph::new(badge_line);
        // Overlay on the last (F12) key column
        f.render_widget(badge_paragraph, last_col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::editor::EditorState;
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn label(cells: &[(String, String)], key: usize) -> &str {
        &cells[key - 1].1
    }

    fn panel_cells(modifiers: KeyModifiers) -> Vec<(String, String)> {
        let context = AppContext::new(AppConfig::default());
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.fkeys_modifier_override = Some(modifiers);
        bar_cells(&context, &state)
    }

    #[test]
    fn editor_bar_shows_save_as_with_shift() {
        let context = AppContext::new(AppConfig::default());
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.push_screen(Screen::Editor(EditorState::from_text("x")));
        let plain = bar_cells(&context, &state);
        assert_eq!(label(&plain, 2), t("fkey_ed_save"));
        assert_eq!(label(&plain, 3), t("fkey_ed_next"));
        assert_eq!(label(&plain, 4), t("fkey_ed_hex"));
        state.fkeys_modifier_override = Some(KeyModifiers::SHIFT);
        let shifted = bar_cells(&context, &state);
        assert_eq!(label(&shifted, 2), t("fkey_ed_save_as"));
        assert_eq!(label(&shifted, 7), t("fkey_ed_next"));
        assert_eq!(label(&shifted, 1), "");
    }

    #[test]
    fn every_row_has_twelve_numbered_cells() {
        let cells = panel_cells(KeyModifiers::NONE);
        assert_eq!(cells.len(), 12);
        assert_eq!(cells[11].0, "12");
    }

    #[test]
    fn panel_rows_follow_the_keymap() {
        let plain = panel_cells(KeyModifiers::NONE);
        assert_eq!(label(&plain, 5), t("fkey_copy"));
        assert_eq!(label(&plain, 11), t("fkey_plugin"), "F11: Far's plugins");
        let shift = panel_cells(KeyModifiers::SHIFT);
        assert_eq!(label(&shift, 1), t("fkey_sh_pack"));
        assert_eq!(label(&shift, 6), t("fkey_rename"));
        let ctrl = panel_cells(KeyModifiers::CONTROL);
        assert_eq!(label(&ctrl, 3), t("fkey_ctrl_name"));
        let alt = panel_cells(KeyModifiers::ALT);
        assert_eq!(label(&alt, 7), t("fkey_alt_find"));
        assert_eq!(label(&alt, 12), t("fkey_alt_foldhs"));
    }

    #[test]
    fn every_bound_function_key_has_a_label() {
        let context = AppContext::new(AppConfig::default());
        let unlabeled: Vec<String> = context
            .resolver
            .rows()
            .iter()
            .map(|row| (row.seq.to_string(), row.command))
            .filter(|(chord, _)| {
                let key = chord.rsplit('+').next().unwrap_or_default();
                key.len() > 1 && key.starts_with('F') && key[1..].parse::<u8>().is_ok()
            })
            .filter(|(_, action)| action_label(*action).is_none())
            .map(|(chord, action)| format!("{chord} {action:?}"))
            .collect();
        assert!(unlabeled.is_empty(), "F-keys without label: {unlabeled:?}");
    }
}
