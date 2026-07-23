use crate::app::context::AppContext;
use crate::app::state::AppState;
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
    if let Some(action) = context.resolver.resolve_for_key_string(&key) {
        if let Some(label_key) = action_label(action) {
            return t(label_key);
        }
    }
    t(fallback_key)
}

pub fn render_fkeys(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;

    let is_editor = matches!(
        state.screens.get(state.active_screen_idx),
        Some(crate::app::state::Screen::Editor(_))
    );
    let is_viewer = matches!(
        state.screens.get(state.active_screen_idx),
        Some(crate::app::state::Screen::Viewer(_))
    );

    let modifiers = state
        .fkeys_modifier_override
        .unwrap_or(state.current_modifiers);

    let fkeys: Vec<(String, String)> = if is_editor {
        vec![
            ("1".to_string(), t("fkey_help")),
            ("2".to_string(), t("fkey_ed_save")),
            ("3".to_string(), String::new()),
            ("4".to_string(), t("fkey_ed_hex")),
            ("5".to_string(), String::new()),
            ("6".to_string(), String::new()),
            ("7".to_string(), t("fkey_ed_search")),
            ("8".to_string(), t("fkey_ed_discard")),
            ("9".to_string(), String::new()),
            ("10".to_string(), t("fkey_ed_quit")),
            ("11".to_string(), String::new()),
            ("12".to_string(), String::new()),
        ]
    } else if is_viewer {
        vec![
            ("1".to_string(), t("fkey_help")),
            ("2".to_string(), String::new()),
            ("3".to_string(), String::new()),
            ("4".to_string(), t("fkey_vw_hex")),
            ("5".to_string(), String::new()),
            ("6".to_string(), String::new()),
            ("7".to_string(), t("fkey_vw_search")),
            ("8".to_string(), String::new()),
            ("9".to_string(), String::new()),
            ("10".to_string(), t("fkey_vw_quit")),
            ("11".to_string(), String::new()),
            ("12".to_string(), String::new()),
        ]
    } else if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        vec![
            ("1".to_string(), t("fkey_ctrl_left")),
            ("2".to_string(), t("fkey_ctrl_right")),
            ("3".to_string(), t("fkey_ctrl_name")),
            ("4".to_string(), t("fkey_ctrl_extens")),
            ("5".to_string(), t("fkey_ctrl_time")),
            ("6".to_string(), t("fkey_ctrl_size")),
            ("7".to_string(), t("fkey_ctrl_unsort")),
            ("8".to_string(), t("fkey_ctrl_creatn")),
            ("9".to_string(), t("fkey_ctrl_access")),
            ("10".to_string(), t("fkey_ctrl_descr")),
            ("11".to_string(), t("fkey_ctrl_owner")),
            ("12".to_string(), t("fkey_ctrl_sort")),
        ]
    } else if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        vec![
            ("1".to_string(), t("fkey_alt_left")),
            ("2".to_string(), t("fkey_alt_right")),
            ("3".to_string(), t("fkey_alt_view")),
            ("4".to_string(), t("fkey_alt_edit")),
            ("5".to_string(), t("fkey_alt_print")),
            ("6".to_string(), t("fkey_alt_mklink")),
            ("7".to_string(), t("fkey_alt_find")),
            ("8".to_string(), t("fkey_alt_history")),
            ("9".to_string(), t("fkey_alt_video")),
            ("10".to_string(), t("fkey_alt_tree")),
            ("11".to_string(), t("fkey_alt_viewhs")),
            ("12".to_string(), t("fkey_alt_foldhs")),
        ]
    } else if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
        let is_dev_plugin_dir = context.config.settings.plugins_developer_mode && {
            let active_panel = state.get_active_panel();
            let current_dir = &active_panel.current_path;
            current_dir.join("manifest.toml").exists()
                || active_panel
                    .entries
                    .get(active_panel.cursor_index)
                    .map(|e| e.path.is_dir() && e.path.join("manifest.toml").exists())
                    .unwrap_or(false)
        };
        let mut fks: Vec<(String, String)> =
            (1..=12).map(|n| (n.to_string(), String::new())).collect();
        if is_dev_plugin_dir {
            fks[10] = ("11".to_string(), t("plugin_install_dev"));
        }
        fks
    } else {
        // Default F-row — resolve each F-key from the user's current keymap so the
        // bar always reflects what pressing the key will actually do.
        let defaults = [
            "fkey_help",   // F1
            "fkey_user",   // F2
            "fkey_view",   // F3
            "fkey_edit",   // F4
            "fkey_copy",   // F5
            "fkey_move",   // F6
            "fkey_rename", // F7
            "fkey_delete", // F8
            "fkey_menu",   // F9
            "fkey_quit",   // F10
            "",            // F11 (unbound by default — Plugin menu lives under F9 → Files)
            "fkey_screen", // F12
        ];
        defaults
            .iter()
            .enumerate()
            .map(|(i, fallback)| {
                let n = (i + 1).to_string();
                let label = slot_label(context, i, fallback);
                (n, label)
            })
            .collect()
    };

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
    if state.update_available.is_some() {
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
