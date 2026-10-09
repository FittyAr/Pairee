//! Key strings written by plugins (`[keybindings]` in the manifest, the `on`
//! specs of `pairee.which`) in any of the accepted spellings: `Ctrl+h`,
//! `ctrl+h`, `ctrl-h`, `<C-h>`, `<Down>`.

use keybinds::KeyInput;

/// A key spec split into its modifiers and the key name (case kept).
struct KeySpec {
    ctrl: bool,
    alt: bool,
    shift: bool,
    key: String,
}

impl KeySpec {
    fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        let mut rest = trimmed
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or(trimmed)
            .trim();
        let mut spec = Self {
            ctrl: false,
            alt: false,
            shift: false,
            key: String::new(),
        };
        while let Some((flag, next)) = strip_modifier(rest) {
            match flag {
                Modifier::Ctrl => spec.ctrl = true,
                Modifier::Alt => spec.alt = true,
                Modifier::Shift => spec.shift = true,
            }
            rest = next;
        }
        spec.key = rest.to_string();
        spec
    }
}

#[derive(Clone, Copy)]
enum Modifier {
    Ctrl,
    Alt,
    Shift,
}

/// Prefixes of each modifier, matched case-insensitively.
const MODIFIER_PREFIXES: &[(&str, Modifier)] = &[
    ("ctrl+", Modifier::Ctrl),
    ("ctrl-", Modifier::Ctrl),
    ("control+", Modifier::Ctrl),
    ("control-", Modifier::Ctrl),
    ("c-", Modifier::Ctrl),
    ("alt+", Modifier::Alt),
    ("alt-", Modifier::Alt),
    ("a-", Modifier::Alt),
    ("shift+", Modifier::Shift),
    ("shift-", Modifier::Shift),
    ("s-", Modifier::Shift),
];

/// The leading modifier of `spec` and what follows it. A lone `-` or `+`
/// after the prefix is the key itself (`ctrl+-`), so the rest must not be
/// empty.
fn strip_modifier(spec: &str) -> Option<(Modifier, &str)> {
    MODIFIER_PREFIXES.iter().find_map(|(prefix, modifier)| {
        let head = spec.get(..prefix.len())?;
        let rest = &spec[prefix.len()..];
        (head.eq_ignore_ascii_case(prefix) && !rest.is_empty()).then_some((*modifier, rest))
    })
}

/// Compare a pressed key (keybinds display form) to a Lua `on` spec.
///
/// Accepts both `Ctrl+c` / `Down` (resolver) and `<C-c>` / `<Down>` (Lua).
pub fn key_matches_spec(pressed: &str, spec: &str) -> bool {
    normalize_key_spec(pressed) == normalize_key_spec(spec)
}

/// Lower-case `ctrl+alt+shift+key` form, ignoring the key's case.
pub fn normalize_key_spec(raw: &str) -> String {
    let spec = KeySpec::parse(raw);
    let mut out = String::new();
    if spec.ctrl {
        out.push_str("ctrl+");
    }
    if spec.alt {
        out.push_str("alt+");
    }
    if spec.shift {
        out.push_str("shift+");
    }
    out.push_str(&spec.key.to_ascii_lowercase());
    out
}

/// The spec in the `keybinds` display form the resolver gives a pressed key
/// ([`crate::keybindings::resolver::key_event_to_string`]), e.g. `ctrl-p`
/// → `Ctrl+p`, `<A-Down>` → `Alt+Down`, `shift+g` → `G`. `None` when the
/// key is not one `keybinds` knows.
pub fn canonical_key(raw: &str) -> Option<String> {
    let spec = KeySpec::parse(raw);
    let mut chars = spec.key.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    };
    // A shifted character is the upper-case character itself.
    let (key, shift) = match single {
        Some(c) if spec.shift && c.is_alphabetic() => (c.to_uppercase().collect(), false),
        _ => (
            crate::keybindings::loader::normalize_user_chord(&spec.key),
            spec.shift,
        ),
    };
    let mut chord = String::new();
    for (on, name) in [(spec.ctrl, "Ctrl+"), (spec.alt, "Alt+"), (shift, "Shift+")] {
        if on {
            chord.push_str(name);
        }
    }
    chord.push_str(&key);
    chord.parse::<KeyInput>().ok().map(|k| k.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::resolver::key_event_to_string;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn normalize_lua_and_resolver_ctrl_c() {
        assert_eq!(normalize_key_spec("<C-c>"), "ctrl+c");
        assert_eq!(normalize_key_spec("Ctrl+c"), "ctrl+c");
        assert_eq!(normalize_key_spec("Ctrl+C"), "ctrl+c");
        assert!(key_matches_spec("Ctrl+c", "<C-c>"));
    }

    #[test]
    fn normalize_arrow_and_plain_char() {
        assert_eq!(normalize_key_spec("<Down>"), "down");
        assert_eq!(normalize_key_spec("Down"), "down");
        assert!(key_matches_spec("Down", "<Down>"));
        assert!(key_matches_spec("a", "a"));
        assert!(key_matches_spec("A", "a"));
        assert!(!key_matches_spec("a", "b"));
    }

    #[test]
    fn canonical_key_matches_what_the_resolver_reports() {
        let pressed = |code, mods| key_event_to_string(KeyEvent::new(code, mods));
        let ctrl_h = pressed(KeyCode::Char('h'), KeyModifiers::CONTROL);
        for spec in ["ctrl+h", "ctrl-h", "Ctrl+h", "<C-h>", "CONTROL-h"] {
            assert_eq!(
                canonical_key(spec).as_deref(),
                Some(ctrl_h.as_str()),
                "{spec}"
            );
        }
        let alt_down = pressed(KeyCode::Down, KeyModifiers::ALT);
        assert_eq!(
            canonical_key("<A-Down>").as_deref(),
            Some(alt_down.as_str())
        );
        let big_g = pressed(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(canonical_key("shift+g").as_deref(), Some(big_g.as_str()));
        assert_eq!(canonical_key("G").as_deref(), Some(big_g.as_str()));
        let f5 = pressed(KeyCode::F(5), KeyModifiers::NONE);
        assert_eq!(canonical_key("f5").as_deref(), Some(f5.as_str()));
        let ctrl_minus = pressed(KeyCode::Char('-'), KeyModifiers::CONTROL);
        assert_eq!(
            canonical_key("ctrl+-").as_deref(),
            Some(ctrl_minus.as_str())
        );
        assert_eq!(canonical_key("ctrl+nonsense"), None);
    }
}
