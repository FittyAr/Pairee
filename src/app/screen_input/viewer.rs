//! Viewer screen keys: global panel actions pass through, everything else
//! runs the command the `[viewer]` keymap section binds.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::screens::ViewerAction;
use crate::ui::viewer::{LineSelection, ViewerMode};
use crossterm::event::{KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Lines moved by PgUp / PgDn.
const PAGE: usize = 20;

pub fn handle_viewer_screen(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<(), ()> {
    if context.resolver.global_action(key).is_some() || state.active_viewer_mut().is_none() {
        return Err(());
    }
    if let Some(command) = context.resolver.viewer.dispatch(key) {
        run(state, command, context);
    }
    Ok(())
}

fn run(state: &mut AppState, command: ViewerAction, context: &AppContext) {
    use ViewerAction as V;
    if command == V::Copy {
        copy_selection(state);
        return;
    }
    let Some(vw) = state.active_viewer_mut() else {
        return;
    };
    let rows = vw.text_rows();
    // Text mode moves a cursor line (selections start from it); hex and
    // image modes only scroll.
    let text = vw.mode == ViewerMode::Text;
    let page = rows.saturating_sub(1).max(1) as isize;
    match command {
        V::LineUp if text => vw.move_cursor(-1, rows, false),
        V::LineDown if text => vw.move_cursor(1, rows, false),
        V::PageUp if text => vw.move_cursor(-page, rows, false),
        V::PageDown if text => vw.move_cursor(page, rows, false),
        V::Top if text => vw.cursor_to(0, rows, false),
        V::Bottom if text => vw.cursor_to(usize::MAX, rows, false),
        V::LineUp => vw.scroll_up(1),
        V::LineDown => vw.scroll_down(1),
        V::PageUp => vw.scroll_up(PAGE),
        V::PageDown => vw.scroll_down(PAGE),
        V::Top => vw.scroll = 0,
        V::Bottom => vw.scroll = vw.last_scroll(),
        V::ToggleHex => vw.toggle_mode(),
        V::Edit => {
            // Viewer → editor: always the built-in editor.
            let path = vw.path.clone();
            crate::app::actions::fs_ops::edit::edit_file(state, path, &context.config.settings);
        }
        V::Search => state
            .dialogs
            .replace(PopupType::ViewerSearchPrompt(Default::default())),
        V::Encoding => crate::app::input_popup::viewer::open_encoding_selector(state),
        V::SearchNext => state.repeat_viewer_search(),
        V::SelectUp | V::SelectDown | V::SelectPageUp | V::SelectPageDown | V::SelectAll
            if !text => {}
        V::SelectUp => vw.move_cursor(-1, rows, true),
        V::SelectDown => vw.move_cursor(1, rows, true),
        V::SelectPageUp => vw.move_cursor(-page, rows, true),
        V::SelectPageDown => vw.move_cursor(page, rows, true),
        V::SelectAll => vw.select_all(),
        V::Copy => {}
        // A running search stops first, then the selection goes; the next
        // Quit closes the viewer.
        V::Quit if state.cancel_viewer_search() => {}
        V::Quit if vw_has_selection(state) => {
            if let Some(vw) = state.active_viewer_mut() {
                vw.selection = None;
            }
        }
        V::Quit => state.close_current_screen(),
    }
}

fn vw_has_selection(state: &mut AppState) -> bool {
    state
        .active_viewer_mut()
        .is_some_and(|vw| vw.selection.is_some())
}

/// Copies the active viewer's selected lines (to the editor's clipboard,
/// which keeps them inside Pairee when there is no system clipboard) and
/// says so in the status line.
fn copy_selection(state: &mut AppState) {
    let Some(vw) = state.active_viewer_mut() else {
        return;
    };
    if vw.mode != ViewerMode::Text {
        vw.notice = Some(t("viewer_copy_nothing"));
        return;
    }
    let copied = vw.selected_text();
    let lines = copied.lines.to_string();
    let key = if copied.truncated {
        "viewer_copy_truncated"
    } else {
        "viewer_copied"
    };
    let system = state.editor_clipboard.copy(copied.text);
    let key = if system {
        key
    } else {
        "viewer_copied_internal"
    };
    if let Some(vw) = state.active_viewer_mut() {
        vw.notice = Some(t(key).replacen("{}", &lines, 1));
    }
}

/// The mouse over the active viewer: a left press on the text and a drag
/// select whole lines (scrolling when the pointer leaves the text). `true`
/// when it did something.
pub fn handle_viewer_mouse(state: &mut AppState, mouse: MouseEvent) -> bool {
    if state.dialogs.is_open() {
        return false;
    }
    let Some(vw) = state.active_viewer_mut() else {
        return false;
    };
    if vw.mode != ViewerMode::Text {
        return false;
    }
    let layout = vw.layout.borrow().clone();
    let rows = vw.text_rows();
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if layout.contains(mouse.column, mouse.row) => {
            // A click puts the cursor there; Shift+click selects up to it.
            let Some(line) = layout.line_at(mouse.row) else {
                return false;
            };
            let extend = mouse.modifiers.contains(KeyModifiers::SHIFT);
            vw.cursor_to(line, rows, extend);
            vw.drag_anchor = Some(line);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let (Some(anchor), Some(line)) = (vw.drag_anchor, layout.line_at(mouse.row)) else {
                return false;
            };
            vw.selection.get_or_insert(LineSelection {
                anchor,
                end: anchor,
            });
            vw.cursor_to(line, rows, true);
        }
        MouseEventKind::Up(MouseButton::Left) if vw.drag_anchor.is_some() => {
            vw.drag_anchor = None;
        }
        _ => return false,
    }
    true
}
