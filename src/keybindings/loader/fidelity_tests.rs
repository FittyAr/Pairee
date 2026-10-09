//! Each built-in preset keeps the keys of the program it imitates
//! (docs/technical/keybindings-presets-plan.md §5).

use super::tests::{BUILTIN, load};
use crate::keybindings::Action;
use crate::keybindings::chord::normalize_chord;
use crate::keybindings::options::TypingMode;
use crate::keybindings::screens::{ListAction, ViewerAction};

/// `(chord, action)` cases of `preset`'s panels keymap.
fn check(preset: &str, cases: &[(&str, Action)]) {
    let map = load(preset, &[]);
    let leader = map.options.leader.as_deref();
    for (chord, action) in cases {
        let seq = normalize_chord(chord, leader);
        assert_eq!(
            map.panels.resolve_key_string(&seq),
            Some(*action),
            "{preset}: {chord}"
        );
    }
}

#[test]
fn norton_keeps_norton_commander_and_far_keys() {
    use Action as A;
    check(
        "norton",
        &[
            ("F2", A::UserMenu),
            ("F3", A::View),
            ("F4", A::Edit),
            ("Shift+F4", A::NewFile),
            ("F5", A::Copy),
            ("F6", A::Move),
            ("Shift+F6", A::Rename),
            ("F7", A::MkDir),
            ("F8", A::Delete),
            ("F9", A::Menu),
            ("F10", A::Quit),
            ("F11", A::PluginMenu),
            ("Alt+F7", A::FindFile),
            ("Alt+F12", A::FoldersHistory),
            ("Insert", A::SelectItem),
            ("Ctrl+\\", A::GoRoot),
            ("Ctrl+o", A::ToggleBothPanels),
            ("Ctrl+p", A::ToggleInactivePanel),
            ("Ctrl+u", A::SwapPanels),
            ("Ctrl+b", A::ToggleKeybar),
            ("Ctrl+Enter", A::InsertNameToCli),
            ("Ctrl+f", A::InsertPathToCli),
            ("Ctrl+y", A::ClearCli),
            ("Ctrl+e", A::CliHistoryPrev),
            ("Ctrl+x", A::CliHistoryNext),
            ("Ctrl+Alt+t", A::NewTab),
            ("Ctrl+Alt+g", A::OpenGitPanel),
            ("Ctrl+Alt+h", A::Hotlist),
        ],
    );
}

#[test]
fn standard_keeps_explorer_and_vs_code_keys() {
    use Action as A;
    check(
        "standard",
        &[
            ("Ctrl+c", A::Yank),
            ("Ctrl+x", A::Cut),
            ("Ctrl+v", A::Paste),
            ("Delete", A::Trash),
            ("Shift+Delete", A::DeletePermanent),
            ("F2", A::Rename),
            ("F5", A::Copy),
            ("Ctrl+z", A::UndoFileOp),
            ("Ctrl+y", A::RedoFileOp),
            ("Ctrl+a", A::SelectAll),
            ("Ctrl+f", A::FindInPanel),
            ("Ctrl+t", A::NewTab),
            ("Ctrl+w", A::CloseTab),
            ("Ctrl+Tab", A::NextTab),
            ("Alt+Left", A::HistoryBack),
            ("Alt+Up", A::GoParent),
            ("Alt+Enter", A::FileAttributes),
            ("Ctrl+k Ctrl+s", A::WhichKey),
            ("F10", A::Menu),
            ("Ctrl+q", A::Quit),
        ],
    );
}

#[test]
fn neovim_keeps_vim_and_vim_file_manager_keys() {
    use Action as A;
    check(
        "neovim",
        &[
            ("k", A::MoveUp),
            ("j", A::MoveDown),
            ("g g", A::GoToTop),
            ("G", A::GoToBottom),
            ("h", A::GoParent),
            ("-", A::GoParent),
            ("l", A::Execute),
            ("Ctrl+d", A::HalfPageDown),
            ("y y", A::Yank),
            ("p", A::Paste),
            ("d d", A::Trash),
            ("D D", A::DeletePermanent),
            ("c w", A::Rename),
            ("a", A::Create),
            ("v", A::VisualMode),
            ("u", A::UndoFileOp),
            ("Ctrl+r", A::RedoFileOp),
            ("/", A::FindInPanel),
            ("n", A::FindNext),
            (":", A::CommandPalette),
            ("!", A::FocusCli),
            ("g t", A::NextTab),
            ("Ctrl+w w", A::ChangePanel),
            ("<leader>ff", A::FindFile),
            ("g ?", A::WhichKey),
            ("Z Z", A::Quit),
            ("F5", A::Copy),
        ],
    );
}

#[test]
fn yazi_keeps_yazi_keys() {
    use Action as A;
    check(
        "yazi",
        &[
            ("y", A::Yank),
            ("x", A::Cut),
            ("p", A::Paste),
            ("d", A::Trash),
            ("D", A::DeletePermanent),
            ("a", A::Create),
            ("r", A::RenameBasename),
            (".", A::ToggleHidden),
            ("Comma m", A::SortByWriteTime),
            ("m s", A::PanelViewFull),
            ("c c", A::CopyPath),
            ("t", A::NewTab),
            ("1", A::GoToTab(1)),
            ("]", A::NextTab),
            ("H", A::HistoryBack),
            ("Space", A::SelectItem),
            ("f", A::QuickFilter),
            ("w", A::TaskList),
            ("~", A::WhichKey),
            (";", A::FocusCli),
            ("q", A::Quit),
            ("g g", A::GoToTop),
            ("F5", A::Copy),
        ],
    );
}

#[test]
fn letters_follow_each_programs_typing_model() {
    let typing = |preset| load(preset, &[]).options.typing;
    assert_eq!(typing("norton"), TypingMode::Cli);
    assert_eq!(typing("standard"), TypingMode::TypeAhead);
    assert_eq!(typing("neovim"), TypingMode::Commands);
    assert_eq!(typing("yazi"), TypingMode::Commands);
    assert!(load("norton", &[]).options.alt_quick_search);
    assert_eq!(load("neovim", &[]).options.leader.as_deref(), Some("Space"));
    assert_eq!(load("yazi", &[]).options.sequence_timeout_ms, 0);
}

#[test]
fn vim_presets_move_with_jk_in_the_viewer_and_lists() {
    for preset in ["neovim", "yazi"] {
        let map = load(preset, &[]);
        assert_eq!(
            map.viewer.resolve_key_string("j"),
            Some(ViewerAction::LineDown)
        );
        assert_eq!(map.viewer.resolve_key_string("q"), Some(ViewerAction::Quit));
        assert_eq!(map.list.resolve_key_string("k"), Some(ListAction::Up));
    }
    assert_eq!(load("norton", &[]).list.resolve_key_string("k"), None);
}

#[test]
fn norton_leaves_alt_letters_to_the_quick_search() {
    let map = load("norton", &[]);
    for c in 'a'..='z' {
        assert_eq!(
            map.panels.resolve_key_string(&format!("Alt+{c}")),
            None,
            "Alt+{c}"
        );
    }
    assert_eq!(BUILTIN.len(), 4);
}
