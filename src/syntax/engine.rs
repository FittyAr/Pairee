//! The syntect side: the embedded grammars, the scope → [`SyntaxKind`]
//! mapping and the parsing of one line.
//!
//! The mapping is a syntect theme built in code whose "colors" are kind
//! numbers, so syntect's own selector scoring decides which kind wins for
//! nested scopes (`keyword.operator` inside `keyword`, ...).

use super::{LineTokens, SyntaxKind, Token};
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;
use syntect::highlighting::{
    Color, HighlightState, Highlighter, RangedHighlightIterator, ScopeSelectors, StyleModifier,
    Theme, ThemeItem,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

/// Scope selectors of each kind (Sublime Text / TextMate scope names).
const SELECTORS: [(SyntaxKind, &str); 8] = [
    (
        SyntaxKind::Comment,
        "comment, punctuation.definition.comment",
    ),
    (SyntaxKind::String, "string, markup.raw"),
    (
        SyntaxKind::Keyword,
        "keyword - keyword.operator, storage, variable.language",
    ),
    (
        SyntaxKind::Constant,
        "constant, support.constant, string constant.character.escape",
    ),
    (
        SyntaxKind::Function,
        "entity.name.function, support.function, variable.function, \
         meta.function-call.identifier, entity.other.attribute-name, meta.annotation",
    ),
    (
        SyntaxKind::TypeName,
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, \
         entity.name.trait, entity.name.namespace, entity.other.inherited-class, \
         support.type, support.class, storage.type.primitive",
    ),
    (SyntaxKind::Operator, "keyword.operator"),
    (
        SyntaxKind::Tag,
        "entity.name.tag, markup.heading, entity.name.section, markup.underline.link",
    ),
];

/// The embedded grammars (loaded on first use).
pub fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// The kind encoded in a theme color (`None` for the default color).
fn kind_of(color: Color) -> Option<SyntaxKind> {
    SyntaxKind::ALL
        .get(usize::from(color.r).checked_sub(1)?)
        .copied()
}

/// The color that stands for `kind` in [`kind_theme`].
fn color_of(kind: SyntaxKind) -> Color {
    let index = SyntaxKind::ALL
        .iter()
        .position(|k| *k == kind)
        .unwrap_or_default();
    Color {
        r: u8::try_from(index + 1).unwrap_or(u8::MAX),
        g: 0,
        b: 0,
        a: u8::MAX,
    }
}

fn kind_theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| Theme {
        scopes: SELECTORS
            .iter()
            .filter_map(|(kind, selector)| {
                Some(ThemeItem {
                    scope: ScopeSelectors::from_str(selector).ok()?,
                    style: StyleModifier {
                        foreground: Some(color_of(*kind)),
                        ..StyleModifier::default()
                    },
                })
            })
            .collect(),
        ..Theme::default()
    })
}

fn highlighter() -> &'static Highlighter<'static> {
    static HIGHLIGHTER: OnceLock<Highlighter<'static>> = OnceLock::new();
    HIGHLIGHTER.get_or_init(|| Highlighter::new(kind_theme()))
}

/// The grammar for `path` (by file name, extension, then first line), or
/// `None` for plain text.
pub fn detect(path: &Path, first_line: &str) -> Option<&'static SyntaxReference> {
    let set = syntax_set();
    let by_name = |s: Option<&std::ffi::OsStr>| {
        s.and_then(|s| s.to_str())
            .and_then(|s| set.find_syntax_by_extension(s))
    };
    by_name(path.file_name())
        .or_else(|| by_name(path.extension()))
        .or_else(|| set.find_syntax_by_first_line(first_line))
        .filter(|syntax| syntax.name != set.find_syntax_plain_text().name)
}

/// Parser and highlighter state before some line.
#[derive(Debug, Clone)]
pub struct LineState {
    parse: ParseState,
    highlight: HighlightState,
}

impl LineState {
    /// The state at the start of a file in `syntax`.
    pub fn start(syntax: &SyntaxReference) -> Self {
        Self {
            parse: ParseState::new(syntax),
            highlight: HighlightState::new(highlighter(), ScopeStack::new()),
        }
    }

    /// Tokens of `line` (without its line break); advances the state to the
    /// next line. `buf` is scratch space reused between lines.
    pub fn advance(&mut self, line: &str, buf: &mut String) -> LineTokens {
        buf.clear();
        buf.push_str(line);
        buf.push('\n');
        let Ok(ops) = self.parse.parse_line(buf, syntax_set()) else {
            return LineTokens::new();
        };
        let mut tokens = LineTokens::new();
        let ranges = RangedHighlightIterator::new(&mut self.highlight, &ops, buf, highlighter());
        for (style, _, range) in ranges {
            let end = range.end.min(line.len());
            let Some(kind) = kind_of(style.foreground).filter(|_| range.start < end) else {
                continue;
            };
            match tokens.last_mut() {
                Some(last) if last.kind == kind && last.range.end == range.start => {
                    last.range.end = end;
                }
                _ => tokens.push(Token {
                    range: range.start..end,
                    kind,
                }),
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(path: &str, lines: &[&str]) -> Vec<LineTokens> {
        let syntax = detect(Path::new(path), lines[0]).expect("grammar");
        let mut state = LineState::start(syntax);
        let mut buf = String::new();
        lines.iter().map(|l| state.advance(l, &mut buf)).collect()
    }

    fn kind_at(tokens: &LineTokens, line: &str, needle: &str) -> Option<SyntaxKind> {
        let at = line.find(needle).expect("needle");
        tokens
            .iter()
            .find(|t| t.range.contains(&at))
            .map(|t| t.kind)
    }

    #[test]
    fn selectors_parse_and_kinds_roundtrip() {
        assert_eq!(kind_theme().scopes.len(), SELECTORS.len());
        for kind in SyntaxKind::ALL {
            assert_eq!(kind_of(color_of(kind)), Some(kind));
        }
        assert_eq!(kind_of(Color::BLACK), None);
    }

    #[test]
    fn maps_rust_scopes_to_kinds() {
        let line = r#"fn main() { let x = 42 + 1; println!("hi\n"); } // done"#;
        let toks = &tokens("main.rs", &[line])[0];
        assert_eq!(kind_at(toks, line, "fn"), Some(SyntaxKind::Keyword));
        assert_eq!(kind_at(toks, line, "main"), Some(SyntaxKind::Function));
        assert_eq!(kind_at(toks, line, "42"), Some(SyntaxKind::Constant));
        assert_eq!(kind_at(toks, line, "+"), Some(SyntaxKind::Operator));
        assert_eq!(kind_at(toks, line, "\"hi"), Some(SyntaxKind::String));
        assert_eq!(kind_at(toks, line, "\\n"), Some(SyntaxKind::Constant));
        assert_eq!(kind_at(toks, line, "// done"), Some(SyntaxKind::Comment));
        assert!(toks.iter().all(|t| t.range.end <= line.len()));
        assert!(toks.windows(2).all(|w| w[0].range.end <= w[1].range.start));
    }

    #[test]
    fn state_carries_across_lines() {
        let lines = ["/* open", "still comment */ int x;"];
        let toks = tokens("a.c", &lines);
        assert_eq!(
            kind_at(&toks[1], lines[1], "still"),
            Some(SyntaxKind::Comment)
        );
        assert_eq!(
            kind_at(&toks[1], lines[1], "int"),
            Some(SyntaxKind::Keyword)
        );
    }

    #[test]
    fn detects_by_name_extension_and_shebang() {
        assert_eq!(detect(Path::new("Makefile"), "").unwrap().name, "Makefile");
        assert_eq!(detect(Path::new("x.PY"), "").unwrap().name, "Python");
        assert_eq!(
            detect(Path::new("script"), "#!/bin/bash").unwrap().name,
            "Bourne Again Shell (bash)"
        );
        assert!(detect(Path::new("notes.txt"), "hello").is_none());
        assert!(detect(Path::new("unknown.zzz"), "plain").is_none());
    }

    /// Every embedded grammar parses ordinary text with the fancy-regex
    /// engine (a pattern it cannot compile would panic here, not on screen).
    #[test]
    fn every_grammar_parses_sample_text() {
        let sample = [
            "#!/usr/bin/env x",
            "<a href=\"x\">{{ y }}</a> /* c */ // d # e -- f",
            "fn f(a: i32) -> String { \"s\\n\" + 'c' + 0x1F + 3.5e-2 }",
            "  key: [1, 2, {k = v}] ; @attr $var %{x} `cmd` '''doc'''",
        ];
        let mut buf = String::new();
        for syntax in syntax_set().syntaxes() {
            let mut state = LineState::start(syntax);
            for line in sample {
                state.advance(line, &mut buf);
            }
        }
    }
}
