use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Returns the localization key used in the F-key bar for the given action, if any.
fn action_label(action: Action) -> Option<&'static str> {
    match action {
        Action::Help => Some("fkey_help"),
        Action::UserMenu => Some("fkey_user"),
        Action::View => Some("fkey_view"),
        Action::Edit => Some("fkey_edit"),
        Action::Copy => Some("fkey_copy"),
        Action::Move => Some("fkey_move"),
        Action::Rename => Some("fkey_rename"),
        Action::MkDir => Some("fkey_mkdir"),
        Action::Delete => Some("fkey_delete"),
        Action::Menu => Some("fkey_menu"),
        Action::Quit => Some("fkey_quit"),
        Action::PluginMenu => Some("fkey_plugin"),
        Action::ScreensList => Some("fkey_screen"),
        _ => None,
    }
}

/// Resolve what label should be displayed in slot `n` (0-indexed, where 0 = F1).
/// Queries the resolver first so the bar always matches the actual binding.
fn slot_label(context: &AppContext, slot: usize, fallback_key: &str) -> String {
    let key = format!("F{}", slot + 1);
    if let Some(action) = context.resolver.resolve_for_key_string(&key)
        && let Some(label_key) = action_label(action)
    {
        return t(label_key);
    }
    translated(slot, fallback_key)
}

/// Labels of the twelve F-keys, `""` for an unlabeled key.
type Row = [&'static str; 12];

const EDITOR_ROW: Row = [
    "fkey_help",
    "fkey_ed_save",
    "fkey_ed_next",
    "fkey_ed_hex",
    "",
    "",
    "fkey_ed_search",
    "fkey_ed_discard",
    "",
    "fkey_ed_quit",
    "",
    "",
];

const EDITOR_SHIFT_ROW: Row = [
    "",
    "fkey_ed_save_as",
    "",
    "",
    "",
    "",
    "fkey_ed_next",
    "",
    "",
    "",
    "",
    "",
];

const VIEWER_ROW: Row = [
    "fkey_help",
    "",
    "",
    "fkey_vw_hex",
    "",
    "fkey_edit",
    "fkey_vw_search",
    "",
    "",
    "fkey_vw_quit",
    "",
    "",
];

const CTRL_ROW: Row = [
    "fkey_ctrl_left",
    "fkey_ctrl_right",
    "fkey_ctrl_name",
    "fkey_ctrl_extens",
    "fkey_ctrl_time",
    "fkey_ctrl_size",
    "fkey_ctrl_unsort",
    "fkey_ctrl_creatn",
    "fkey_ctrl_access",
    "fkey_ctrl_descr",
    "fkey_ctrl_owner",
    "fkey_ctrl_sort",
];

const ALT_ROW: Row = [
    "fkey_alt_left",
    "fkey_alt_right",
    "fkey_alt_view",
    "fkey_alt_edit",
    "fkey_alt_print",
    "fkey_alt_mklink",
    "fkey_alt_find",
    "fkey_alt_history",
    "fkey_alt_video",
    "fkey_alt_tree",
    "fkey_alt_viewhs",
    "fkey_alt_foldhs",
];

/// Default panel row; each key is resolved from the user's keymap first.
const PANEL_ROW: Row = [
    "fkey_help",
    "fkey_user",
    "fkey_view",
    "fkey_edit",
    "fkey_copy",
    "fkey_move",
    "fkey_rename",
    "fkey_delete",
    "fkey_menu",
    "fkey_quit",
    "", // F11 (unbound by default — Plugin menu lives under F9 → Files)
    "fkey_screen",
];

/// `(key number, label)` cells of a row, labels produced by `label`.
fn cells(row: &Row, label: impl Fn(usize, &str) -> String) -> Vec<(String, String)> {
    row.iter()
        .enumerate()
        .map(|(i, key)| ((i + 1).to_string(), label(i, key)))
        .collect()
}

/// Translated label, empty for an unlabeled key.
fn translated(_slot: usize, key: &str) -> String {
    if key.is_empty() {
        String::new()
    } else {
        t(key)
    }
}

/// The cells shown for the active screen and held modifier keys.
fn bar_cells(context: &AppContext, state: &AppState) -> Vec<(String, String)> {
    use crossterm::event::KeyModifiers;
    let modifiers = state
        .fkeys_modifier_override
        .unwrap_or(state.current_modifiers);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    match state.screens.get(state.active_screen_idx) {
        Some(Screen::Editor(_)) if shift => cells(&EDITOR_SHIFT_ROW, translated),
        Some(Screen::Editor(_)) => cells(&EDITOR_ROW, translated),
        Some(Screen::Viewer(_)) => cells(&VIEWER_ROW, translated),
        _ if modifiers.contains(KeyModifiers::CONTROL) => cells(&CTRL_ROW, translated),
        _ if modifiers.contains(KeyModifiers::ALT) => cells(&ALT_ROW, translated),
        _ if shift => {
            let dev_install = is_dev_plugin_dir(context, state);
            cells(&[""; 12], |slot, _| {
                if dev_install && slot == 10 {
                    t("plugin_install_dev")
                } else {
                    String::new()
                }
            })
        }
        _ => cells(&PANEL_ROW, |slot, key| slot_label(context, slot, key)),
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
    use crossterm::event::KeyModifiers;
    use std::path::PathBuf;

    fn label(cells: &[(String, String)], key: usize) -> &str {
        &cells[key - 1].1
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
        let context = AppContext::new(AppConfig::default());
        let state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let cells = bar_cells(&context, &state);
        assert_eq!(cells.len(), 12);
        assert_eq!(cells[11].0, "12");
    }
}
