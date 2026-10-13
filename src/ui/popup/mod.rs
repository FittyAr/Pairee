pub mod about;
pub mod color_groups;
pub mod command_palette;
pub mod config_dialog;
pub mod disk_usage;
pub mod editor;
pub mod git_commit_prompt;
pub mod git_confirm_checkout;
pub mod git_new_popups;
pub mod git_panel;
pub mod history_lists;
pub mod info;
pub mod kit;
pub mod menus;
pub mod onboarding;
pub mod plugin_menu;
pub mod prompts;
pub mod screens_menu;
pub mod shortcuts;
pub mod sync_dirs;
pub mod update;
pub mod view;
pub mod viewer;
pub mod yazi;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub fn render_popup(
    f: &mut Frame,
    state: &AppState,
    context: &AppContext,
    left_rect: Rect,
    right_rect: Rect,
) {
    let theme = context.theme();
    let cx = view::ViewCtx {
        state,
        context,
        theme: &theme,
        area: f.area(),
        panels: (left_rect, right_rect),
    };
    let depth = state.dialogs.depth();
    for (level, popup) in state.dialogs.iter().enumerate() {
        if level + 1 < depth {
            // Dialogs under the top one stay visible where they were
            // opened, but only the top one takes the mouse.
            crate::ui::hit::muted(|| {
                crate::ui::placement::unplaced(|| render_specific_popup(f, popup, &cx))
            });
        } else {
            render_specific_popup(f, popup, &cx);
        }
    }
}

/// Draws `popup` (a dialog type, or one of the variants not migrated yet).
fn render_specific_popup(f: &mut ratatui::Frame, popup: &PopupType, cx: &view::ViewCtx<'_>) {
    if view::render_dialog(f, popup, cx) {
        return;
    }
    let (state, context, theme, size) = (cx.state, cx.context, cx.theme, cx.area);
    if prompts::render_prompt_popup(f, popup, theme, size, state) {
        return;
    }
    if plugin_menu::render(f, popup, theme, size, context) {
        return;
    }
    if plugin_menu::select::render_dev_select(f, popup, theme, size, Some(&state.scrollbar)) {
        return;
    }
    if git_panel::render(
        f,
        popup,
        theme,
        size,
        Some(&state.scrollbar),
        &state.git_panel,
    ) {
        return;
    }
    if git_commit_prompt::render(f, popup, theme, size) {
        return;
    }
    if git_confirm_checkout::render(f, popup, theme, size) {
        return;
    }
    git_new_popups::render(f, popup, theme, size);
}

/// Records `area` as a dialog on screen: a click there counts as inside it,
/// its top border moves it and (when resizable) its bottom-right corner
/// resizes it. Returns `area`.
pub(crate) fn popup_body(area: Rect) -> Rect {
    use crate::ui::hit::{HitTarget, register};
    register(area, HitTarget::PopupBody);
    if area.width > 2 {
        register(
            Rect::new(area.x + 1, area.y, area.width - 2, 1),
            HitTarget::PopupTitle,
        );
    }
    if crate::ui::placement::resizable() && area.width > 1 && area.height > 1 {
        register(
            Rect::new(area.right() - 1, area.bottom() - 1, 1, 1),
            HitTarget::PopupResize,
        );
    }
    area
}

/// A centered dialog area on `screen`, moved where the user put the dialog,
/// and recorded for the mouse.
pub(crate) fn place_popup(area: Rect, screen: Rect) -> Rect {
    popup_body(crate::ui::placement::place(area, screen))
}

/// Centers a rectangle of fixed `width` and `height` over the full screen.
pub(crate) fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    place_popup(centered_area(width, height, r), r)
}

/// Where [`centered_rect_fixed`] puts a dialog, before the user moves it
/// and without recording it (for dialogs that shift it first).
pub(crate) fn centered_area(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(r.height)),
            Constraint::Length(r.height.saturating_sub(height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(r.width)),
            Constraint::Length(r.width.saturating_sub(width) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Centers a rectangle of `percent_x` × `percent_y` within `parent` (the
/// screen, or a panel for panel-specific popups such as DriveSelect).
pub(crate) fn centered_rect_in(percent_x: u16, percent_y: u16, parent: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(parent);

    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1];
    place_popup(area, parent)
}
