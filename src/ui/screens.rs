//! Screens rendering (Panels, Editor, Viewer, Terminal).

use super::layout::AppLayout;
use super::panel;
use super::quickview;
use super::tab_bar;
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType, Screen};
use ansi_to_tui::IntoText as _;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;

/// Visible terminal scrollback as ratatui Text, with SGR colors applied.
pub fn terminal_output_text(output_lines: &[String], max_lines: usize) -> Text<'static> {
    if output_lines.is_empty() || max_lines == 0 {
        return Text::default();
    }
    let start = output_lines.len().saturating_sub(max_lines);
    let joined = output_lines[start..].join("\n");
    joined.into_text().unwrap_or_else(|_| Text::from(joined))
}

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
                },
                &context.config.theme,
                state.dialogs.top(),
            );
        }
        Screen::Viewer(vw) => {
            crate::ui::viewer::render_viewer(
                f,
                layout.main_rect,
                vw,
                &crate::ui::viewer::ViewerOpts {
                    theme: &context.config.theme,
                    active_popup: state.dialogs.top(),
                    show_scrollbar: context.config.settings.viewer_show_scrollbar,
                    tab_size: context.config.settings.viewer_tab_size as usize,
                    scrollbar: Some(&state.scrollbar),
                    search_progress: state.viewer.search_progress(),
                },
            );
        }
        Screen::Terminal(ts) => {
            let max_lines = layout.main_rect.height.saturating_sub(2) as usize;
            let text = terminal_output_text(&ts.output_lines, max_lines);
            let p = ratatui::widgets::Paragraph::new(text).block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(format!(" Terminal: {} ", ts.command)),
            );
            f.render_widget(p, layout.main_rect);
        }
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
    let (visible, scroll_id) = match side {
        ActivePanel::Left => (
            state.panels.left_visible,
            crate::ui::scrollbar::ScrollTargetId::PanelLeft,
        ),
        ActivePanel::Right => (
            state.panels.right_visible,
            crate::ui::scrollbar::ScrollTargetId::PanelRight,
        ),
    };
    if !visible || rect.width <= 1 {
        return;
    }
    let is_active = state.panels.active == side;
    if state.panels.quick_view_active
        && !is_active
        && let Some(PopupType::QuickViewPanel(qv)) = state.dialogs.top()
    {
        quickview::draw_quick_view(f, rect, qv, &context.config.theme, Some(&state.scrollbar));
        return;
    }
    let tabs = state.panels.tabs(side);
    let settings = &context.config.settings;
    let rect = if tab_bar::is_visible(tabs, settings.always_show_tab_bar) && rect.height > 1 {
        let bar = Rect { height: 1, ..rect };
        tab_bar::render(
            f,
            bar,
            tabs,
            is_active,
            &context.config.theme,
            &state.tab_bar,
        );
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
        is_active,
        context,
        Some(&state.scrollbar),
        scroll_id,
    );
}
