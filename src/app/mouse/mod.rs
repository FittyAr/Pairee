//! The mouse router: turns a terminal mouse event into what the pointer
//! was on in the last frame ([`crate::ui::hit`]) and acts on it, reusing
//! the keyboard's actions wherever one exists.

mod bars;
pub mod gesture;
mod panel;
pub mod scrollbar;
mod tabs;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::terminal::TerminalBackend;
use crate::ui::hit::{Hit, HitTarget, Layer};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

/// Mouse state kept between events.
#[derive(Debug, Default)]
pub struct MouseState {
    clicks: gesture::ClickCounter,
    /// Right-button drag tagging panel rows.
    mark_drag: Option<panel::MarkDrag>,
    /// The menu bar is shown because the pointer reached the hidden bar.
    menu_revealed: bool,
}

impl MouseState {
    /// The keyboard took over: a menu shown by pointing stays open.
    pub fn keyboard_used(&mut self) {
        self.menu_revealed = false;
    }
}

/// A button press, with how many clicks in a row it completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Click {
    pub button: MouseButton,
    /// 1 = single, 2 = double, 3 = triple click.
    pub count: u8,
    pub modifiers: KeyModifiers,
    pub column: u16,
    pub row: u16,
}

impl Click {
    pub fn is_double(&self) -> bool {
        self.count == 2
    }
}

/// Handles one mouse event. Returns `true` when it changed something (the
/// caller then redraws); pointer motion alone never does.
pub async fn handle(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    mouse: MouseEvent,
) -> anyhow::Result<bool> {
    if mouse.kind == MouseEventKind::Moved {
        let Some(hit) = lookup(state, mouse.column, mouse.row) else {
            return Ok(false);
        };
        return Ok(bars::hover(state, context, &hit.target));
    }
    // The panel wheel moves the cursor (the list follows it), so it goes
    // before the scrollbars, which would only shift the view.
    if matches!(
        mouse.kind,
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
    ) && let Some(consumed) = wheel(state, mouse)
    {
        return Ok(consumed);
    }
    if scrollbar::handle_scrollbar_mouse(state, mouse) {
        return Ok(true);
    }
    if let MouseEventKind::Drag(button) = mouse.kind {
        // A press after a drag is a new single click.
        state.mouse.clicks.reset();
        let target = lookup(state, mouse.column, mouse.row).map(|hit| hit.target);
        if button == MouseButton::Right && panel::drag(state, target.as_ref()) {
            return Ok(true);
        }
    }
    if matches!(mouse.kind, MouseEventKind::Up(_)) && state.mouse.mark_drag.take().is_some() {
        return Ok(true);
    }
    if let MouseEventKind::Down(button) = mouse.kind {
        let interval =
            Duration::from_millis(u64::from(context.config.settings.mouse_double_click_ms));
        let count =
            state
                .mouse
                .clicks
                .press(button, mouse.column, mouse.row, Instant::now(), interval);
        let click = Click {
            button,
            count,
            modifiers: mouse.modifiers,
            column: mouse.column,
            row: mouse.row,
        };
        if let Some(consumed) = on_click(state, context, terminal_backend, click).await? {
            return Ok(consumed);
        }
    }
    Ok(crate::app::screen_input::editor::handle_editor_mouse(
        state, mouse,
    ))
}

/// Acts on a press. `None` lets the event fall through to the editor.
async fn on_click(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    click: Click,
) -> anyhow::Result<Option<bool>> {
    let hit = lookup(state, click.column, click.row);
    if let Some(Hit {
        target: HitTarget::MenuTitle(menu),
        ..
    }) = hit
        && bars::menu_bar_reachable(state)
    {
        return Ok(Some(bars::click_title(state, menu, click)));
    }
    if state.dialogs.is_some() && hit.as_ref().is_none_or(|hit| hit.layer < Layer::Popup) {
        return outside_popup(state, context, terminal_backend, click)
            .await
            .map(Some);
    }
    let Some(Hit { target, .. }) = hit else {
        return Ok(None);
    };
    let show_hidden = context.config.settings.show_hidden;
    Ok(match target {
        HitTarget::Tab(id) => Some(tabs::click(state, id, click, show_hidden)),
        HitTarget::PanelBody(_)
        | HitTarget::PanelTitle(_)
        | HitTarget::PanelHeader { .. }
        | HitTarget::PanelRow { .. } => {
            Some(panel::click(state, context, terminal_backend, target, click).await?)
        }
        HitTarget::MenuReveal => {
            let width = state
                .hits
                .first(|t| (t == &HitTarget::MenuReveal).then_some(()));
            let width = width.map_or(0, |(area, ())| area.width);
            Some(bars::click_reveal(state, click, width))
        }
        HitTarget::MenuItem { menu, row } => {
            Some(bars::click_item(state, context, terminal_backend, (menu, row), click).await?)
        }
        HitTarget::FKey { number, modifiers } => Some(
            bars::click_fkey(state, context, terminal_backend, (number, modifiers), click).await?,
        ),
        HitTarget::Action(action) if click.button == MouseButton::Left => {
            act(state, context, terminal_backend, action).await?;
            Some(true)
        }
        HitTarget::MenuTitle(_)
        | HitTarget::Action(_)
        | HitTarget::PopupBody
        | HitTarget::Scrollbar(_) => Some(false),
    })
}

/// A wheel notch not taken by a scrollbar. `None` lets it fall through.
fn wheel(state: &mut AppState, mouse: MouseEvent) -> Option<bool> {
    let hit = lookup(state, mouse.column, mouse.row)?;
    if state.dialogs.is_some() && hit.layer < Layer::Popup {
        return Some(false);
    }
    let down = mouse.kind == MouseEventKind::ScrollDown;
    panel::side_of(&hit.target).map(|side| panel::wheel(state, side, down))
}

/// Runs `action` as if its key had been pressed.
async fn act(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    action: crate::keybindings::Action,
) -> anyhow::Result<()> {
    crate::app::actions::handle_action(state, action, context, terminal_backend).await
}

/// A press outside the open dialog: transient popups close as with `Esc`,
/// the rest ignore it. The panels behind never see it.
async fn outside_popup(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    click: Click,
) -> anyhow::Result<bool> {
    // A popup that drew nothing recognisable (no body) might be clicked
    // inside, so only close what is known to be outside. The menu may show
    // just its highlighted title, with no drop-down.
    let drawn = state
        .hits
        .first(|t| (t == &HitTarget::PopupBody).then_some(()))
        .is_some()
        || matches!(
            state.dialogs.top(),
            Some(crate::app::state::PopupType::Menu { .. })
        );
    let closes = click.button == MouseButton::Left
        && drawn
        && state
            .dialogs
            .top()
            .is_some_and(|popup| popup.closes_on_outside_click());
    if !closes {
        return Ok(false);
    }
    let esc = crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Esc);
    crate::app::app::events::dispatch_popup_key(state, context, terminal_backend, esc).await?;
    Ok(true)
}

/// What the pointer is on. The hidden menu bar's reveal strip lies over
/// the top row, but tabs drawn there keep their clicks.
fn lookup(state: &AppState, x: u16, y: u16) -> Option<Hit> {
    let hit = state.hits.at(x, y)?;
    if hit.target != HitTarget::MenuReveal {
        return Some(hit);
    }
    state
        .hits
        .find(x, y, |below| matches!(below.target, HitTarget::Tab(_)))
        .or(Some(hit))
}
