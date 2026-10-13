pub mod cli;
pub mod drag_label;
pub mod fkeys;
pub mod half_block_image;
pub mod highlight;
pub mod hit;
pub mod hotkey;
pub mod layout;
pub mod menu;
pub mod panel;
pub mod placement;
pub mod popup;
pub mod quickview;
pub mod render_cache;
pub mod resolved_theme;
pub mod screens;
pub mod scrollbar;
pub mod search_highlight;
pub mod tab_bar;
pub mod terminal_screen;
pub mod text_width;
pub mod theme_apply;
pub mod too_small;
pub mod transfer;
pub mod viewer;
pub mod which_key_prefix;
pub mod wrap;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_archive;
#[cfg(test)]
mod tests_disk_usage;
#[cfg(test)]
mod tests_tab_bar;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use ratatui::Frame;
use ratatui::layout::Rect;

/// The primary render dispatch function for drawing the application.
pub fn draw_ui(f: &mut Frame, context: &AppContext, state: &AppState) {
    let _recording = hit::Recording::start(&state.hits);
    if too_small::is_too_small(f.area()) {
        too_small::render(f);
        return;
    }
    // 1. Compute geometry partitions (respects panel visibility flags)
    let layout = layout::calculate_layout(f.area(), state, &context.config);

    // 2. Draw static bar layouts
    if layout.fkeys_rect.height > 0 {
        fkeys::render_fkeys(f, layout.fkeys_rect, context, state);
    }
    if layout.transfer_rect.height > 0 {
        transfer::bar::render_transfer_bar(f, layout.transfer_rect, state, context);
    }
    cli::render_cli(f, layout.cli_rect, state, context);

    // 3. Draw active screen
    if let Some(screen) = Some(state.screens.active()) {
        screens::render_screen(f, &layout, context, state, screen);
    }

    // 3a. The menu bar, after the screen: it may lie over the top row.
    if layout.menu_rect.height > 0 {
        menu::render_menu(f, layout.menu_rect, context, state);
    }

    // 3b. The border between two visible panels can be dragged.
    if matches!(
        Some(state.screens.active()),
        Some(crate::app::state::Screen::Panels)
    ) && state.panels.left_visible
        && state.panels.right_visible
        && layout.left_rect.width > 1
        && layout.right_rect.width > 1
    {
        let border = Rect {
            x: layout.left_rect.right() - 1,
            width: 2,
            y: layout.left_rect.y + 1,
            height: layout.left_rect.height.saturating_sub(2),
        };
        let width = layout.right_rect.right() - layout.left_rect.x;
        hit::register(border, hit::HitTarget::PanelSplitter { width });
    }

    // 3c. A hidden menu bar shows up when the pointer reaches the top row.
    if layout.menu_rect.height == 0 && state.dialogs.is_none() {
        hit::register(
            Rect {
                height: 1,
                ..f.area()
            },
            hit::HitTarget::MenuReveal,
        );
    }

    // 4. Overlay active popup dialogs if present, where the user put them.
    hit::begin_layer(hit::Layer::Popup);
    let (placement, resizable) = state.mouse.placement_for(&state.dialogs);
    placement::apply(placement, resizable);
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
    state.mouse.placed(placement::finish());
    // Last, over everything: where dragged files would go.
    drag_label::render(f, state, &context.theme());
}
