//! Apply scrollbar mouse drag/jump/wheel commands to application scroll state.

use crate::app::state::{ActivePanel, AppState, PopupType, Screen};
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
    let target = targets.iter().find(|t| t.id == id);
    let viewport = target.map(|t| t.viewport_len).unwrap_or(1);
    let content_len = target
        .map(|t| t.content_len)
        .unwrap_or(offset.saturating_add(1));
    apply_scroll_offset(state, id, offset, viewport, content_len);
    true
}

/// Scroll offset of the popup text view scrolled by `id`, if on top.
fn popup_scroll(popup: &mut PopupType, id: ScrollTargetId) -> Option<&mut usize> {
    match (id, popup) {
        (ScrollTargetId::HelpContent, PopupType::Help { scroll_y, .. })
        | (ScrollTargetId::About, PopupType::About { scroll_y })
        | (ScrollTargetId::UpdateNotes, PopupType::UpdateAvailable { scroll_y, .. }) => {
            Some(scroll_y)
        }
        (ScrollTargetId::QuickView, PopupType::QuickViewPanel(qv)) => Some(&mut qv.scroll),
        (ScrollTargetId::MultiRenamePreview, PopupType::MultiRename(dialog)) => {
            Some(&mut dialog.scroll)
        }
        _ => None,
    }
}

/// Cursor and length of the popup list scrolled by `id`, if on top.
fn popup_list(popup: &mut PopupType, id: ScrollTargetId) -> Option<(&mut usize, usize)> {
    match (id, popup) {
        (
            ScrollTargetId::HistoryCommand,
            PopupType::CommandHistoryList {
                cursor_idx,
                entries,
            },
        ) => Some((cursor_idx, entries.len())),
        (
            ScrollTargetId::HistoryView,
            PopupType::FileViewHistoryList {
                cursor_idx,
                entries,
            },
        ) => Some((cursor_idx, entries.len())),
        (
            ScrollTargetId::HistoryFolder,
            PopupType::FoldersHistoryList {
                cursor_idx,
                entries,
            },
        ) => Some((cursor_idx, entries.len())),
        (
            ScrollTargetId::PluginSelect,
            PopupType::SelectDevPlugin {
                cursor_idx,
                options,
                ..
            },
        ) => Some((cursor_idx, options.len())),
        _ => None,
    }
}

fn apply_scroll_offset(
    state: &mut AppState,
    id: ScrollTargetId,
    offset: usize,
    viewport: usize,
    content_len: usize,
) {
    let clamp = |cursor: &mut usize, len: usize| {
        scrollbar::clamp_cursor_to_offset(cursor, offset, viewport, content_len.min(len));
    };
    if let Some(popup) = state.dialogs.top_mut() {
        if let Some(scroll) = popup_scroll(popup, id) {
            *scroll = offset;
            return;
        }
        if let Some((cursor, len)) = popup_list(popup, id) {
            clamp(cursor, len);
            return;
        }
        if let (ScrollTargetId::GitList, PopupType::GitPanel(panel)) = (id, popup) {
            panel.scroll = offset;
            clamp(&mut panel.cursor_idx, usize::MAX);
            return;
        }
    }
    match id {
        ScrollTargetId::Viewer => {
            if let Some(Screen::Viewer(vw)) = state.screens.get_mut(state.active_screen_idx) {
                vw.scroll = offset;
            }
        }
        ScrollTargetId::PanelLeft | ScrollTargetId::PanelRight => {
            let side = if id == ScrollTargetId::PanelLeft {
                ActivePanel::Left
            } else {
                ActivePanel::Right
            };
            let panel = state.panels.side_mut(side);
            let len = panel.entries.len();
            clamp(&mut panel.cursor_index, len);
        }
        ScrollTargetId::TransferJobs | ScrollTargetId::TransferFiles => {
            if let Some(ts) = state.transfer.as_mut() {
                let cursor = if id == ScrollTargetId::TransferJobs {
                    &mut ts.queue_cursor
                } else {
                    &mut ts.file_list_cursor
                };
                clamp(cursor, usize::MAX);
            }
        }
        ScrollTargetId::TransferLog => {
            if let Some(ts) = state.transfer.as_mut() {
                ts.log_scroll = offset;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::MultiRenameState;

    #[test]
    fn multi_rename_preview_follows_the_scrollbar() {
        let mut state = AppState::new(".".into(), ".".into());
        let dialog =
            MultiRenameState::new(Vec::new(), Vec::new(), crate::fs::vfs::PanelSource::Local);
        state
            .dialogs
            .replace(PopupType::MultiRename(Box::new(dialog)));
        apply_scroll_offset(&mut state, ScrollTargetId::MultiRenamePreview, 7, 5, 40);
        match state.dialogs.top() {
            Some(PopupType::MultiRename(dialog)) => assert_eq!(dialog.scroll, 7),
            other => panic!("expected the multi-rename dialog, got {other:?}"),
        }
    }
}
