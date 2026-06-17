use crate::app::context::AppContext;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

// ─────────────────────────────────────────────────────────────────────────────
// Menu definitions
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MenuItemData {
    pub label: String,
    pub shortcut: String,
    pub active: bool,
    pub is_separator: bool,
}

impl MenuItemData {
    pub fn new(label: String, shortcut: &str, active: bool) -> Self {
        Self {
            label,
            shortcut: shortcut.to_string(),
            active,
            is_separator: false,
        }
    }
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: String::new(),
            active: false,
            is_separator: true,
        }
    }
}

/// Returns the menu item labels for a given top-level menu index.
pub fn get_menu_items(menu_idx: usize, state: &AppState) -> Vec<MenuItemData> {
    match menu_idx {
        0 => vec![
            MenuItemData::new(
                t("menu_brief"),
                "Ctrl+1",
                state.left_panel.view_mode == PanelViewMode::Brief,
            ),
            MenuItemData::new(
                t("menu_medium"),
                "Ctrl+2",
                state.left_panel.view_mode == PanelViewMode::Medium,
            ),
            MenuItemData::new(
                t("menu_full"),
                "Ctrl+3",
                state.left_panel.view_mode == PanelViewMode::Full,
            ),
            MenuItemData::new(
                t("menu_wide"),
                "Ctrl+4",
                state.left_panel.view_mode == PanelViewMode::Wide,
            ),
            MenuItemData::new(
                t("menu_detailed"),
                "Ctrl+5",
                state.left_panel.view_mode == PanelViewMode::Detailed,
            ),
            MenuItemData::new(
                t("menu_descriptions"),
                "Ctrl+6",
                state.left_panel.view_mode == PanelViewMode::Descriptions,
            ),
            MenuItemData::new(
                t("menu_file_owners"),
                "Ctrl+7",
                state.left_panel.view_mode == PanelViewMode::FileOwners,
            ),
            MenuItemData::new(
                t("menu_file_links"),
                "Ctrl+8",
                state.left_panel.view_mode == PanelViewMode::FileLinks,
            ),
            MenuItemData::new(
                t("menu_alt_full"),
                "Ctrl+9",
                state.left_panel.view_mode == PanelViewMode::AltFull,
            ),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_info_panel"),
                "Ctrl+L",
                matches!(state.active_popup, Some(PopupType::InfoPanel { .. })),
            ),
            MenuItemData::new(t("menu_quick_view"), "Ctrl+Q", state.quick_view_active),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_sort_modes"), "Ctrl+F12", false),
            MenuItemData::new(
                t("menu_show_long_names"),
                "Ctrl+N",
                state.left_panel.show_long_names,
            ),
            MenuItemData::new(t("menu_panel_on_off"), "Ctrl+F1", state.left_panel_visible),
            MenuItemData::new(t("menu_re_read"), "Ctrl+R", false),
            MenuItemData::new(t("menu_change_drive"), "Alt+F1", false),
            MenuItemData::new(t("menu_connect_ssh"), "Ctrl+Shift+S", false),
        ],
        1 => vec![
            MenuItemData::new(t("menu_view"), "F3", false),
            MenuItemData::new(t("menu_edit"), "F4", false),
            MenuItemData::new(t("menu_copy"), "F5", false),
            MenuItemData::new(t("menu_rename_move"), "F6", false),
            MenuItemData::new(t("menu_link"), "Alt+F6", false),
            MenuItemData::new(t("menu_make_folder"), "F7", false),
            MenuItemData::new(t("menu_delete"), "F8", false),
            MenuItemData::new(t("menu_wipe"), "Alt+Del", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_add_to_archive"), "Shf+F1", false),
            MenuItemData::new(t("menu_extract_files"), "Shf+F2", false),
            MenuItemData::new(t("menu_archive_commands"), "Shf+F3", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_file_attributes"), "Ctrl+A", false),
            MenuItemData::new(t("menu_apply_command"), "Ctrl+G", false),
            MenuItemData::new(t("menu_describe_files"), "Ctrl+Z", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_select_group"), "Gray+", false),
            MenuItemData::new(t("menu_unselect_group"), "Gray-", false),
            MenuItemData::new(t("menu_invert_selection"), "Gray*", false),
            MenuItemData::new(t("menu_restore_selection"), "Ctrl+M", false),
        ],
        2 => vec![
            MenuItemData::new(t("menu_find_file"), "Alt+F7", false),
            MenuItemData::new(t("menu_history"), "Alt+F8", false),
            MenuItemData::new(t("menu_file_view_hist"), "Alt+F11", false),
            MenuItemData::new(t("menu_folders_hist"), "Alt+F12", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_swap_panels"), "Ctrl+U", false),
            MenuItemData::new(t("menu_panels_on_off"), "Ctrl+O", false),
            MenuItemData::new(t("menu_compare_folders"), "", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_edit_user_menu"), "", false),
            MenuItemData::new(t("menu_file_associations"), "", false),
            MenuItemData::new(t("menu_folder_shortcuts"), "", false),
            MenuItemData::new(t("menu_file_panel_filter"), "Ctrl+I", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_plugin_commands"), "F11", false),
            MenuItemData::new(t("menu_screens_list"), "F12", false),
            MenuItemData::new(t("menu_task_list"), "Ctrl+W", false),
            MenuItemData::new(t("menu_hotplug_devices"), "", false),
        ],
        3 => vec![
            MenuItemData::new(t("menu_configuration"), "", false),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_save_setup"), "Shf+F9", false),
        ],
        4 => vec![
            MenuItemData::new(
                t("menu_brief"),
                "Ctrl+1",
                state.right_panel.view_mode == PanelViewMode::Brief,
            ),
            MenuItemData::new(
                t("menu_medium"),
                "Ctrl+2",
                state.right_panel.view_mode == PanelViewMode::Medium,
            ),
            MenuItemData::new(
                t("menu_full"),
                "Ctrl+3",
                state.right_panel.view_mode == PanelViewMode::Full,
            ),
            MenuItemData::new(
                t("menu_wide"),
                "Ctrl+4",
                state.right_panel.view_mode == PanelViewMode::Wide,
            ),
            MenuItemData::new(
                t("menu_detailed"),
                "Ctrl+5",
                state.right_panel.view_mode == PanelViewMode::Detailed,
            ),
            MenuItemData::new(
                t("menu_descriptions"),
                "Ctrl+6",
                state.right_panel.view_mode == PanelViewMode::Descriptions,
            ),
            MenuItemData::new(
                t("menu_file_owners"),
                "Ctrl+7",
                state.right_panel.view_mode == PanelViewMode::FileOwners,
            ),
            MenuItemData::new(
                t("menu_file_links"),
                "Ctrl+8",
                state.right_panel.view_mode == PanelViewMode::FileLinks,
            ),
            MenuItemData::new(
                t("menu_alt_full"),
                "Ctrl+9",
                state.right_panel.view_mode == PanelViewMode::AltFull,
            ),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_info_panel"),
                "Ctrl+L",
                matches!(state.active_popup, Some(PopupType::InfoPanel { .. })),
            ),
            MenuItemData::new(t("menu_quick_view"), "Ctrl+Q", state.quick_view_active),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_sort_modes"), "Ctrl+F12", false),
            MenuItemData::new(
                t("menu_show_long_names"),
                "Ctrl+N",
                state.right_panel.show_long_names,
            ),
            MenuItemData::new(t("menu_panel_on_off"), "Ctrl+F2", state.right_panel_visible),
            MenuItemData::new(t("menu_re_read"), "Ctrl+R", false),
            MenuItemData::new(t("menu_change_drive"), "Alt+F2", false),
            MenuItemData::new(t("menu_connect_ssh"), "Ctrl+Shift+S", false),
        ],
        _ => vec![],
    }
}

/// Returns the display labels for the top-level menu bar.
pub fn get_menu_titles() -> Vec<String> {
    vec![
        format!("  {}  ", t("menu_left")),
        format!("  {}  ", t("menu_files")),
        format!("  {}  ", t("menu_commands")),
        format!("  {}  ", t("menu_options")),
        format!("  {}  ", t("menu_right")),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Renderer
// ─────────────────────────────────────────────────────────────────────────────

pub fn render_menu(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;

    let active_menu_idx = if let Some(PopupType::Menu {
        active_menu_idx, ..
    }) = state.active_popup
    {
        Some(active_menu_idx)
    } else {
        None
    };

    let mut spans = Vec::new();
    let titles = get_menu_titles();
    for (i, title) in titles.iter().enumerate() {
        let is_active = Some(i) == active_menu_idx;
        let style = if is_active {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(parse_color(&theme.panel_fg))
                .add_modifier(Modifier::BOLD)
        };

        let hotkey_style = style.fg(ratatui::style::Color::Yellow);
        let hotkey_spans = crate::ui::hotkey::render_hotkey_spans(title, style, hotkey_style);
        spans.extend(hotkey_spans);
    }

    let menu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(12)])
        .split(area);

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(parse_color("DarkGray")));
    f.render_widget(paragraph, menu_chunks[0]);

    if context.config.settings.interface_clock {
        let time_str = chrono::Local::now().format(" %H:%M:%S ").to_string();
        let clock_para = Paragraph::new(time_str)
            .style(
                Style::default()
                    .bg(parse_color("DarkGray"))
                    .fg(parse_color(&theme.panel_fg)),
            )
            .alignment(ratatui::layout::Alignment::Right);
        f.render_widget(clock_para, menu_chunks[1]);
    } else {
        let empty_para = Paragraph::new("").style(Style::default().bg(parse_color("DarkGray")));
        f.render_widget(empty_para, menu_chunks[1]);
    }
}
