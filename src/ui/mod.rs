pub mod cli;
pub mod fkeys;
pub mod layout;
pub mod menu;
pub mod panel;
pub mod popup;
pub mod theme_apply;

use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState};
use ratatui::Frame;

/// The primary render dispatch function for drawing the application.
pub fn draw_ui(f: &mut Frame, context: &AppContext, state: &AppState) {
    // 1. Compute geometry partitions
    let layout = layout::calculate_layout(f.size());

    // 2. Draw static bar layouts
    menu::render_menu(f, layout.menu_rect, context);
    fkeys::render_fkeys(f, layout.fkeys_rect, context);
    cli::render_cli(f, layout.cli_rect, state, context);

    // 3. Draw active and passive directory panels
    let left_active = state.active_panel == ActivePanel::Left;
    let right_active = state.active_panel == ActivePanel::Right;

    panel::render_panel(f, layout.left_rect, &state.left_panel, left_active, context);
    panel::render_panel(
        f,
        layout.right_rect,
        &state.right_panel,
        right_active,
        context,
    );

    // 4. Overlap active popup dialogs if present
    popup::render_popup(f, state, context);
}
