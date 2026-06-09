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

use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use ratatui::Frame;

/// The primary render dispatch function for drawing the application.
pub fn draw_ui(f: &mut Frame, context: &AppContext, state: &AppState) {
    // 1. Compute geometry partitions (respects panel visibility flags)
    let layout = layout::calculate_layout(f.size(), state, &context.config.settings);

    // 2. Draw static bar layouts
    if layout.menu_rect.height > 0 {
        menu::render_menu(f, layout.menu_rect, context, state);
    }
    if layout.fkeys_rect.height > 0 {
        fkeys::render_fkeys(f, layout.fkeys_rect, context);
    }
    cli::render_cli(f, layout.cli_rect, state, context);

    // 3. Draw panels (unless Ctrl+O hides both)
    if !state.both_panels_hidden {
        let left_active = state.active_panel == ActivePanel::Left;
        let right_active = state.active_panel == ActivePanel::Right;

        // Left panel
        if state.left_panel_visible && layout.left_rect.width > 1 {
            panel::render_panel(f, layout.left_rect, &state.left_panel, left_active, context);
        }

        // Right panel — replaced by quick view if active and the right panel is passive
        if state.right_panel_visible && layout.right_rect.width > 1 {
            if state.quick_view_active {
                // Quick view shows the content of the active panel's selected file
                if let Some(PopupType::QuickViewPanel {
                    ref path,
                    ref content,
                    scroll,
                }) = state.active_popup
                {
                    quickview::draw_quick_view(
                        f,
                        layout.right_rect,
                        path,
                        content,
                        scroll,
                        &context.config.theme,
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

    // 4. Overlay active popup dialogs if present
    popup::render_popup(f, state, context, layout.left_rect, layout.right_rect);
}
