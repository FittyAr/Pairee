//! Apply scrollbar mouse drag/jump/wheel commands to application scroll state.

use crate::app::state::popup::DialogScroll;
use crate::app::state::{ActivePanel, AppState, Screen};
use crate::ui::scrollbar::{self, ScrollTargetId};
use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::Position;

/// Handle a mouse event against last-frame scrollbar hit targets.
/// Returns `true` if a scrollbar consumed the event.
pub fn handle_scrollbar_mouse(state: &mut AppState, mouse: MouseEvent) -> bool {
    // Layers first (a dialog over a bar hides it), then the bar's own cells.
    let mut targets = state.hits.scrollbars_for(mouse);
    // The bar only checks the row of a vertical track: a press counts only
    // on the bar's own cells, and a drag or release goes to the bar that
    // was pressed (never to another one in the same rows).
    let pointer = Position::new(mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Down(_) => targets.retain(|t| t.area.contains(pointer)),
        MouseEventKind::Drag(_) | MouseEventKind::Up(_) => {
            let pressed = state.mouse.scrollbar_drag;
            targets.retain(|t| Some(t.id) == pressed);
        }
        _ => {}
    }
    let found =
        scrollbar::handle_mouse_on_targets(&targets, mouse, &mut state.scrollbar.interaction);
    match mouse.kind {
        MouseEventKind::Down(_) => state.mouse.scrollbar_drag = found.map(|(id, _)| id),
        MouseEventKind::Up(_) => state.mouse.scrollbar_drag = None,
        _ => {}
    }
    let Some((id, offset)) = found else {
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
        match popup.dialog_mut().scroll_mut(id) {
            Some(DialogScroll::Offset(scroll)) => {
                *scroll = offset;
                return;
            }
            Some(DialogScroll::Cursor(cursor, len)) => {
                clamp(cursor, len);
                return;
            }
            Some(DialogScroll::OffsetAndCursor {
                offset: view,
                cursor,
            }) => {
                *view = offset;
                clamp(cursor, usize::MAX);
                return;
            }
            None => {}
        }
    }
    match id {
        ScrollTargetId::Viewer => {
            if let Some(Screen::Viewer(vw)) = Some(state.screens.active_mut()) {
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
            panel.scroll_top.set(offset);
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
    use crate::app::state::PopupType;
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
