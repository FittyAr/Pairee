//! Menu, sorting, drive selection, and context action popup rendering.

pub mod bookmarks;
pub mod dialogs;
pub mod selectors;
pub mod top_bar;

pub use bookmarks::{render_folder_shortcuts, render_hotlist};
pub use dialogs::{render_sort_modes_dialog, render_user_menu_dialog};
pub use selectors::{render_archive_commands_menu, render_context_menu, render_drive_select};
pub use top_bar::render_top_dropdown;

use crate::app::state::popup::{
    ArchiveCommandsMenu, ContextMenu, DriveSelect, FolderShortcuts, Hotlist, Menu, SortModesDialog,
    UserMenu,
};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for SortModesDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render_sort_modes_dialog(
            f,
            cx.theme,
            cx.area,
            &self.current,
            self.reverse,
            self.cursor_idx,
        );
    }
}

impl DialogView for UserMenu {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let active = cx.state.panels.active;
        render_user_menu_dialog(
            f,
            cx.theme,
            cx.panels,
            active,
            (&self.items, self.cursor_idx),
        );
    }
}

impl DialogView for Menu {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let focus = top_bar::MenuFocus {
            menu: self.active_menu_idx,
            item: self.active_item_idx,
            submenu: self.active_submenu_idx,
            sub_item: self.active_submenu_item_idx,
        };
        render_top_dropdown(f, cx.theme, cx.area, (cx.state, cx.context), focus);
    }
}

impl DialogView for DriveSelect {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (panel, drives) = (&self.panel, &self.drives);
        render_drive_select(f, cx.theme, cx.panels, panel, drives, self.cursor_idx);
    }
}

impl DialogView for Hotlist {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render_hotlist(f, cx.theme, cx.area, &self.entries, self.cursor_idx);
    }
}

impl DialogView for FolderShortcuts {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (slots, resolver) = (&cx.state.folder_shortcuts, &cx.context.resolver);
        render_folder_shortcuts(f, cx.theme, cx.area, slots, resolver, self.cursor_idx);
    }
}

impl DialogView for ContextMenu {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let active = cx.state.panels.active;
        render_context_menu(f, cx.theme, cx.panels, active, &self.items, self.cursor_idx);
    }
}

impl DialogView for ArchiveCommandsMenu {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (path, items) = (&self.archive_path, &self.items);
        render_archive_commands_menu(f, cx.theme, cx.area, path, items, self.cursor_idx);
    }
}
