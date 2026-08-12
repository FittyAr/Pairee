//! Apply scrollbar mouse drag/jump commands to application scroll state.

use crate::app::state::{AppState, PopupType, Screen};
use crate::ui::scrollbar::{self, ScrollTargetId};
use crossterm::event::MouseEvent;

/// Handle a mouse event against last-frame scrollbar hit targets.
/// Returns `true` if a scrollbar consumed the event.
pub fn handle_scrollbar_mouse(state: &mut AppState, mouse: MouseEvent) -> bool {
    let targets = state.scrollbar.targets_snapshot();
    let Some((id, offset)) =
        scrollbar::handle_mouse_on_targets(&targets, mouse, &mut state.scrollbar.interaction)
    else {
        return false;
    };

    // Viewport stored on the hit target (for list cursor clamping).
    let viewport = targets
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.viewport_len)
        .unwrap_or(1);
    let content_len = targets
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.content_len)
        .unwrap_or(offset.saturating_add(1));

    apply_scroll_offset(state, id, offset, viewport, content_len);
    true
}

fn apply_scroll_offset(
    state: &mut AppState,
    id: ScrollTargetId,
    offset: usize,
    viewport: usize,
    content_len: usize,
) {
    match id {
        ScrollTargetId::Viewer => {
            if let Some(Screen::Viewer(vw)) = state.screens.get_mut(state.active_screen_idx) {
                vw.scroll = offset;
            }
        }
        ScrollTargetId::HelpContent => {
            if let Some(PopupType::Help { scroll_y, .. }) = &mut state.active_popup {
                *scroll_y = offset;
            }
        }
        ScrollTargetId::About => {
            if let Some(PopupType::About { scroll_y }) = &mut state.active_popup {
                *scroll_y = offset;
            }
        }
        ScrollTargetId::UpdateNotes => {
            if let Some(PopupType::UpdateAvailable { scroll_y, .. }) = &mut state.active_popup {
                *scroll_y = offset;
            }
        }
        ScrollTargetId::QuickView => {
            if let Some(PopupType::QuickViewPanel { scroll, .. }) = &mut state.active_popup {
                *scroll = offset;
            }
        }
        ScrollTargetId::PanelLeft => {
            scrollbar::clamp_cursor_to_offset(
                &mut state.left_panel.cursor_index,
                offset,
                viewport,
                content_len.min(state.left_panel.entries.len()),
            );
        }
        ScrollTargetId::PanelRight => {
            scrollbar::clamp_cursor_to_offset(
                &mut state.right_panel.cursor_index,
                offset,
                viewport,
                content_len.min(state.right_panel.entries.len()),
            );
        }
        ScrollTargetId::GitList => {
            if let Some(PopupType::GitPanel {
                cursor_idx, scroll, ..
            }) = &mut state.active_popup
            {
                *scroll = offset;
                scrollbar::clamp_cursor_to_offset(cursor_idx, offset, viewport, content_len);
            }
        }
        ScrollTargetId::HistoryCommand => {
            if let Some(PopupType::CommandHistoryList {
                cursor_idx,
                entries,
            }) = &mut state.active_popup
            {
                scrollbar::clamp_cursor_to_offset(
                    cursor_idx,
                    offset,
                    viewport,
                    content_len.min(entries.len()),
                );
            }
        }
        ScrollTargetId::HistoryView => {
            if let Some(PopupType::FileViewHistoryList {
                cursor_idx,
                entries,
            }) = &mut state.active_popup
            {
                scrollbar::clamp_cursor_to_offset(
                    cursor_idx,
                    offset,
                    viewport,
                    content_len.min(entries.len()),
                );
            }
        }
        ScrollTargetId::HistoryFolder => {
            if let Some(PopupType::FoldersHistoryList {
                cursor_idx,
                entries,
            }) = &mut state.active_popup
            {
                scrollbar::clamp_cursor_to_offset(
                    cursor_idx,
                    offset,
                    viewport,
                    content_len.min(entries.len()),
                );
            }
        }
        ScrollTargetId::TransferJobs => {
            if let Some(ts) = state.transfer.as_mut() {
                scrollbar::clamp_cursor_to_offset(
                    &mut ts.queue_cursor,
                    offset,
                    viewport,
                    content_len,
                );
            }
        }
        ScrollTargetId::TransferFiles => {
            if let Some(ts) = state.transfer.as_mut() {
                scrollbar::clamp_cursor_to_offset(
                    &mut ts.file_list_cursor,
                    offset,
                    viewport,
                    content_len,
                );
            }
        }
        ScrollTargetId::TransferLog => {
            if let Some(ts) = state.transfer.as_mut() {
                ts.log_scroll = offset;
            }
        }
        ScrollTargetId::PluginSelect => {
            if let Some(PopupType::SelectDevPlugin {
                cursor_idx,
                options,
                ..
            }) = &mut state.active_popup
            {
                scrollbar::clamp_cursor_to_offset(
                    cursor_idx,
                    offset,
                    viewport,
                    content_len.min(options.len()),
                );
            }
        }
    }
}
