//! Chord spelling: the one place that turns what users write in keymap TOML,
//! config overrides and plugin manifests into `keybinds` grammar, plus the
//! table of chords that some terminals cannot deliver.

use keybinds::{Key, KeyInput, KeySeq, Mods};

/// Token that stands for the preset's leader key (`"<leader> g b"`,
/// or compact `"<leader>gb"`).
pub const LEADER_TOKEN: &str = "<leader>";

/// Normalizes a chord or a space-separated key sequence written by a user,
/// expanding `<leader>` to `leader`.
pub fn normalize_chord(raw: &str, leader: Option<&str>) -> String {
    raw.split_whitespace()
        .flat_map(|token| expand_leader(token, leader))
        .map(|token| normalize_user_chord(&token))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `<leader>` → the leader; `<leader>gb` → leader, `g`, `b`. Without a
/// configured leader the token is kept, so the chord fails to parse and the
/// loader reports it.
fn expand_leader(token: &str, leader: Option<&str>) -> Vec<String> {
    let (Some(rest), Some(leader)) = (strip_prefix_ci(token, LEADER_TOKEN), leader) else {
        return vec![token.to_string()];
    };
    std::iter::once(leader.to_string())
        .chain(rest.chars().map(String::from))
        .collect()
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// Maps legacy / friendly aliases of a single chord to `keybinds` grammar.
pub fn normalize_user_chord(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    match s.to_ascii_lowercase().as_str() {
        "gray+" => return "Plus".into(),
        "gray-" => return "-".into(),
        "gray*" => return "*".into(),
        "menu" => return "Menu".into(),
        _ => {}
    }
    let s = comma_alias(s).unwrap_or_else(|| s.to_string());
    shift_letter_to_uppercase(&s).unwrap_or(s)
}

/// `Comma` names the `,` key, which cannot be written literally because
/// commas separate alternative chords (`Ctrl+Comma` → `Ctrl+,`).
fn comma_alias(chord: &str) -> Option<String> {
    let (mods, key) = match chord.rsplit_once('+') {
        Some((mods, key)) => (Some(mods), key),
        None => (None, chord),
    };
    if !key.eq_ignore_ascii_case("comma") {
        return None;
    }
    Some(mods.map_or_else(|| ",".to_string(), |m| format!("{m}+,")))
}

/// `keybinds` only accepts `Shift` with named keys: a shifted letter is
/// written as the uppercase letter (`Ctrl+Shift+k` → `Ctrl+K`). Returns the
/// rewritten chord, or `None` when `chord` is not a shifted letter.
fn shift_letter_to_uppercase(chord: &str) -> Option<String> {
    let (mods, key) = chord.rsplit_once('+')?;
    let mut chars = key.chars();
    let letter = chars.next().filter(|c| c.is_alphabetic())?;
    if chars.next().is_some() {
        return None;
    }
    let parts: Vec<&str> = mods.split('+').collect();
    if !parts.iter().any(|m| m.eq_ignore_ascii_case("shift")) {
        return None;
    }
    let mut out: Vec<String> = parts
        .into_iter()
        .filter(|m| !m.eq_ignore_ascii_case("shift"))
        .map(str::to_string)
        .collect();
    out.push(letter.to_uppercase().collect());
    Some(out.join("+"))
}

/// Why a chord may never reach Pairee on some terminals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fragility {
    /// Only distinguishable with the kitty keyboard protocol
    /// (`Ctrl+Shift+<letter>`, `Ctrl+Enter`, `Ctrl+<digit>`…).
    NeedsKittyProtocol,
    /// Sends the same bytes as another key on legacy terminals
    /// (`Ctrl+i` = Tab, `Ctrl+m` = Enter, `Ctrl+h` = Backspace, `Ctrl+[` = Esc).
    AliasesLegacyKey,
    /// Commonly taken by the terminal or window manager (`Alt+F4`, `F11`).
    TakenByTerminal,
    /// `Ctrl+Alt` is AltGr on Windows: with a digit or `e` / `q` / `m` it
    /// types a character (`@`, `€`...) on many keyboard layouts.
    AltGr,
}

impl Fragility {
    /// i18n key explaining the problem.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::NeedsKittyProtocol => "keymap_fragile_kitty",
            Self::AliasesLegacyKey => "keymap_fragile_alias",
            Self::TakenByTerminal => "keymap_fragile_taken",
            Self::AltGr => "keymap_fragile_altgr",
        }
    }
}

/// The first fragile input of `seq`, if any.
pub fn fragility(seq: &KeySeq) -> Option<Fragility> {
    seq.as_slice().iter().find_map(input_fragility)
}

fn input_fragility(input: &KeyInput) -> Option<Fragility> {
    let mods = input.mods();
    let ctrl = mods.contains(Mods::CTRL);
    let alt = mods.contains(Mods::ALT);
    match input.key() {
        Key::Char(c) if ctrl && alt && (c.is_ascii_digit() || matches!(c, 'e' | 'q' | 'm')) => {
            Some(Fragility::AltGr)
        }
        Key::Char(c) if ctrl && matches!(c, 'i' | 'm' | 'h' | '[') => {
            Some(Fragility::AliasesLegacyKey)
        }
        Key::Char(c) if ctrl && (c.is_uppercase() || c.is_ascii_digit()) => {
            Some(Fragility::NeedsKittyProtocol)
        }
        Key::Enter | Key::Tab | Key::Backspace if ctrl || mods.contains(Mods::SHIFT) => {
            Some(Fragility::NeedsKittyProtocol)
        }
        Key::F4 if mods.contains(Mods::ALT) => Some(Fragility::TakenByTerminal),
        Key::F11 if mods.is_empty() => Some(Fragility::TakenByTerminal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(s: &str) -> KeySeq {
        s.parse().unwrap()
    }

    #[test]
    fn leader_expands_in_both_spellings() {
        assert_eq!(normalize_chord("<leader> g b", Some("Space")), "Space g b");
        assert_eq!(normalize_chord("<Leader>gb", Some("Space")), "Space g b");
        assert_eq!(normalize_chord("<leader>?", Some("\\")), "\\ ?");
        assert_eq!(normalize_chord("<leader>x", None), "<leader>x");
        assert!(
            normalize_chord("<leader>x", None)
                .parse::<KeySeq>()
                .is_err()
        );
    }

    #[test]
    fn sequences_normalize_token_by_token() {
        assert_eq!(normalize_chord("Ctrl+Shift+w  h", None), "Ctrl+W h");
        assert_eq!(normalize_chord("g Gray+", None), "g Plus");
        assert!("g g".parse::<KeySeq>().is_ok());
    }

    #[test]
    fn fragile_chords_are_classified() {
        assert_eq!(fragility(&seq("Ctrl+i")), Some(Fragility::AliasesLegacyKey));
        assert_eq!(
            fragility(&seq("Ctrl+K")),
            Some(Fragility::NeedsKittyProtocol)
        );
        assert_eq!(
            fragility(&seq("Ctrl+1")),
            Some(Fragility::NeedsKittyProtocol)
        );
        assert_eq!(
            fragility(&seq("Ctrl+Enter")),
            Some(Fragility::NeedsKittyProtocol)
        );
        assert_eq!(fragility(&seq("Alt+F4")), Some(Fragility::TakenByTerminal));
        assert_eq!(
            fragility(&seq("g Ctrl+m")),
            Some(Fragility::AliasesLegacyKey)
        );
        assert_eq!(fragility(&seq("Ctrl+Alt+2")), Some(Fragility::AltGr));
        assert_eq!(fragility(&seq("Ctrl+Alt+e")), Some(Fragility::AltGr));
        assert_eq!(fragility(&seq("Ctrl+Alt+t")), None);
        for robust in [
            "Ctrl+k",
            "F5",
            "Alt+F7",
            "g g",
            "Shift+F6",
            "Ctrl+PageUp",
            "Space",
        ] {
            assert_eq!(fragility(&seq(robust)), None, "{robust}");
        }
    }
}
