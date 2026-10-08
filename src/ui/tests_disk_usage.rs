//! TestBackend checks for folder sizes in the panel and the disk usage view.

use crate::app::actions::fs_ops::folder_size;
use crate::app::context::AppContext;
use crate::app::input_popup::handle_popup_input;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::config::AppConfig;
use crate::fs::FileEntry;
use crate::ui::draw_ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use std::fs;

/// root/{media/{clip.bin(3000)}, notes.txt(10)}
fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("media")).unwrap();
    fs::write(dir.path().join("media/clip.bin"), vec![0u8; 3000]).unwrap();
    fs::write(dir.path().join("notes.txt"), [0u8; 10]).unwrap();
    dir
}

fn app(root: &std::path::Path) -> (AppContext, AppState) {
    let context = AppContext::new(AppConfig::default());
    let mut state = AppState::new(root.to_path_buf(), root.to_path_buf());
    state.panels.left.view_mode = PanelViewMode::Full;
    state.panels.left.entries = vec![FileEntry {
        name: "media".into(),
        path: root.join("media"),
        size: 0,
        is_dir: true,
        is_symlink: false,
        modified: None,
    }];
    (context, state)
}

fn screen(context: &AppContext, state: &AppState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|f| draw_ui(f, context, state)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn key(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
    let _ = handle_popup_input(state, KeyEvent::new(code, KeyModifiers::NONE), context);
}

#[test]
fn panel_size_column_shows_computed_folder_size() {
    let dir = tree();
    let (context, mut state) = app(dir.path());
    assert!(!screen(&context, &state).contains("2.9 KB"));
    folder_size::calculate(&mut state);
    state.poll_panel_listings();
    assert!(screen(&context, &state).contains("2.9 KB"));
}

#[test]
fn disk_usage_view_lists_and_navigates() {
    let dir = tree();
    let (mut context, mut state) = app(dir.path());
    folder_size::open_disk_usage(&mut state);
    assert!(matches!(state.dialogs.top(), Some(PopupType::DiskUsage)));
    let text = screen(&context, &state);
    assert!(text.contains("media/"), "{text}");
    assert!(text.contains("notes.txt"), "{text}");
    assert!(text.contains("99.7%"), "{text}");

    key(&mut state, &mut context, KeyCode::Enter);
    assert_eq!(state.disk_usage.current_path(), dir.path().join("media"));
    assert!(screen(&context, &state).contains("clip.bin"));

    key(&mut state, &mut context, KeyCode::Backspace);
    assert_eq!(state.disk_usage.current_path(), dir.path());

    key(&mut state, &mut context, KeyCode::Esc);
    assert!(state.dialogs.is_empty());
}

#[test]
fn disk_usage_delete_opens_confirmation_over_the_view() {
    let dir = tree();
    let (mut context, mut state) = app(dir.path());
    context.config.settings.confirmations.confirm_delete = true;
    folder_size::open_disk_usage(&mut state);
    key(&mut state, &mut context, KeyCode::Delete);
    assert!(matches!(
        state.dialogs.top(),
        Some(PopupType::ConfirmDelete { .. })
    ));
    key(&mut state, &mut context, KeyCode::Esc);
    assert!(matches!(state.dialogs.top(), Some(PopupType::DiskUsage)));
}
