//! Built-in editor: find / replace dialog (regex, whole words, replace all
//! as one undo step, history) and syntax highlighting; viewer highlighting.

use crate::app::state::{PopupType, Screen};
use crate::syntax::SyntaxKind;
use crate::test_harness::Harness;
use crate::ui::resolved_theme::ResolvedTheme;

const SOURCE: &str = "fn main() {\n    let año = 1;\n    let año2 = año + 1;\n}\n";

fn lines(h: &Harness) -> Vec<String> {
    match h.state.screens.active() {
        Screen::Editor(ed) => ed.lines.clone(),
        _ => panic!("editor screen expected\n{}", h.screen_text()),
    }
}

fn selected(h: &Harness) -> Option<String> {
    match h.state.screens.active() {
        Screen::Editor(ed) => ed.selected_text(),
        _ => None,
    }
}

/// Foreground of the first cell showing `needle`.
fn fg_of(h: &mut Harness, needle: &str) -> Option<ratatui::style::Color> {
    let (x, y) = h.locate(needle);
    h.cell_style(x, y).fg
}

#[test]
fn replace_all_whole_words_is_one_undo_step_and_find_remembers_it() {
    let mut h = Harness::new();
    h.write("work/left/main.rs", SOURCE);
    h.reread().focus("main.rs").keys("@edit");
    h.keys("Ctrl+F7").text("año").keys("Tab").text("edad");
    // Rows: pattern, replacement, case, whole words, ...; [All] is row 9.
    h.keys("Tab Tab Space Tab*6");
    h.assert_screen("[x] Whole words");
    h.keys("Enter");
    assert_eq!(
        lines(&h)[1..3],
        ["    let edad = 1;", "    let año2 = edad + 1;"]
    );
    h.assert_screen("Replacements made: 2");
    h.keys("Esc Ctrl+z");
    assert_eq!(
        lines(&h)[1..3],
        ["    let año = 1;", "    let año2 = año + 1;"]
    );
    assert_eq!(
        h.state.history.searches.first().map(String::as_str),
        Some("año")
    );
    assert_eq!(
        h.state.history.replacements.first().map(String::as_str),
        Some("edad")
    );

    // F7 offers the last search with its options; Enter selects the match.
    h.keys("F7");
    let Some(PopupType::EditorSearchPrompt(dialog)) = h.state.dialogs.top() else {
        panic!("find dialog expected\n{}", h.screen_text());
    };
    assert_eq!(dialog.query.text(), "año");
    assert!(dialog.options.whole_words && dialog.replacement.is_none());
    h.keys("Enter");
    assert!(h.state.dialogs.is_empty(), "{}", h.screen_text());
    assert_eq!(selected(&h).as_deref(), Some("año"));
    // Next skips "año2" (whole words); previous comes back.
    h.keys("F3");
    let Screen::Editor(ed) = h.state.screens.active() else {
        unreachable!()
    };
    assert_eq!((ed.cursor_y, ed.cursor_x), (2, 20));
    h.keys("Alt+F7");
    let Screen::Editor(ed) = h.state.screens.active() else {
        unreachable!()
    };
    assert_eq!(
        (ed.cursor_y, ed.selected_text().as_deref()),
        (1, Some("año"))
    );
    h.keys("Esc");
    assert!(
        selected(&h).is_none(),
        "Esc drops the selection, not the editor"
    );
    assert!(matches!(h.state.screens.active(), Screen::Editor(_)));
}

#[test]
fn regex_replace_uses_groups_and_reports_bad_patterns() {
    let mut h = Harness::new();
    h.write("work/left/list.txt", "a=1\nb=22\n");
    h.reread().focus("list.txt").keys("@edit");
    h.keys("Ctrl+F7")
        .text("(")
        .keys("Tab Tab Tab Tab Space Enter");
    h.assert_screen("Invalid regular expression");
    h.keys("Esc");
    let Some(PopupType::EditorSearchPrompt(_)) = h.state.dialogs.top() else {
        panic!("the dialog stays open under the error\n{}", h.screen_text());
    };
    h.keys("Up*4 Backspace")
        .text(r"(\w)=(\d+)")
        .keys("Tab")
        .text("$2=$1");
    // [Replace]: first press finds, the next ones replace and move on.
    h.keys("Tab*7 Enter Enter Enter");
    assert_eq!(lines(&h)[..2], ["1=a", "22=b"]);
}

#[test]
fn syntax_colors_follow_the_theme_and_the_setting() {
    let mut h = Harness::new();
    h.write("work/left/main.rs", SOURCE);
    h.reread().focus("main.rs").keys("@edit");
    h.assert_screen("Rust");
    let theme = ResolvedTheme::new(&h.ctx.config.theme);
    let keyword = theme.syntax(SyntaxKind::Keyword).fg;
    assert_eq!(fg_of(&mut h, "fn main"), keyword);
    h.ctx.config.settings.syntax_highlighting = false;
    assert_ne!(fg_of(&mut h, "fn main"), keyword);
    h.keys("F10");

    // The viewer colors the same file.
    h.ctx.config.settings.syntax_highlighting = true;
    h.focus("main.rs").keys("@view");
    h.wait_for_text("fn main");
    assert_eq!(fg_of(&mut h, "fn main"), keyword);
}
