pub mod cli;
pub mod fkeys;
pub mod highlight;
pub mod hotkey;
pub mod layout;
pub mod menu;
pub mod panel;
pub mod popup;
pub mod quickview;
pub mod screens;
pub mod scrollbar;
pub mod text_width;
pub mod theme_apply;
pub mod transfer;
pub mod viewer;
pub mod which_key_prefix;

#[cfg(test)]
mod tests;

use crate::app::context::AppContext;
use crate::app::state::AppState;
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
        screens::render_screen(f, &layout, context, state, screen);
    }

    // 4. Overlay active popup dialogs if present
    popup::render_popup(f, state, context, layout.left_rect, layout.right_rect);

    // 4b. Prefix HUD for in-progress multi-key sequences (not a second keymap).
    if state.dialogs.is_none() {
        which_key_prefix::render(f, context, f.area());
    }

    // 5. Render Transfer Panel overlay if active
    transfer::panel::render_transfer_panel(f, state, context);

    if let Some(ref ts) = state.transfer
        && ts.active_conflict_info.is_some()
        && ts.view_mode == crate::app::state::TransferViewMode::Expanded
    {
        let size = f.area();
        transfer::conflict_dialog::render_conflict_dialog(f, size, ts);
    }
}
