//! "Left" and "Right" menus: the same entries for either panel.

use super::types::{ActionRow, MenuBuilder, MenuItemData};
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::config::settings::Settings;
use crate::keybindings::{Action, KeybindingResolver};

/// Submenu indices (see `super::get_menu_items`) and the panel-specific
/// actions of each side.
struct Side {
    view_submenu: usize,
    sort_submenu: usize,
    toggle: ActionRow,
    drive: ActionRow,
}

fn side(panel: ActivePanel) -> Side {
    match panel {
        ActivePanel::Left => Side {
            view_submenu: 5,
            sort_submenu: 6,
            toggle: ("menu_panel_on_off", Action::TogglePanelLeft, "Ctrl+F1"),
            drive: ("menu_change_drive", Action::DriveSelectLeft, "Alt+F1"),
        },
        ActivePanel::Right => Side {
            view_submenu: 7,
            sort_submenu: 8,
            toggle: ("menu_panel_on_off", Action::TogglePanelRight, "Ctrl+F2"),
            drive: ("menu_change_drive", Action::DriveSelectRight, "Alt+F2"),
        },
    }
}

pub fn get_items(
    state: &AppState,
    resolver: &KeybindingResolver,
    settings: &Settings,
    panel: ActivePanel,
) -> Vec<MenuItemData> {
    let side = side(panel);
    let p = state.panels.side(panel);
    let visible = match panel {
        ActivePanel::Left => state.panels.left_visible,
        ActivePanel::Right => state.panels.right_visible,
    };
    // Repo detection is cached by the background listing (no I/O while drawing).
    let git_row = if p.git_branch.is_some() {
        ("menu_git", Action::OpenGitPanel, "Alt+G")
    } else {
        ("menu_git_init", Action::GitInit, "")
    };
    let menu = MenuBuilder::new(resolver)
        .submenu("menu_view_mode", side.view_submenu)
        .separator()
        .toggle(
            ("menu_info_panel", Action::InfoPanel, "Ctrl+L"),
            matches!(state.dialogs.top(), Some(PopupType::InfoPanel { .. })),
        )
        .toggle(
            ("menu_quick_view", Action::QuickView, "Ctrl+Q"),
            state.panels.quick_view_active,
        )
        .separator()
        .action(("menu_sort_modes", Action::SortModes, "Ctrl+F12"))
        .submenu("menu_sort_by", side.sort_submenu)
        .toggle(
            ("menu_show_long_names", Action::ToggleLongNames, "Ctrl+N"),
            p.show_long_names,
        )
        .toggle(side.toggle, visible)
        .action(("menu_re_read", Action::RereadPanel, "Ctrl+R"))
        .action(side.drive)
        .actions_if(
            settings.ssh_enabled,
            &[("menu_connect_ssh", Action::SshConnect, "Ctrl+Shift+S")],
        )
        .actions_if(
            p.source.ssh().is_some(),
            &[("menu_disconnect_ssh", Action::SshDisconnect, "")],
        );
    if !settings.git_enabled {
        return menu.build();
    }
    menu.separator()
        .action(git_row)
        .action(("menu_git_clone", Action::GitClone, ""))
        .build()
}
