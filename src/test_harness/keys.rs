//! A tiny key DSL for scenario tests: `"F7 n e w Enter"`, `"Ctrl+c"`,
//! `"Alt+Shift+PageUp"`, `"Space"`. Tokens are separated by whitespace;
//! a token starting with `@` names a keymap action (`@mkdir`) and is
//! resolved through the live keymap by the harness.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// One parsed DSL token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Key(KeyEvent),
    /// `@name`: the action's bound chord, or the action itself when unbound.
    Action(String),
}

/// Splits `script` into tokens; `Key*N` repeats a token `N` times. Panics
/// on an unknown key name so a typo in a test fails loudly instead of
/// sending nothing.
pub fn parse(script: &str) -> Vec<Token> {
    script
        .split_whitespace()
        .flat_map(|word| {
            let (token, times) = match word.rsplit_once('*') {
                Some((token, n)) if !token.is_empty() && n.parse::<usize>().is_ok() => {
                    (token, n.parse().unwrap_or(1))
                }
                _ => (word, 1),
            };
            std::iter::repeat_n(parse_token(token), times)
        })
        .collect()
}

fn parse_token(token: &str) -> Token {
    match token.strip_prefix('@') {
        Some(action) if !action.is_empty() => Token::Action(action.to_string()),
        _ => Token::Key(parse_chord(token).unwrap_or_else(|| panic!("unknown key {token:?}"))),
    }
}

/// Parses one chord such as `Ctrl+Shift+PageUp`, `F5` or `a`. A lone `+`
/// is the plus key. Returns `None` for an unknown key name.
pub fn parse_chord(chord: &str) -> Option<KeyEvent> {
    let (mods_part, key_part) = match chord.rfind('+') {
        Some(idx) if idx + 1 < chord.len() => (&chord[..idx], &chord[idx + 1..]),
        Some(idx) if idx > 0 => (&chord[..idx - 1], "+"),
        _ => ("", chord),
    };
    let mut modifiers = KeyModifiers::NONE;
    for name in mods_part.split('+').filter(|m| !m.is_empty()) {
        modifiers |= match name.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => KeyModifiers::CONTROL,
            "alt" => KeyModifiers::ALT,
            "shift" => KeyModifiers::SHIFT,
            "super" | "cmd" | "win" => KeyModifiers::SUPER,
            _ => return None,
        };
    }
    let code = key_code(key_part)?;
    // Terminals report Shift+Tab as BackTab.
    if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        return Some(KeyEvent::new(KeyCode::BackTab, modifiers));
    }
    Some(KeyEvent::new(code, modifiers))
}

fn key_code(name: &str) -> Option<KeyCode> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(KeyCode::Char(c));
    }
    if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
        return Some(KeyCode::F(n));
    }
    Some(match name {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "BackTab" => KeyCode::BackTab,
        "Backspace" => KeyCode::Backspace,
        "Delete" | "Del" => KeyCode::Delete,
        "Insert" | "Ins" => KeyCode::Insert,
        "Space" => KeyCode::Char(' '),
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Menu" => KeyCode::Menu,
        _ => return None,
    })
}

/// The key event a terminal sends for a typed character.
pub fn typed(c: char) -> KeyEvent {
    let modifiers = if c.is_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(c), modifiers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chords_actions_and_plain_keys() {
        let tokens = parse("F7 n Enter Ctrl+\\ Alt+Shift+PageUp @mkdir Shift+Tab +");
        assert_eq!(
            tokens,
            vec![
                Token::Key(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE)),
                Token::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
                Token::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                Token::Key(KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
                Token::Key(KeyEvent::new(
                    KeyCode::PageUp,
                    KeyModifiers::ALT | KeyModifiers::SHIFT
                )),
                Token::Action("mkdir".into()),
                Token::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
                Token::Key(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE)),
            ]
        );
    }

    #[test]
    fn repetition_and_star_key() {
        let down = Token::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(parse("Down*3"), vec![down.clone(), down.clone(), down]);
        assert_eq!(
            parse("*"),
            vec![Token::Key(KeyEvent::new(
                KeyCode::Char('*'),
                KeyModifiers::NONE
            ))]
        );
    }

    #[test]
    fn ctrl_plus_is_parsed() {
        assert_eq!(
            parse_chord("Ctrl++"),
            Some(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::CONTROL))
        );
    }

    #[test]
    #[should_panic(expected = "unknown key")]
    fn unknown_key_names_fail_loudly() {
        parse("Ctrl+Nope");
    }
}
