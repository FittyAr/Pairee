//! Terminal screens (`command &`): output reaches the screen that started
//! it even after earlier screens close, the process takes input and can be
//! stopped, the screen closes from the keyboard, the mouse and the screens
//! list, and the output is bounded.

use crate::app::state::{Screen, ScreenId};
use crate::test_harness::Harness;

/// A command that echoes its input lines until it is stopped.
const ECHO_INPUT: &str = if cfg!(windows) {
    "findstr \"^\""
} else {
    "cat"
};

fn run_in_background(h: &mut Harness, command: &str) -> ScreenId {
    h.text(&format!("{command} &")).keys("Enter");
    assert!(matches!(h.state.screens.active(), Screen::Terminal(_)));
    h.state.screens.active_id()
}

fn terminal(h: &Harness, id: ScreenId) -> &crate::app::state::TerminalState {
    let index = h.state.screens.index_of(id).expect("screen open");
    match h.state.screens.get(index) {
        Some(Screen::Terminal(ts)) => ts,
        other => panic!("not a terminal: {other:?}"),
    }
}

fn output(h: &Harness, id: ScreenId) -> String {
    terminal(h, id).output.all().join("\n")
}

#[test]
fn output_reaches_its_screen_after_earlier_screens_close() {
    let mut h = Harness::new();
    let first = run_in_background(&mut h, ECHO_INPUT);
    h.keys("@prev_screen");
    let second = run_in_background(&mut h, "echo second-output");
    h.wait_until("second output", |h| {
        output(h, second).contains("second-output")
    });

    // Closing the first screen used to shift the indexes the output was
    // routed by.
    h.state.close_screen(first);
    assert!(h.state.screens.index_of(first).is_none());
    h.wait_until("second exits", |h| !terminal(h, second).is_running());
    assert!(output(&h, second).contains("second-output"));
}

#[test]
fn input_reaches_the_process_and_kill_stops_it() {
    let mut h = Harness::new();
    let id = run_in_background(&mut h, ECHO_INPUT);
    assert!(terminal(&h, id).pid().is_some(), "the real pid is kept");
    h.text("marker42").keys("Enter");
    // On the rendered screen: under load ConPTY may paint its window title
    // (an OSC sequence) in the middle of the echoed text.
    h.wait_for_text("marker42");

    h.keys("F8");
    h.wait_until("killed", |h| !terminal(h, id).is_running());
    h.keys("F10");
    assert_eq!(h.state.screens.len(), 1, "F10 closes the terminal screen");
}

#[test]
fn close_box_and_screens_list_close_terminals() {
    let mut h = Harness::new();
    let id = run_in_background(&mut h, ECHO_INPUT);
    let (col, row) = h.hit_cell(|t| *t == crate::ui::hit::HitTarget::CloseScreen(id));
    h.click(col, row);
    assert_eq!(h.state.screens.len(), 1, "the close box closes it");

    run_in_background(&mut h, ECHO_INPUT);
    h.keys("@prev_screen @screens_list Down Delete");
    assert_eq!(
        h.state.screens.len(),
        1,
        "Delete in the screens list closes it"
    );
}

#[test]
fn scrollback_is_bounded_and_scrolls() {
    let mut h = Harness::builder()
        .settings(|s| s.terminal_scrollback_lines = 100)
        .build();
    let command = if cfg!(windows) {
        "for /L %i in (1,1,300) do @echo line%i"
    } else {
        "for i in $(seq 1 300); do echo line$i; done"
    };
    let id = run_in_background(&mut h, command);
    h.wait_until("exit", |h| !terminal(h, id).is_running());
    let ts = terminal(&h, id);
    assert_eq!(ts.output.len(), 100, "only the newest lines are kept");
    let text = output(&h, id);
    assert!(
        text.contains("line300") && !text.lines().any(|l| l == "line150"),
        "{text}"
    );
    h.wait_for_text("line300");

    h.keys("Shift+PageUp");
    assert_eq!(terminal(&h, id).scroll, 20);
    h.assert_no_screen("line300");
    let (col, row) = h.hit_cell(|t| *t == crate::ui::hit::HitTarget::TerminalOutput(id));
    h.wheel(col, row + 1, true);
    assert_eq!(terminal(&h, id).scroll, 17, "the wheel scrolls forward");
    h.keys("Shift+End");
    assert_eq!(terminal(&h, id).scroll, 0);
}
