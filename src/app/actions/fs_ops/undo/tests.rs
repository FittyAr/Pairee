//! Undo/redo from the panels: keys, menu label, confirmation and runs.

use super::{request, transfer_finished};
use crate::app::context::AppContext;
use crate::app::state::popup::CreateKind;
use crate::app::state::{AppState, PopupType};
use crate::config::AppConfig;
use crate::config::localization::t;
use crate::fs::journal::{Direction, FsCommand};
use crate::keybindings::{Action, KeybindingResolver};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;

fn app(dir: &Path) -> (AppContext, AppState) {
    let context = AppContext::new(AppConfig::default());
    let state = AppState::new(dir.to_path_buf(), dir.to_path_buf());
    (context, state)
}

fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
    let key = KeyEvent::new(code, KeyModifiers::NONE);
    let _ = crate::app::input_popup::handle_popup_input(state, key, context);
}

/// Requests `direction` and answers the confirmation with Enter.
fn confirm(state: &mut AppState, context: &mut AppContext, direction: Direction) {
    request(state, direction);
    assert!(
        matches!(state.dialogs.top(), Some(PopupType::ConfirmUndo { .. })),
        "{:?}",
        state.dialogs.top()
    );
    press(state, context, KeyCode::Enter);
}

#[test]
fn empty_journal_says_there_is_nothing_to_undo() {
    let (_, mut state) = app(Path::new("."));
    request(&mut state, Direction::Undo);
    assert!(
        matches!(state.dialogs.top(), Some(PopupType::Info(m)) if *m == t("journal_nothing_to_undo"))
    );
}

#[test]
fn irreversible_entry_is_reported_and_dropped() {
    let (_, mut state) = app(Path::new("."));
    state.journal.record(FsCommand::NotUndoable {
        kind: crate::fs::journal::command::Irreversible::Delete,
        count: 3,
    });
    request(&mut state, Direction::Undo);
    assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
    assert!(state.journal.peek(Direction::Undo).is_none());
}

#[test]
fn new_file_is_undone_only_while_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut context, mut state) = app(tmp.path());
    crate::app::actions::fs_ops::mkdir::handle(&mut state, CreateKind::Auto);
    for c in "notes.md".chars() {
        press(&mut state, &mut context, KeyCode::Char(c));
    }
    press(&mut state, &mut context, KeyCode::Enter);
    let made = tmp.path().join("notes.md");
    assert!(made.is_file());
    assert_eq!(
        state.journal.peek(Direction::Undo).map(FsCommand::verb),
        Some(t("journal_verb_mkfile"))
    );
}

#[test]
fn make_folder_is_undone_and_redone_from_the_dialog() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut context, mut state) = app(tmp.path());
    crate::app::actions::fs_ops::mkdir::handle(&mut state, CreateKind::Folder);
    for c in "made".chars() {
        press(&mut state, &mut context, KeyCode::Char(c));
    }
    press(&mut state, &mut context, KeyCode::Enter);
    let made = tmp.path().join("made");
    assert!(made.is_dir());

    let label = crate::ui::menu::files::journal_label(&state.journal, Direction::Undo);
    assert!(label.contains("made"), "{label}");

    confirm(&mut state, &mut context, Direction::Undo);
    assert!(!made.exists());
    assert!(state.journal.peek(Direction::Redo).is_some());

    confirm(&mut state, &mut context, Direction::Redo);
    assert!(made.is_dir());
    assert!(state.journal.peek(Direction::Redo).is_none());
}

#[test]
fn escape_keeps_the_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut context, mut state) = app(tmp.path());
    std::fs::create_dir(tmp.path().join("d")).unwrap();
    state.journal.record(FsCommand::MakeDir {
        path: tmp.path().join("d"),
    });
    request(&mut state, Direction::Undo);
    press(&mut state, &mut context, KeyCode::Esc);
    assert!(state.dialogs.top().is_none());
    assert!(tmp.path().join("d").exists());
    assert!(state.journal.peek(Direction::Undo).is_some());
}

#[test]
fn rename_is_undone_through_the_rename_executor() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("old.txt"), "x").unwrap();
    let (mut context, mut state) = app(dir);
    crate::app::actions::fs_ops::rename::commit(
        &mut state,
        &mut context,
        "new.txt".into(),
        "old.txt".into(),
        dir.join("old.txt"),
        dir.to_path_buf(),
    );
    assert!(dir.join("new.txt").exists());

    confirm(&mut state, &mut context, Direction::Undo);
    // No Tokio runtime in unit tests: the job ran inline.
    crate::app::actions::fs_ops::multi_rename::poll(&mut state, &context);
    assert!(dir.join("old.txt").exists() && !dir.join("new.txt").exists());
    assert!(matches!(
        state.journal.peek(Direction::Redo),
        Some(FsCommand::Rename { .. })
    ));
}

#[test]
fn changed_files_are_listed_before_undoing() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, mut state) = app(tmp.path());
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    std::fs::write(&b, "b").unwrap();
    std::fs::write(tmp.path().join("c"), "c").unwrap();
    state.journal.record(FsCommand::Trash {
        paths: vec![a.clone(), b.clone()],
    });
    // Undoing a trash restores: `b` is in the way.
    request(&mut state, Direction::Undo);
    let Some(PopupType::ConfirmUndo { lines, .. }) = state.dialogs.top() else {
        panic!("{:?}", state.dialogs.top());
    };
    let text = lines.join("\n");
    assert!(text.contains(&t("journal_will_skip")), "{text}");
    assert!(text.contains(&crate::fs::journal::check::SkipReason::Occupied.text()));
}

#[test]
fn finished_transfer_jobs_are_journaled_once() {
    let (_, mut state) = app(Path::new("."));
    crate::fs::transfer::submit::ensure_transfer_ui(&mut state);
    let job = crate::fs::transfer::job::TransferJob::new(
        crate::fs::transfer::job::TransferOperation::Move,
        vec!["a".into()],
        "dst".into(),
        Default::default(),
    );
    let mut results = crate::fs::transfer::job::TransferResults::default();
    results
        .completed_files
        .push(crate::fs::transfer::control::done(
            Path::new("a"),
            "dst/a".into(),
            1,
            std::time::Instant::now(),
        ));
    transfer_finished(&mut state, &job, &results);
    transfer_finished(&mut state, &job, &job.results);
    assert!(matches!(
        state.journal.peek(Direction::Undo),
        Some(FsCommand::Move(_))
    ));
    state.journal.take(Direction::Undo);
    assert!(
        state.journal.peek(Direction::Undo).is_none(),
        "recorded once"
    );
}

#[test]
fn every_keymap_binds_undo_and_redo() {
    for preset in ["norton", "vscode", "neovim"] {
        let mut config = AppConfig::default();
        config.keybindings.preset = preset.into();
        let mut resolver = KeybindingResolver::new(&config);
        let undo = KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT);
        let redo = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(resolver.resolve(undo), Some(Action::UndoFileOp), "{preset}");
        assert_eq!(resolver.resolve(redo), Some(Action::RedoFileOp), "{preset}");
    }
}

#[test]
fn confirmation_is_rendered() {
    let tmp = tempfile::tempdir().unwrap();
    let (context, mut state) = app(tmp.path());
    std::fs::create_dir(tmp.path().join("folder")).unwrap();
    state.journal.record(FsCommand::MakeDir {
        path: tmp.path().join("folder"),
    });
    request(&mut state, Direction::Undo);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|f| crate::ui::draw_ui(f, &context, &state))
        .unwrap();
    let screen: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(screen.contains(t("journal_undo_title").trim()));
    assert!(screen.contains("folder"));
}
