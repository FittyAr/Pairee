//! What an unbound printable key does in the panels, per the preset's
//! `typing` option: start the command line (Far), jump to a file
//! (type-ahead) or nothing (Vim / yazi, where `focus_cli` opens the line).

use crate::keybindings::KeybindingResolver;
use crate::test_harness::Harness;

/// A harness on a user preset extending norton with `typing = <mode>`.
fn with_typing(mode: &str) -> Harness {
    let mut h = Harness::builder().keymap("norton").build();
    h.write(
        "home/config/keymaps/mine.toml",
        format!(
            "extends = \"norton\"\n[options]\ntyping = \"{mode}\"\n[panels]\nfocus_cli = \":\"\n"
        ),
    );
    h.write("work/left/alpha.txt", "a");
    h.write("work/left/beta.log", "b");
    h.write("work/left/bravo.md", "b");
    h.ctx.config.keybindings.preset = "mine".into();
    h.ctx.resolver = KeybindingResolver::new(&h.ctx.config);
    h.reread();
    h
}

#[test]
fn cli_mode_types_into_the_command_line() {
    let mut h = with_typing("cli");
    h.text("ls");
    assert_eq!(h.state.cli_input, "ls");
}

#[test]
fn type_ahead_jumps_to_the_typed_name() {
    let mut h = with_typing("type_ahead");
    h.text("br");
    assert_eq!(h.cursor_name(), "bravo.md");
    h.text("a");
    assert!(h.state.cli_input.is_empty());
}

#[test]
fn commands_mode_ignores_unbound_letters_until_the_line_is_focused() {
    let mut h = with_typing("commands");
    let before = h.cursor_name();
    h.text("xq");
    assert!(h.state.cli_input.is_empty());
    assert_eq!(h.cursor_name(), before);
    h.text(":ls");
    assert_eq!(h.state.cli_input, "ls");
    h.keys("Esc");
    assert!(h.state.cli_input.is_empty() && !h.state.cli_focused);
    h.text("x");
    assert!(h.state.cli_input.is_empty());
}
