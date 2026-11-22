//! The bytes a terminal sends for a key: what the terminal screens and a
//! command run on a server's terminal type into the program.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The bytes a terminal sends for `key` (xterm sequences), if any.
pub fn key_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let seq: &[u8] = match key.code {
        KeyCode::Char(c) if ctrl && c.is_ascii_alphabetic() => {
            return Some(vec![c.to_ascii_lowercase() as u8 - b'a' + 1]);
        }
        KeyCode::Char(c) => {
            let mut bytes = if alt { vec![0x1b] } else { Vec::new() };
            bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            return Some(bytes);
        }
        KeyCode::Enter => b"\r",
        KeyCode::Backspace => b"\x7f",
        KeyCode::Tab => b"\t",
        KeyCode::BackTab => b"\x1b[Z",
        KeyCode::Esc => b"\x1b",
        KeyCode::Up => b"\x1b[A",
        KeyCode::Down => b"\x1b[B",
        KeyCode::Right => b"\x1b[C",
        KeyCode::Left => b"\x1b[D",
        KeyCode::Home => b"\x1b[H",
        KeyCode::End => b"\x1b[F",
        KeyCode::Insert => b"\x1b[2~",
        KeyCode::Delete => b"\x1b[3~",
        KeyCode::PageUp => b"\x1b[5~",
        KeyCode::PageDown => b"\x1b[6~",
        KeyCode::F(n) => return function_key(n).map(|s| s.as_bytes().to_vec()),
        _ => return None,
    };
    Some(seq.to_vec())
}

fn function_key(n: u8) -> Option<&'static str> {
    const KEYS: [&str; 12] = [
        "\x1bOP", "\x1bOQ", "\x1bOR", "\x1bOS", "\x1b[15~", "\x1b[17~", "\x1b[18~", "\x1b[19~",
        "\x1b[20~", "\x1b[21~", "\x1b[23~", "\x1b[24~",
    ];
    KEYS.get(usize::from(n).checked_sub(1)?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn keys_become_terminal_bytes() {
        let none = KeyModifiers::NONE;
        assert_eq!(key_bytes(key(KeyCode::Char('ñ'), none)), Some("ñ".into()));
        assert_eq!(
            key_bytes(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![3])
        );
        assert_eq!(
            key_bytes(key(KeyCode::Char('x'), KeyModifiers::ALT)),
            Some(b"\x1bx".to_vec())
        );
        assert_eq!(key_bytes(key(KeyCode::Enter, none)), Some(b"\r".to_vec()));
        assert_eq!(key_bytes(key(KeyCode::Up, none)), Some(b"\x1b[A".to_vec()));
        assert_eq!(
            key_bytes(key(KeyCode::F(5), none)),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(key_bytes(key(KeyCode::F(13), none)), None);
    }
}
