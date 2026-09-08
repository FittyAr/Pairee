//! Menu, sorting, drive selection, and context action popup rendering.

pub mod dialogs;
pub mod selectors;
pub mod top_bar;

pub use dialogs::{render_sort_modes_dialog, render_user_menu_dialog};
pub use selectors::{
    render_archive_commands_menu, render_context_menu, render_drive_select, render_hotlist,
};
pub use top_bar::render_top_dropdown;

use crate::app::state::PopupType;
use ratatui::{Frame, layout::Rect};

pub fn render_menu_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    left_rect: Rect,
    right_rect: Rect,
    state: &crate::app::state::AppState,
    context: &crate::app::context::AppContext,
) -> bool {
    match popup {
        PopupType::SortModesDialog {
            current,
            reverse,
            cursor_idx,
        } => {
            render_sort_modes_dialog(f, theme, size, current, *reverse, *cursor_idx);
            true
        }
        PopupType::UserMenu { cursor_idx } => {
            render_user_menu_dialog(
                f,
                theme,
                left_rect,
                right_rect,
                state.panels.active,
                *cursor_idx,
            );
            true
        }
        PopupType::Menu {
            active_menu_idx,
            active_item_idx,
            active_submenu_idx,
            active_submenu_item_idx,
        } => {
            render_top_dropdown(
                f,
                theme,
                size,
                state,
                context,
                *active_menu_idx,
                *active_item_idx,
                *active_submenu_idx,
                *active_submenu_item_idx,
            );
            true
        }
        PopupType::DriveSelect {
            panel,
            drives,
            cursor_idx,
        } => {
            render_drive_select(f, theme, left_rect, right_rect, panel, drives, *cursor_idx);
            true
        }
        PopupType::Hotlist {
            bookmarks,
            cursor_idx,
        } => {
            render_hotlist(f, theme, size, bookmarks, *cursor_idx);
            true
        }
        PopupType::ContextMenu { items, cursor_idx } => {
            render_context_menu(
                f,
                theme,
                left_rect,
                right_rect,
                state.panels.active,
                items,
                *cursor_idx,
            );
            true
        }
        PopupType::ArchiveCommandsMenu {
            archive_path,
            items,
            cursor_idx,
        } => {
            render_archive_commands_menu(f, theme, size, archive_path, items, *cursor_idx);
            true
        }
        _ => false,
    }
}
