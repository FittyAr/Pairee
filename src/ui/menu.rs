use crate::app::context::AppContext;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
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
    pub action: Option<Action>,
}

impl MenuItemData {
    pub fn new(label: String, shortcut: &str, active: bool) -> Self {
        Self {
            label,
            shortcut: shortcut.to_string(),
            active,
            is_separator: false,
            action: None,
        }
    }
    pub fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: String::new(),
            active: false,
            is_separator: true,
            action: None,
        }
    }
}

/// Returns the menu item labels for a given top-level menu index.
pub fn get_menu_items(
    menu_idx: usize,
    state: &AppState,
    resolver: &crate::keybindings::KeybindingResolver,
    settings: &crate::config::settings::Settings,
) -> Vec<MenuItemData> {
    use crate::keybindings::Action;
    let shortcut_for = |action: Action, fallback: &str| -> String {
        resolver
            .key_for_action(action)
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };

    match menu_idx {
        0 => {
            let mut items = vec![
                MenuItemData::new(
                    t("menu_brief"),
                    &shortcut_for(Action::PanelViewBrief, "Ctrl+1"),
                    state.left_panel.view_mode == PanelViewMode::Brief,
                )
                .with_action(Action::PanelViewBrief),
                MenuItemData::new(
                    t("menu_medium"),
                    &shortcut_for(Action::PanelViewMedium, "Ctrl+2"),
                    state.left_panel.view_mode == PanelViewMode::Medium,
                )
                .with_action(Action::PanelViewMedium),
                MenuItemData::new(
                    t("menu_full"),
                    &shortcut_for(Action::PanelViewFull, "Ctrl+3"),
                    state.left_panel.view_mode == PanelViewMode::Full,
                )
                .with_action(Action::PanelViewFull),
                MenuItemData::new(
                    t("menu_wide"),
                    &shortcut_for(Action::PanelViewWide, "Ctrl+4"),
                    state.left_panel.view_mode == PanelViewMode::Wide,
                )
                .with_action(Action::PanelViewWide),
                MenuItemData::new(
                    t("menu_detailed"),
                    &shortcut_for(Action::PanelViewDetailed, "Ctrl+5"),
                    state.left_panel.view_mode == PanelViewMode::Detailed,
                )
                .with_action(Action::PanelViewDetailed),
                MenuItemData::new(
                    t("menu_descriptions"),
                    &shortcut_for(Action::PanelViewDescriptions, "Ctrl+6"),
                    state.left_panel.view_mode == PanelViewMode::Descriptions,
                )
                .with_action(Action::PanelViewDescriptions),
                MenuItemData::new(
                    t("menu_file_owners"),
                    &shortcut_for(Action::PanelViewFileOwners, "Ctrl+7"),
                    state.left_panel.view_mode == PanelViewMode::FileOwners,
                )
                .with_action(Action::PanelViewFileOwners),
                MenuItemData::new(
                    t("menu_file_links"),
                    &shortcut_for(Action::PanelViewFileLinks, "Ctrl+8"),
                    state.left_panel.view_mode == PanelViewMode::FileLinks,
                )
                .with_action(Action::PanelViewFileLinks),
                MenuItemData::new(
                    t("menu_alt_full"),
                    &shortcut_for(Action::PanelViewAltFull, "Ctrl+9"),
                    state.left_panel.view_mode == PanelViewMode::AltFull,
                )
                .with_action(Action::PanelViewAltFull),
                MenuItemData::separator(),
                MenuItemData::new(
                    t("menu_info_panel"),
                    &shortcut_for(Action::InfoPanel, "Ctrl+L"),
                    matches!(state.active_popup, Some(PopupType::InfoPanel { .. })),
                )
                .with_action(Action::InfoPanel),
                MenuItemData::new(
                    t("menu_quick_view"),
                    &shortcut_for(Action::QuickView, "Ctrl+Q"),
                    state.quick_view_active,
                )
                .with_action(Action::QuickView),
                MenuItemData::separator(),
                MenuItemData::new(
                    t("menu_sort_modes"),
                    &shortcut_for(Action::SortModes, "Ctrl+F12"),
                    false,
                )
                .with_action(Action::SortModes),
                MenuItemData::new(
                    t("menu_sort_name"),
                    &shortcut_for(Action::SortByName, "Ctrl+F3"),
                    false,
                )
                .with_action(Action::SortByName),
                MenuItemData::new(
                    t("menu_sort_ext"),
                    &shortcut_for(Action::SortByExtension, "Ctrl+F4"),
                    false,
                )
                .with_action(Action::SortByExtension),
                MenuItemData::new(
                    t("menu_sort_write"),
                    &shortcut_for(Action::SortByWriteTime, "Ctrl+F5"),
                    false,
                )
                .with_action(Action::SortByWriteTime),
                MenuItemData::new(
                    t("menu_sort_size"),
                    &shortcut_for(Action::SortBySize, "Ctrl+F6"),
                    false,
                )
                .with_action(Action::SortBySize),
                MenuItemData::new(
                    t("menu_sort_unsorted"),
                    &shortcut_for(Action::SortUnsorted, "Ctrl+F7"),
                    false,
                )
                .with_action(Action::SortUnsorted),
                MenuItemData::new(
                    t("menu_sort_create"),
                    &shortcut_for(Action::SortByCreationTime, "Ctrl+F8"),
                    false,
                )
                .with_action(Action::SortByCreationTime),
                MenuItemData::new(
                    t("menu_sort_access"),
                    &shortcut_for(Action::SortByAccessTime, "Ctrl+F9"),
                    false,
                )
                .with_action(Action::SortByAccessTime),
                MenuItemData::new(
                    t("menu_sort_desc"),
                    &shortcut_for(Action::SortByDescription, "Ctrl+F10"),
                    false,
                )
                .with_action(Action::SortByDescription),
                MenuItemData::new(
                    t("menu_sort_owner"),
                    &shortcut_for(Action::SortByOwner, "Ctrl+F11"),
                    false,
                )
                .with_action(Action::SortByOwner),
                MenuItemData::new(
                    t("menu_show_long_names"),
                    &shortcut_for(Action::ToggleLongNames, "Ctrl+N"),
                    state.left_panel.show_long_names,
                )
                .with_action(Action::ToggleLongNames),
                MenuItemData::new(
                    t("menu_panel_on_off"),
                    &shortcut_for(Action::TogglePanelLeft, "Ctrl+F1"),
                    state.left_panel_visible,
                )
                .with_action(Action::TogglePanelLeft),
                MenuItemData::new(
                    t("menu_re_read"),
                    &shortcut_for(Action::RereadPanel, "Ctrl+R"),
                    false,
                )
                .with_action(Action::RereadPanel),
                MenuItemData::new(
                    t("menu_change_drive"),
                    &shortcut_for(Action::DriveSelectLeft, "Alt+F1"),
                    false,
                )
                .with_action(Action::DriveSelectLeft),
                MenuItemData::new(
                    t("menu_connect_ssh"),
                    &shortcut_for(Action::SshConnect, "Ctrl+Shift+S"),
                    false,
                )
                .with_action(Action::SshConnect),
            ];
            if state.left_panel.ssh_conn.is_some() {
                items.push(
                    MenuItemData::new(
                        t("menu_disconnect_ssh"),
                        &shortcut_for(Action::SshDisconnect, ""),
                        false,
                    )
                    .with_action(Action::SshDisconnect),
                );
            }
            if settings.git_enabled {
                if crate::git::repo::find_repo(&state.left_panel.current_path).is_some() {
                    items.push(MenuItemData::separator());
                    items.push(
                        MenuItemData::new(
                            t("menu_git"),
                            &shortcut_for(Action::OpenGitPanel, "Alt+G"),
                            false,
                        )
                        .with_action(Action::OpenGitPanel),
                    );
                }
            }
            items
        }
        1 => vec![
            MenuItemData::new(t("menu_view"), &shortcut_for(Action::View, "F3"), false)
                .with_action(Action::View),
            MenuItemData::new(
                t("menu_view_alt"),
                &shortcut_for(Action::ViewAlt, "Alt+F3"),
                false,
            )
            .with_action(Action::ViewAlt),
            MenuItemData::new(t("menu_edit"), &shortcut_for(Action::Edit, "F4"), false)
                .with_action(Action::Edit),
            MenuItemData::new(t("menu_copy"), &shortcut_for(Action::Copy, "F5"), false)
                .with_action(Action::Copy),
            MenuItemData::new(
                t("menu_print"),
                &shortcut_for(Action::PrintFile, "Alt+F5"),
                false,
            )
            .with_action(Action::PrintFile),
            MenuItemData::new(
                t("menu_rename_move"),
                &shortcut_for(Action::Move, "F6"),
                false,
            )
            .with_action(Action::Move),
            MenuItemData::new(
                t("menu_link"),
                &shortcut_for(Action::CreateLink, "Alt+F6"),
                false,
            )
            .with_action(Action::CreateLink),
            MenuItemData::new(
                t("menu_make_folder"),
                &shortcut_for(Action::MkDir, "F7"),
                false,
            )
            .with_action(Action::MkDir),
            MenuItemData::new(t("menu_delete"), &shortcut_for(Action::Delete, "F8"), false)
                .with_action(Action::Delete),
            MenuItemData::new(
                t("menu_wipe"),
                &shortcut_for(Action::WipeFile, "Alt+Del"),
                false,
            )
            .with_action(Action::WipeFile),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_add_to_archive"),
                &shortcut_for(Action::CompressFiles, "Shf+F1"),
                false,
            )
            .with_action(Action::CompressFiles),
            MenuItemData::new(
                t("menu_extract_files"),
                &shortcut_for(Action::ExtractArchive, "Shf+F2"),
                false,
            )
            .with_action(Action::ExtractArchive),
            MenuItemData::new(
                t("menu_archive_commands"),
                &shortcut_for(Action::ArchiveCommands, "Shf+F3"),
                false,
            )
            .with_action(Action::ArchiveCommands),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_file_attributes"),
                &shortcut_for(Action::FileAttributes, "Ctrl+A"),
                false,
            )
            .with_action(Action::FileAttributes),
            MenuItemData::new(
                t("menu_apply_command"),
                &shortcut_for(Action::ApplyCommand, "Ctrl+G"),
                false,
            )
            .with_action(Action::ApplyCommand),
            MenuItemData::new(
                t("menu_describe_files"),
                &shortcut_for(Action::DescribeFile, "Ctrl+Z"),
                false,
            )
            .with_action(Action::DescribeFile),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_select_group"),
                &shortcut_for(Action::SelectGroup, "Gray+"),
                false,
            )
            .with_action(Action::SelectGroup),
            MenuItemData::new(
                t("menu_unselect_group"),
                &shortcut_for(Action::UnselectGroup, "Gray-"),
                false,
            )
            .with_action(Action::UnselectGroup),
            MenuItemData::new(
                t("menu_invert_selection"),
                &shortcut_for(Action::InvertSelection, "Gray*"),
                false,
            )
            .with_action(Action::InvertSelection),
            MenuItemData::new(
                t("menu_restore_selection"),
                &shortcut_for(Action::RestoreSelection, "Ctrl+M"),
                false,
            )
            .with_action(Action::RestoreSelection),
            MenuItemData::separator(),
            MenuItemData::new(t("menu_exit"), &shortcut_for(Action::Quit, "F10"), false)
                .with_action(Action::Quit),
        ],
        2 => vec![
            MenuItemData::new(
                t("menu_find_file"),
                &shortcut_for(Action::FindFile, "Alt+F7"),
                false,
            )
            .with_action(Action::FindFile),
            MenuItemData::new(
                t("menu_history"),
                &shortcut_for(Action::CommandHistory, "Alt+F8"),
                false,
            )
            .with_action(Action::CommandHistory),
            MenuItemData::new(
                t("menu_video_mode"),
                &shortcut_for(Action::VideoMode, "Alt+F9"),
                false,
            )
            .with_action(Action::VideoMode),
            MenuItemData::new(
                t("menu_tree_view"),
                &shortcut_for(Action::TreeView, "Alt+F10"),
                false,
            )
            .with_action(Action::TreeView),
            MenuItemData::new(
                t("menu_file_view_hist"),
                &shortcut_for(Action::FileViewHistory, "Alt+F11"),
                false,
            )
            .with_action(Action::FileViewHistory),
            MenuItemData::new(
                t("menu_folders_hist"),
                &shortcut_for(Action::FoldersHistory, "Alt+F12"),
                false,
            )
            .with_action(Action::FoldersHistory),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_swap_panels"),
                &shortcut_for(Action::SwapPanels, "Ctrl+U"),
                false,
            )
            .with_action(Action::SwapPanels),
            MenuItemData::new(
                t("menu_panels_on_off"),
                &shortcut_for(Action::ToggleBothPanels, "Ctrl+O"),
                false,
            )
            .with_action(Action::ToggleBothPanels),
            MenuItemData::new(
                t("menu_compare_folders"),
                &shortcut_for(Action::CompareFolder, ""),
                false,
            )
            .with_action(Action::CompareFolder),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_user_menu"),
                &shortcut_for(Action::UserMenu, "F2"),
                false,
            )
            .with_action(Action::UserMenu),
            MenuItemData::new(
                t("menu_edit_user_menu"),
                &shortcut_for(Action::EditUserMenu, ""),
                false,
            )
            .with_action(Action::EditUserMenu),
            MenuItemData::new(
                t("menu_file_associations"),
                &shortcut_for(Action::FileAssociations, ""),
                false,
            )
            .with_action(Action::FileAssociations),
            MenuItemData::new(
                t("menu_folder_shortcuts"),
                &shortcut_for(Action::FolderShortcutsConfig, ""),
                false,
            )
            .with_action(Action::FolderShortcutsConfig),
            MenuItemData::new(
                t("menu_file_panel_filter"),
                &shortcut_for(Action::FilePanelFilter, "Ctrl+I"),
                false,
            )
            .with_action(Action::FilePanelFilter),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_plugin_commands"),
                &shortcut_for(Action::PluginMenu, "F11"),
                false,
            )
            .with_action(Action::PluginMenu),
            MenuItemData::new(
                t("menu_screens_list"),
                &shortcut_for(Action::ScreensList, "F12"),
                false,
            )
            .with_action(Action::ScreensList),
            MenuItemData::new(
                t("menu_task_list"),
                &shortcut_for(Action::TaskList, "Ctrl+W"),
                false,
            )
            .with_action(Action::TaskList),
            MenuItemData::new(t("menu_hotplug_devices"), "", false),
        ],
        3 => vec![
            MenuItemData::new(t("menu_help"), &shortcut_for(Action::Help, "F1"), false)
                .with_action(Action::Help),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_configuration"),
                &shortcut_for(Action::SystemSettings, ""),
                false,
            )
            .with_action(Action::SystemSettings),
            MenuItemData::separator(),
            MenuItemData::new(
                t("menu_save_setup"),
                &shortcut_for(Action::SaveSetup, "Shf+F9"),
                false,
            )
            .with_action(Action::SaveSetup),
        ],
        4 => {
            let mut items = vec![
                MenuItemData::new(
                    t("menu_brief"),
                    &shortcut_for(Action::PanelViewBrief, "Ctrl+1"),
                    state.right_panel.view_mode == PanelViewMode::Brief,
                )
                .with_action(Action::PanelViewBrief),
                MenuItemData::new(
                    t("menu_medium"),
                    &shortcut_for(Action::PanelViewMedium, "Ctrl+2"),
                    state.right_panel.view_mode == PanelViewMode::Medium,
                )
                .with_action(Action::PanelViewMedium),
                MenuItemData::new(
                    t("menu_full"),
                    &shortcut_for(Action::PanelViewFull, "Ctrl+3"),
                    state.right_panel.view_mode == PanelViewMode::Full,
                )
                .with_action(Action::PanelViewFull),
                MenuItemData::new(
                    t("menu_wide"),
                    &shortcut_for(Action::PanelViewWide, "Ctrl+4"),
                    state.right_panel.view_mode == PanelViewMode::Wide,
                )
                .with_action(Action::PanelViewWide),
                MenuItemData::new(
                    t("menu_detailed"),
                    &shortcut_for(Action::PanelViewDetailed, "Ctrl+5"),
                    state.right_panel.view_mode == PanelViewMode::Detailed,
                )
                .with_action(Action::PanelViewDetailed),
                MenuItemData::new(
                    t("menu_descriptions"),
                    &shortcut_for(Action::PanelViewDescriptions, "Ctrl+6"),
                    state.right_panel.view_mode == PanelViewMode::Descriptions,
                )
                .with_action(Action::PanelViewDescriptions),
                MenuItemData::new(
                    t("menu_file_owners"),
                    &shortcut_for(Action::PanelViewFileOwners, "Ctrl+7"),
                    state.right_panel.view_mode == PanelViewMode::FileOwners,
                )
                .with_action(Action::PanelViewFileOwners),
                MenuItemData::new(
                    t("menu_file_links"),
                    &shortcut_for(Action::PanelViewFileLinks, "Ctrl+8"),
                    state.right_panel.view_mode == PanelViewMode::FileLinks,
                )
                .with_action(Action::PanelViewFileLinks),
                MenuItemData::new(
                    t("menu_alt_full"),
                    &shortcut_for(Action::PanelViewAltFull, "Ctrl+9"),
                    state.right_panel.view_mode == PanelViewMode::AltFull,
                )
                .with_action(Action::PanelViewAltFull),
                MenuItemData::separator(),
                MenuItemData::new(
                    t("menu_info_panel"),
                    &shortcut_for(Action::InfoPanel, "Ctrl+L"),
                    matches!(state.active_popup, Some(PopupType::InfoPanel { .. })),
                )
                .with_action(Action::InfoPanel),
                MenuItemData::new(
                    t("menu_quick_view"),
                    &shortcut_for(Action::QuickView, "Ctrl+Q"),
                    state.quick_view_active,
                )
                .with_action(Action::QuickView),
                MenuItemData::separator(),
                MenuItemData::new(
                    t("menu_sort_modes"),
                    &shortcut_for(Action::SortModes, "Ctrl+F12"),
                    false,
                )
                .with_action(Action::SortModes),
                MenuItemData::new(
                    t("menu_sort_name"),
                    &shortcut_for(Action::SortByName, "Ctrl+F3"),
                    false,
                )
                .with_action(Action::SortByName),
                MenuItemData::new(
                    t("menu_sort_ext"),
                    &shortcut_for(Action::SortByExtension, "Ctrl+F4"),
                    false,
                )
                .with_action(Action::SortByExtension),
                MenuItemData::new(
                    t("menu_sort_write"),
                    &shortcut_for(Action::SortByWriteTime, "Ctrl+F5"),
                    false,
                )
                .with_action(Action::SortByWriteTime),
                MenuItemData::new(
                    t("menu_sort_size"),
                    &shortcut_for(Action::SortBySize, "Ctrl+F6"),
                    false,
                )
                .with_action(Action::SortBySize),
                MenuItemData::new(
                    t("menu_sort_unsorted"),
                    &shortcut_for(Action::SortUnsorted, "Ctrl+F7"),
                    false,
                )
                .with_action(Action::SortUnsorted),
                MenuItemData::new(
                    t("menu_sort_create"),
                    &shortcut_for(Action::SortByCreationTime, "Ctrl+F8"),
                    false,
                )
                .with_action(Action::SortByCreationTime),
                MenuItemData::new(
                    t("menu_sort_access"),
                    &shortcut_for(Action::SortByAccessTime, "Ctrl+F9"),
                    false,
                )
                .with_action(Action::SortByAccessTime),
                MenuItemData::new(
                    t("menu_sort_desc"),
                    &shortcut_for(Action::SortByDescription, "Ctrl+F10"),
                    false,
                )
                .with_action(Action::SortByDescription),
                MenuItemData::new(
                    t("menu_sort_owner"),
                    &shortcut_for(Action::SortByOwner, "Ctrl+F11"),
                    false,
                )
                .with_action(Action::SortByOwner),
                MenuItemData::new(
                    t("menu_show_long_names"),
                    &shortcut_for(Action::ToggleLongNames, "Ctrl+N"),
                    state.right_panel.show_long_names,
                )
                .with_action(Action::ToggleLongNames),
                MenuItemData::new(
                    t("menu_panel_on_off"),
                    &shortcut_for(Action::TogglePanelRight, "Ctrl+F2"),
                    state.right_panel_visible,
                )
                .with_action(Action::TogglePanelRight),
                MenuItemData::new(
                    t("menu_re_read"),
                    &shortcut_for(Action::RereadPanel, "Ctrl+R"),
                    false,
                )
                .with_action(Action::RereadPanel),
                MenuItemData::new(
                    t("menu_change_drive"),
                    &shortcut_for(Action::DriveSelectRight, "Alt+F2"),
                    false,
                )
                .with_action(Action::DriveSelectRight),
                MenuItemData::new(
                    t("menu_connect_ssh"),
                    &shortcut_for(Action::SshConnect, "Ctrl+Shift+S"),
                    false,
                )
                .with_action(Action::SshConnect),
            ];
            if state.right_panel.ssh_conn.is_some() {
                items.push(
                    MenuItemData::new(
                        t("menu_disconnect_ssh"),
                        &shortcut_for(Action::SshDisconnect, ""),
                        false,
                    )
                    .with_action(Action::SshDisconnect),
                );
            }
            if settings.git_enabled {
                if crate::git::repo::find_repo(&state.right_panel.current_path).is_some() {
                    items.push(MenuItemData::separator());
                    items.push(
                        MenuItemData::new(
                            t("menu_git"),
                            &shortcut_for(Action::OpenGitPanel, "Alt+G"),
                            false,
                        )
                        .with_action(Action::OpenGitPanel),
                    );
                }
            }
            items
        }
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
