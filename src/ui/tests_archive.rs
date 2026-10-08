//! TestBackend checks for browsing archives in a panel: entering with
//! Enter / Ctrl+PgDn, subfolders, leaving through `..` with the cursor
//! back on the archive, and actions an archive cannot run.

use crate::app::actions::fs_ops::capability::refuse_unsupported;
use crate::app::actions::navigation::handle_navigation_action;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::config::AppConfig;
use crate::fs::archive::test_fixtures::{SAMPLE_TREE, write_zip};
use crate::keybindings::Action;
use crate::ui::draw_ui;
use crate::ui::panel::helpers::build_panel_title;
use ratatui::{Terminal, backend::TestBackend};
use std::path::Path;

/// root/{bundle.zip(SAMPLE_TREE), plain.txt}
fn app() -> (tempfile::TempDir, AppContext, AppState) {
    let dir = tempfile::tempdir().unwrap();
    write_zip(&dir.path().join("bundle.zip"), SAMPLE_TREE);
    std::fs::write(dir.path().join("plain.txt"), b"plain").unwrap();
    let context = AppContext::new(AppConfig::default());
    let mut state = AppState::new(dir.path().to_path_buf(), dir.path().to_path_buf());
    state
        .panels
        .side_mut(crate::app::state::ActivePanel::Left)
        .view_mode = PanelViewMode::Full;
    state.refresh_both_panels(false);
    (dir, context, state)
}

fn screen(context: &AppContext, state: &AppState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
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

fn cursor_to(state: &mut AppState, name: &str) {
    let panel = state.get_active_panel_mut();
    panel.cursor_index = panel.entries.iter().position(|e| e.name == name).unwrap();
}

fn cursor_name(state: &AppState) -> String {
    let panel = state.get_active_panel();
    panel.entries[panel.cursor_index].name.clone()
}

fn act(state: &mut AppState, context: &mut AppContext, action: Action) {
    assert!(handle_navigation_action(state, &action, context));
}

#[test]
fn entering_and_leaving_an_archive() {
    let (dir, mut context, mut state) = app();
    let archive = dir.path().join("bundle.zip");

    cursor_to(&mut state, "bundle.zip");
    act(&mut state, &mut context, Action::Execute);
    let panel = state.get_active_panel();
    assert_eq!(panel.current_path, archive);
    assert!(panel.source.archive().is_some());
    let text = screen(&context, &state);
    assert!(text.contains("a.txt") && text.contains("sub"), "{text}");

    cursor_to(&mut state, "sub");
    act(&mut state, &mut context, Action::Execute);
    let title = build_panel_title(state.get_active_panel(), &context.config.settings);
    let inner = Path::new("bundle.zip").join("sub");
    assert!(title.contains(&*inner.to_string_lossy()), "{title}");
    let text = screen(&context, &state);
    assert!(text.contains("b.txt") && text.contains("deeper"), "{text}");

    // `..` twice: back to the archive root, then out of it.
    cursor_to(&mut state, "..");
    act(&mut state, &mut context, Action::Execute);
    assert_eq!(cursor_name(&state), "sub");
    act(&mut state, &mut context, Action::GoParent);
    let panel = state.get_active_panel();
    assert_eq!(panel.current_path, dir.path());
    assert!(panel.source.is_local());
    assert_eq!(cursor_name(&state), "bundle.zip");
    assert!(screen(&context, &state).contains("plain.txt"));
}

#[test]
fn ctrl_page_down_opens_archives_but_not_files() {
    let (dir, mut context, mut state) = app();
    cursor_to(&mut state, "plain.txt");
    act(&mut state, &mut context, Action::OpenArchive);
    assert_eq!(state.get_active_panel().current_path, dir.path());
    assert!(state.screens.len() <= 1, "no viewer opened");

    cursor_to(&mut state, "bundle.zip");
    act(&mut state, &mut context, Action::OpenArchive);
    assert_eq!(
        state.get_active_panel().current_path,
        dir.path().join("bundle.zip")
    );
}

#[test]
fn local_only_actions_are_refused_inside_archives() {
    let (_dir, mut context, mut state) = app();
    cursor_to(&mut state, "bundle.zip");
    act(&mut state, &mut context, Action::Execute);
    for action in [Action::Edit, Action::Rename, Action::MultiRename] {
        state.dialogs.clear();
        assert!(refuse_unsupported(&mut state, &action), "{action:?}");
        assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
    }
    // Zip archives take new folders, copies out and deletions.
    for action in [Action::MkDir, Action::Copy, Action::Delete] {
        assert!(!refuse_unsupported(&mut state, &action), "{action:?}");
    }
    assert!(refuse_unsupported(&mut state, &Action::Move));
}

#[test]
fn viewer_and_quick_view_read_archive_entries() {
    use crate::app::state::quick_view::load::load_preview;
    let (dir, mut context, mut state) = app();
    cursor_to(&mut state, "bundle.zip");
    act(&mut state, &mut context, Action::Execute);
    let entry = dir.path().join("bundle.zip").join("a.txt");

    let vfs = state.vfs_of_listed(&entry);
    assert_eq!(
        load_preview(vfs.as_ref(), &entry, false, 1024).content,
        ["alpha"]
    );
    let folder = dir.path().join("bundle.zip").join("sub");
    let listing = load_preview(vfs.as_ref(), &folder, false, 1024);
    assert!(listing.content.iter().any(|l| l == "b.txt"), "{listing:?}");

    state.open_viewer(entry, &context.config.settings, false);
    let viewer = state.active_viewer_mut().expect("viewer screen");
    assert_eq!(viewer.doc.lines(0, 5), ["alpha"]);
}
