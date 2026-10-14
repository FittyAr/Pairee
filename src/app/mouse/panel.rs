//! Mouse on the file panels: rows, column titles, the folder title, the
//! empty area and the wheel. Wherever a key does the same thing, the click
//! dispatches that key's action.

use super::{Click, act};
use crate::app::actions::navigation::{set_range, set_selected};
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, SortField};
use crate::config::settings::RightClick;
use crate::keybindings::Action;
use crate::terminal::TerminalBackend;
use crate::ui::hit::{HitTarget, WHEEL_LINES};
use crossterm::event::{KeyModifiers, MouseButton};

/// A right-button drag over rows, tagging (or untagging) what it crosses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkDrag {
    side: ActivePanel,
    selected: bool,
    last: usize,
}

/// The panel a hit belongs to.
pub fn side_of(target: &HitTarget) -> Option<ActivePanel> {
    match *target {
        HitTarget::PanelBody(side)
        | HitTarget::PanelTitle(side)
        | HitTarget::PanelHeader { side, .. }
        | HitTarget::PanelRow { side, .. } => Some(side),
        _ => None,
    }
}

/// Acts on a press on a panel hit. Returns whether anything changed.
pub async fn click(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    target: HitTarget,
    click: Click,
) -> anyhow::Result<bool> {
    let Some(side) = side_of(&target) else {
        return Ok(false);
    };
    if click.button == MouseButton::Middle {
        return Ok(false);
    }
    focus(state, context, terminal_backend, side).await?;
    match target {
        HitTarget::PanelRow { index, .. } => {
            row(state, context, terminal_backend, index, click).await?
        }
        HitTarget::PanelHeader {
            sort: Some(field), ..
        } if click.button == MouseButton::Left => {
            let action = if state.get_active_panel().sort_field == field {
                Action::ToggleSortReverse
            } else {
                sort_action(field)
            };
            act(state, context, terminal_backend, action).await?;
        }
        HitTarget::PanelTitle(side) => {
            let action = match (click.button, side) {
                (MouseButton::Right, ActivePanel::Left) => Action::DriveSelectLeft,
                (MouseButton::Right, ActivePanel::Right) => Action::DriveSelectRight,
                _ => Action::FoldersHistory,
            };
            act(state, context, terminal_backend, action).await?;
        }
        _ => {}
    }
    Ok(true)
}

async fn row(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    index: usize,
    click: Click,
) -> anyhow::Result<()> {
    let anchor = state.get_active_panel().cursor_index;
    if index >= state.get_active_panel().entries.len() {
        return Ok(());
    }
    state.get_active_panel_mut().cursor_index = index;
    match click.button {
        MouseButton::Left if click.is_double() && click.modifiers.is_empty() => {
            act(state, context, terminal_backend, Action::Execute).await?;
        }
        MouseButton::Left if click.modifiers.contains(KeyModifiers::CONTROL) => {
            let selected = !state.get_active_panel().is_selected(index);
            state.snapshot_selection();
            set_selected(state, index, selected);
        }
        MouseButton::Left if click.modifiers.contains(KeyModifiers::SHIFT) => {
            state.snapshot_selection();
            set_range(state, anchor, index, true);
        }
        MouseButton::Left if click.modifiers.is_empty() => {
            // The press may become a drag of the files.
            let side = state.panels.active;
            super::file_drag::press(state, side, index, (click.column, click.row));
        }
        MouseButton::Right => match context.config.settings.mouse_right_click {
            RightClick::Mark => {
                let selected = !state.get_active_panel().is_selected(index);
                state.snapshot_selection();
                set_selected(state, index, selected);
                state.mouse.mark_drag = Some(MarkDrag {
                    side: state.panels.active,
                    selected,
                    last: index,
                });
            }
            RightClick::Menu => act(state, context, terminal_backend, Action::ContextMenu).await?,
        },
        _ => {}
    }
    Ok(())
}

/// Continues a right-button drag over `target`. Returns whether it applied.
pub fn drag(state: &mut AppState, target: Option<&HitTarget>) -> bool {
    let Some(drag) = state.mouse.mark_drag else {
        return false;
    };
    let Some(&HitTarget::PanelRow { side, index }) = target else {
        return true;
    };
    if side != drag.side || index == drag.last {
        return true;
    }
    set_range(state, drag.last, index, drag.selected);
    state.get_active_panel_mut().cursor_index = index;
    state.mouse.mark_drag = Some(MarkDrag {
        last: index,
        ..drag
    });
    true
}

/// Moves the cursor of the panel under the wheel, focused or not.
pub fn wheel(state: &mut AppState, side: ActivePanel, down: bool) -> bool {
    let panel = state.panels.side_mut(side);
    if down {
        panel.page_down(WHEEL_LINES);
    } else {
        panel.page_up(WHEEL_LINES);
    }
    true
}

async fn focus(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    side: ActivePanel,
) -> anyhow::Result<()> {
    if state.panels.active == side {
        return Ok(());
    }
    let action = match side {
        ActivePanel::Left => Action::FocusLeftPanel,
        ActivePanel::Right => Action::FocusRightPanel,
    };
    act(state, context, terminal_backend, action).await
}

fn sort_action(field: SortField) -> Action {
    match field {
        SortField::Name => Action::SortByName,
        SortField::Extension => Action::SortByExtension,
        SortField::Size => Action::SortBySize,
        SortField::Date => Action::SortByWriteTime,
        SortField::Unsorted => Action::SortUnsorted,
    }
}

/// A drag of the border between the panels: the columns the panels share,
/// where the border was grabbed and the split at that moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitDrag {
    pub width: u16,
    pub grab: u16,
    pub split: u16,
}

/// Moves the border by as many columns as the pointer moved since the grab
/// (so grabbing does not jump). Saved when the drag ends.
pub fn move_split(context: &mut AppContext, drag: SplitDrag, x: u16) -> bool {
    if drag.width == 0 {
        return false;
    }
    let moved = i32::from(x) - i32::from(drag.grab);
    let width = i32::from(drag.width);
    let delta = (moved * 100 + moved.signum() * width / 2) / width;
    let percent = (i32::from(drag.split) + delta).clamp(0, 100) as u16;
    let split = crate::config::settings::clamp_panel_split(percent);
    let settings = &mut context.config.settings;
    let changed = settings.panel_split != split;
    settings.panel_split = split;
    changed
}
