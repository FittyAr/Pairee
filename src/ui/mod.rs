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
pub mod which_key_prefix;

use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use ansi_to_tui::IntoText as _;
use ratatui::Frame;
use ratatui::text::Text;

/// Visible terminal scrollback as ratatui `Text`, with SGR colors applied.
///
/// Only the last `max_lines` captured lines are parsed so color can still
/// span newlines inside the viewport without re-parsing the whole buffer.
/// If parsing fails, the joined text is shown unstyled.
fn terminal_output_text(output_lines: &[String], max_lines: usize) -> Text<'static> {
    if output_lines.is_empty() || max_lines == 0 {
        return Text::default();
    }
    let start = output_lines.len().saturating_sub(max_lines);
    let joined = output_lines[start..].join("\n");
    joined.into_text().unwrap_or_else(|_| Text::from(joined))
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::types::TerminalState;
    use crate::app::state::{PopupType, Screen};
    use crate::config::{
        AppConfig, keybindings::KeybindingsConfig, settings::Settings, theme::Theme,
    };
    use crate::fs::FileEntry;
    use ratatui::style::Color;
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

    fn buffer_joined(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn draw_ui_which_key_overlay_lists_live_chords() {
        let (context, mut state) = test_app();
        crate::app::actions::which_key::open_which_key(&mut state, &context.resolver);
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw which-key overlay");
        let painted = buffer_joined(&terminal);
        assert!(
            painted.contains("Which-key"),
            "overlay title missing, got {painted:?}"
        );
        assert!(
            painted.contains("F5") || painted.contains("copy"),
            "live keymap rows missing, got {painted:?}"
        );
    }

    #[test]
    fn draw_ui_prefix_hud_while_sequence_ongoing() {
        let mut config = AppConfig {
            settings: Settings::default(),
            theme: Theme::default(),
            keybindings: KeybindingsConfig::default(),
        };
        config
            .keybindings
            .custom_bindings
            .insert("about".into(), "Alt+q x".into());
        config
            .keybindings
            .custom_bindings
            .insert("help".into(), "Alt+q h".into());
        let mut context = AppContext::new(config);
        let state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let alt_q = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::ALT,
        );
        assert_eq!(context.resolver.resolve(alt_q), None);
        assert!(context.resolver.is_ongoing());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw prefix HUD");
        let painted = buffer_joined(&terminal);
        assert!(
            painted.contains("Prefix") || painted.contains("about") || painted.contains("help"),
            "prefix HUD should show remaining chords, got {painted:?}"
        );
    }

    fn span_has_fg(text: &Text, needle: &str, color: Color) -> bool {
        text.lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains(needle) && span.style.fg == Some(color))
        })
    }

    fn text_plain(text: &Text) -> String {
        text.lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn terminal_output_parses_sgr_red() {
        let lines = vec!["\x1b[31mred\x1b[0m".to_string()];
        let text = terminal_output_text(&lines, 10);
        assert!(
            span_has_fg(&text, "red", Color::Red),
            "expected SGR 31 to paint 'red' as Color::Red, got {:?}",
            text.lines
        );
        assert!(!text_plain(&text).contains('\u{1b}'));
    }

    #[test]
    fn terminal_output_keeps_sgr_across_visible_newlines() {
        let lines = vec!["\x1b[32mhello".to_string(), "world\x1b[0m".to_string()];
        let text = terminal_output_text(&lines, 10);
        assert!(
            span_has_fg(&text, "hello", Color::Green),
            "first line should stay green, got {:?}",
            text.lines
        );
        assert!(
            span_has_fg(&text, "world", Color::Green),
            "joined SGR should color the second line, got {:?}",
            text.lines
        );
    }

    #[test]
    fn terminal_output_limits_to_last_max_lines() {
        let lines = vec!["hidden".to_string(), "visible".to_string()];
        let text = terminal_output_text(&lines, 1);
        let plain = text_plain(&text);
        assert!(plain.contains("visible"), "got {plain:?}");
        assert!(
            !plain.contains("hidden"),
            "viewport should drop older lines, got {plain:?}"
        );
    }

    #[test]
    fn draw_ui_terminal_screen_renders_ansi_without_escape_codes() {
        let (context, mut state) = test_app();
        state.push_screen(Screen::Terminal(TerminalState {
            command: "echo".into(),
            output_lines: vec!["\x1b[31mred\x1b[0m".into(), "plain".into()],
            is_running: false,
            pid: None,
            job_id: None,
        }));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| draw_ui(f, &context, &state))
            .expect("draw terminal screen");

        let buf = terminal.backend().buffer();
        let painted: String = buf.content().iter().map(|cell| cell.symbol()).collect();
        assert!(
            painted.contains("red"),
            "terminal screen should show decoded text"
        );
        assert!(painted.contains("plain"));
        assert!(
            !painted.contains('\u{1b}') && !painted.contains("[31m"),
            "SGR escapes must not leak into the buffer, got a slice around red"
        );
        let has_red_cell = buf.content().iter().any(|cell| {
            cell.symbol() == "r" && cell.fg == Color::Red
                || cell.symbol() == "e" && cell.fg == Color::Red
        });
        assert!(
            has_red_cell,
            "decoded 'red' should be painted with Color::Red"
        );
    }
}
