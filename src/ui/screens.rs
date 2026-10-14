//! Screens rendering (Panels, Editor, Viewer, Terminal).

use super::layout::AppLayout;
use super::panel;
use super::quickview;
use super::tab_bar;
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType, Screen};
use ratatui::Frame;
use ratatui::layout::Rect;

pub fn render_screen(
    f: &mut Frame,
    layout: &AppLayout,
    context: &AppContext,
    state: &AppState,
    screen: &Screen,
) {
    match screen {
        Screen::Panels => {
            if !state.panels.both_hidden {
                render_side(f, state, context, ActivePanel::Left, layout.left_rect);
                render_side(f, state, context, ActivePanel::Right, layout.right_rect);
            }
        }
        Screen::Editor(ed) => {
            let settings = &context.config.settings;
            crate::ui::popup::editor::render_editor_widget(
                f,
                layout.main_rect,
                ed,
                crate::ui::popup::editor::EditorView {
                    show_line_numbers: settings.editor_show_line_numbers,
                    syntax: settings.syntax_highlighting,
                },
                &context.theme(),
                state.dialogs.top(),
            );
        }
        Screen::Viewer(vw) => {
            crate::ui::viewer::render_viewer(
                f,
                layout.main_rect,
                vw,
                &crate::ui::viewer::ViewerOpts {
                    theme: &context.theme(),
                    active_popup: state.dialogs.top(),
                    show_scrollbar: context.config.settings.viewer_show_scrollbar,
                    tab_size: context.config.settings.viewer_tab_size as usize,
                    syntax: context.config.settings.syntax_highlighting,
                    scrollbar: Some(&state.scrollbar),
                    search_progress: state.viewer.search_progress(),
                },
            );
        }
        Screen::Terminal(ts) => crate::ui::terminal_screen::render(
            f,
            layout.main_rect,
            ts,
            state.screens.active_id(),
            &context.theme(),
        ),
    }
}

/// One panel, replaced by the quick view when that is open and the panel is
/// the passive one.
fn render_side(
    f: &mut Frame,
    state: &AppState,
    context: &AppContext,
    side: ActivePanel,
    rect: Rect,
) {
    let visible = match side {
        ActivePanel::Left => state.panels.left_visible,
        ActivePanel::Right => state.panels.right_visible,
    };
    if !visible || rect.width <= 1 {
        return;
    }
    let is_active = state.panels.active == side;
    if state.panels.quick_view_active
        && !is_active
        && let Some(PopupType::QuickViewPanel(qv)) = state.dialogs.top()
    {
        quickview::draw_quick_view(
            f,
            rect,
            qv,
            &context.theme(),
            Some(&state.scrollbar),
            context.config.settings.viewer_tab_size as usize,
        );
        return;
    }
    let tabs = state.panels.tabs(side);
    let settings = &context.config.settings;
    let rect = if tab_bar::is_visible(tabs, settings.always_show_tab_bar) && rect.height > 1 {
        let bar = Rect { height: 1, ..rect };
        tab_bar::render(f, bar, tabs, is_active, &context.theme());
        Rect {
            y: rect.y + 1,
            height: rect.height - 1,
            ..rect
        }
    } else {
        rect
    };
    panel::render_panel(
        f,
        rect,
        tabs.panel(),
        (
            side,
            state.mouse.file_drag().and_then(|drag| drag.peek_for(side)),
        ),
        is_active,
        context,
        Some(&state.scrollbar),
    );
}
