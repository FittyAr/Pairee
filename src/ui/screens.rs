//! Screens rendering (Panels, Editor, Viewer, Terminal).

use super::layout::AppLayout;
use super::panel;
use super::quickview;
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType, Screen};
use ansi_to_tui::IntoText as _;
use ratatui::Frame;
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
                let left_active = state.panels.active == ActivePanel::Left;
                let right_active = state.panels.active == ActivePanel::Right;

                // Left panel — replaced by quick view if active and the left panel is passive
                if state.panels.left_visible && layout.left_rect.width > 1 {
                    if state.panels.quick_view_active && !left_active {
                        if let Some(PopupType::QuickViewPanel(qv)) = state.dialogs.top() {
                            quickview::draw_quick_view(
                                f,
                                layout.left_rect,
                                &qv.path,
                                &qv.content,
                                qv.scroll,
                                &context.config.theme,
                                &qv.image_data,
                                &qv.plugin_widget,
                                Some(&state.scrollbar),
                            );
                        } else {
                            panel::render_panel(
                                f,
                                layout.left_rect,
                                &state.panels.left,
                                left_active,
                                context,
                                Some(&state.scrollbar),
                                crate::ui::scrollbar::ScrollTargetId::PanelLeft,
                            );
                        }
                    } else {
                        panel::render_panel(
                            f,
                            layout.left_rect,
                            &state.panels.left,
                            left_active,
                            context,
                            Some(&state.scrollbar),
                            crate::ui::scrollbar::ScrollTargetId::PanelLeft,
                        );
                    }
                }

                // Right panel — replaced by quick view if active and the right panel is passive
                if state.panels.right_visible && layout.right_rect.width > 1 {
                    if state.panels.quick_view_active && !right_active {
                        if let Some(PopupType::QuickViewPanel(qv)) = state.dialogs.top() {
                            quickview::draw_quick_view(
                                f,
                                layout.right_rect,
                                &qv.path,
                                &qv.content,
                                qv.scroll,
                                &context.config.theme,
                                &qv.image_data,
                                &qv.plugin_widget,
                                Some(&state.scrollbar),
                            );
                        } else {
                            panel::render_panel(
                                f,
                                layout.right_rect,
                                &state.panels.right,
                                right_active,
                                context,
                                Some(&state.scrollbar),
                                crate::ui::scrollbar::ScrollTargetId::PanelRight,
                            );
                        }
                    } else {
                        panel::render_panel(
                            f,
                            layout.right_rect,
                            &state.panels.right,
                            right_active,
                            context,
                            Some(&state.scrollbar),
                            crate::ui::scrollbar::ScrollTargetId::PanelRight,
                        );
                    }
                }
            }
        }
        Screen::Editor(ed) => {
            crate::ui::popup::editor::render_editor_widget(
                f,
                layout.main_rect,
                &ed.path,
                &ed.lines,
                ed.cursor_x,
                ed.cursor_y,
                ed.scroll_y,
                ed.is_dirty,
                &context.config.theme,
                state.dialogs.top(),
            );
        }
        Screen::Viewer(vw) => {
            crate::ui::viewer::render_viewer(
                f,
                layout.main_rect,
                vw,
                &context.config.theme,
                state.dialogs.top(),
                context.config.settings.viewer_show_scrollbar,
                Some(&state.scrollbar),
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
