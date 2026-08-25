pub mod cli;
pub mod fkeys;
pub mod highlight;
pub mod hotkey;
pub mod layout;
pub mod menu;
pub mod panel;
pub mod popup;
pub mod quickview;
pub mod scrollbar;
pub mod text_width;
pub mod theme_apply;
pub mod transfer;
pub mod viewer;

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
                    state.dialogs.top(),
                );
            }
            crate::app::state::Screen::Viewer(vw) => {
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

    if let Some(ref ts) = state.transfer
        && ts.active_conflict_info.is_some()
        && ts.view_mode == crate::app::state::TransferViewMode::Expanded
    {
        let size = f.area();
        transfer::conflict_dialog::render_conflict_dialog(f, size, ts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::PopupType;
    use crate::config::{
        AppConfig, keybindings::KeybindingsConfig, settings::Settings, theme::Theme,
    };
    use crate::fs::FileEntry;
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};
    use std::path::PathBuf;

    fn test_app() -> (AppContext, AppState) {
        let config = AppConfig {
            settings: Settings::default(),
            theme: Theme::default(),
            keybindings: KeybindingsConfig::default(),
        };
        let context = AppContext::new(config);
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.panels.left.entries.push(FileEntry {
            name: "日本語ファイル.txt".into(),
            path: PathBuf::from("./日本語ファイル.txt"),
            size: 12,
            is_dir: false,
            is_symlink: false,
            modified: None,
        });
        (context, state)
    }

    fn buffer_nonblank_count(terminal: &Terminal<TestBackend>) -> usize {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|cell| cell.symbol() != " ")
            .count()
    }

    #[test]
    fn draw_ui_fills_standard_size() {
        let (context, state) = test_app();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw 80x24");
        assert!(
            buffer_nonblank_count(&terminal) > 10,
            "dual-panel frame should paint more than a blank screen"
        );
    }

    #[test]
    fn draw_ui_survives_resize_and_tiny_frame() {
        let (context, state) = test_app();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw 80x24");

        terminal
            .resize(Rect::new(0, 0, 40, 12))
            .expect("resize 40x12");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw 40x12");

        terminal
            .resize(Rect::new(0, 0, 10, 5))
            .expect("resize tiny");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw 10x5");

        terminal
            .resize(Rect::new(0, 0, 120, 40))
            .expect("resize wide");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw 120x40");
        assert!(buffer_nonblank_count(&terminal) > 10);
    }

    #[test]
    fn draw_ui_rename_prompt_does_not_panic() {
        let (context, mut state) = test_app();
        state.dialogs.replace(PopupType::RenamePrompt {
            input: "nuevo.txt".into(),
            original: "old.txt".into(),
            src_path: PathBuf::from("./old.txt"),
            parent_dir: PathBuf::from("."),
            cursor_idx: 0,
        });
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw rename overlay");
        assert!(buffer_nonblank_count(&terminal) > 10);
    }
}
