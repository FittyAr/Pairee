pub mod cli;
pub mod fkeys;
pub mod highlight;
pub mod hotkey;
pub mod layout;
pub mod menu;
pub mod panel;
pub mod popup;
pub mod quickview;
pub mod theme_apply;
pub mod viewer;
pub mod transfer;

use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use ratatui::Frame;

/// The primary render dispatch function for drawing the application.
pub fn draw_ui(f: &mut Frame, context: &AppContext, state: &AppState) {
    // 1. Compute geometry partitions (respects panel visibility flags)
    let layout = layout::calculate_layout(f.area(), state, &context.config.settings);

    // 2. Draw static bar layouts
    if layout.menu_rect.height > 0 {
        menu::render_menu(f, layout.menu_rect, context, state);
    }
    if layout.fkeys_rect.height > 0 {
        fkeys::render_fkeys(f, layout.fkeys_rect, context, state);
    }
    if layout.transfer_rect.height > 0 {
        transfer::bar::render_transfer_bar(f, layout.transfer_rect, state, context);
    }
    cli::render_cli(f, layout.cli_rect, state, context);

    // 3. Draw active screen
    if let Some(screen) = state.screens.get(state.active_screen_idx) {
        match screen {
            crate::app::state::Screen::Panels => {
                if !state.both_panels_hidden {
                    let left_active = state.active_panel == ActivePanel::Left;
                    let right_active = state.active_panel == ActivePanel::Right;

                    // Left panel — replaced by quick view if active and the left panel is passive
                    if state.left_panel_visible && layout.left_rect.width > 1 {
                        if state.quick_view_active && !left_active {
                            if let Some(PopupType::QuickViewPanel {
                                ref path,
                                ref content,
                                scroll,
                                ref image_data,
                                ref plugin_widget,
                            }) = state.active_popup
                            {
                                quickview::draw_quick_view(
                                    f,
                                    layout.left_rect,
                                    path,
                                    content,
                                    scroll,
                                    &context.config.theme,
                                    image_data,
                                    plugin_widget,
                                );
                            } else {
                                panel::render_panel(
                                    f,
                                    layout.left_rect,
                                    &state.left_panel,
                                    left_active,
                                    context,
                                );
                            }
                        } else {
                            panel::render_panel(
                                f,
                                layout.left_rect,
                                &state.left_panel,
                                left_active,
                                context,
                            );
                        }
                    }

                    // Right panel — replaced by quick view if active and the right panel is passive
                    if state.right_panel_visible && layout.right_rect.width > 1 {
                        if state.quick_view_active && !right_active {
                            if let Some(PopupType::QuickViewPanel {
                                ref path,
                                ref content,
                                scroll,
                                ref image_data,
                                ref plugin_widget,
                            }) = state.active_popup
                            {
                                quickview::draw_quick_view(
                                    f,
                                    layout.right_rect,
                                    path,
                                    content,
                                    scroll,
                                    &context.config.theme,
                                    image_data,
                                    plugin_widget,
                                );
                            } else {
                                panel::render_panel(
                                    f,
                                    layout.right_rect,
                                    &state.right_panel,
                                    right_active,
                                    context,
                                );
                            }
                        } else {
                            panel::render_panel(
                                f,
                                layout.right_rect,
                                &state.right_panel,
                                right_active,
                                context,
                            );
                        }
                    }
                }
            }
            crate::app::state::Screen::Editor(ed) => {
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
                    &state.active_popup,
                );
            }
            crate::app::state::Screen::Viewer(vw) => {
                crate::ui::viewer::render_viewer(
                    f,
                    layout.main_rect,
                    vw,
                    &context.config.theme,
                    &state.active_popup,
                );
            }
            crate::app::state::Screen::Terminal(ts) => {
                let lines: Vec<ratatui::text::Line> = ts
                    .output_lines
                    .iter()
                    .rev()
                    .take((layout.main_rect.height.saturating_sub(2)) as usize)
                    .rev()
                    .map(|l| ratatui::text::Line::from(l.as_str()))
                    .collect();
                let p = ratatui::widgets::Paragraph::new(lines).block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title(format!(" Terminal: {} ", ts.command)),
                );
                f.render_widget(p, layout.main_rect);
            }
        }
    }

    // 4. Overlay active popup dialogs if present
    popup::render_popup(f, state, context, layout.left_rect, layout.right_rect);

    // 5. Render Transfer Panel overlay if active
    transfer::panel::render_transfer_panel(f, state, context);
}
