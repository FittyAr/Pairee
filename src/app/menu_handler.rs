use super::sys_helpers::get_system_drives;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;

/// Skip separator lines (those starting with " ─") when mapping item_idx to action.
/// The index is the raw cursor position in the menu; separator lines are not actionable.
pub fn trigger_menu_item(
    state: &mut AppState,
    _context: &mut AppContext,
    menu_idx: usize,
    item_idx: usize,
) -> Option<Action> {
    match menu_idx {
        0 | 4 => {
            // Left / Right panel menu — both have the same layout
            let is_right = menu_idx == 4;
            match item_idx {
                0 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::Brief;
                    None
                }
                1 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::Medium;
                    None
                }
                2 => {
                    state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Full;
                    None
                }
                3 => {
                    state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Wide;
                    None
                }
                4 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::Detailed;
                    None
                }
                5 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::Descriptions;
                    None
                }
                6 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::FileOwners;
                    None
                }
                7 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::FileLinks;
                    None
                }
                8 => {
                    state.get_active_panel_mut().view_mode =
                        crate::app::state::PanelViewMode::AltFull;
                    None
                }
                // 9 = separator
                10 => Some(Action::InfoPanel),
                11 => Some(Action::QuickView),
                // 12 = separator
                13 => Some(Action::SortModes),
                14 => Some(Action::ToggleLongNames),
                15 => {
                    if is_right {
                        Some(Action::TogglePanelRight)
                    } else {
                        Some(Action::TogglePanelLeft)
                    }
                }
                16 => Some(Action::Refresh),
                17 => {
                    if is_right {
                        Some(Action::DriveSelectRight)
                    } else {
                        Some(Action::DriveSelectLeft)
                    }
                }
                18 => {
                    state.active_panel = if is_right {
                        crate::app::state::ActivePanel::Right
                    } else {
                        crate::app::state::ActivePanel::Left
                    };
                    Some(Action::SshConnect)
                }
                19 => {
                    state.active_panel = if is_right {
                        crate::app::state::ActivePanel::Right
                    } else {
                        crate::app::state::ActivePanel::Left
                    };
                    Some(Action::SshDisconnect)
                }
                _ => None,
            }
        }
        1 => {
            // Files menu
            match item_idx {
                0 => Some(Action::View),
                1 => Some(Action::Edit),
                2 => Some(Action::Copy),
                3 => Some(Action::Move),
                4 => Some(Action::CreateLink),
                5 => Some(Action::MkDir),
                6 => Some(Action::Delete),
                7 => Some(Action::WipeFile),
                // 8 = separator
                9 => Some(Action::CompressFiles),
                10 => Some(Action::ExtractArchive),
                11 => Some(Action::ArchiveCommands),
                // 12 = separator
                13 => Some(Action::FileAttributes),
                14 => Some(Action::ApplyCommand),
                15 => Some(Action::DescribeFile),
                // 16 = separator
                17 => Some(Action::SelectGroup),
                18 => Some(Action::UnselectGroup),
                19 => Some(Action::InvertSelection),
                20 => Some(Action::RestoreSelection),
                _ => None,
            }
        }
        2 => {
            // Commands menu
            match item_idx {
                0 => Some(Action::FindFile),
                1 => Some(Action::CommandHistory),
                2 => Some(Action::FileViewHistory),
                3 => Some(Action::FoldersHistory),
                // 4 = separator
                5 => Some(Action::SwapPanels),
                6 => Some(Action::ToggleBothPanels),
                7 => Some(Action::CompareFolder),
                // 8 = separator
                9 => Some(Action::EditUserMenu),
                10 => Some(Action::FileAssociations),
                11 => Some(Action::FolderShortcutsConfig),
                12 => Some(Action::FilePanelFilter),
                // 13 = separator
                14 => Some(Action::PluginMenu),
                15 => Some(Action::ScreensList),
                16 => Some(Action::TaskList),
                17 => {
                    // Hotplug devices — reuse DriveSelect
                    let drives = get_system_drives();
                    state.active_popup = Some(PopupType::DriveSelect {
                        panel: state.active_panel,
                        drives,
                        cursor_idx: 0,
                    });
                    None
                }
                _ => None,
            }
        }
        3 => {
            // Options menu
            match item_idx {
                0 => Some(Action::SystemSettings),
                2 => Some(Action::SaveSetup),
                _ => None,
            }
        }
        _ => None,
    }
}
