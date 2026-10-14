//! Tests of the highlighter as a whole: the line engine and every
//! built-in language.

mod delimiters;
mod embedded;
mod engine;
mod languages;
mod regions;
mod samples;

use super::definition::Definition;
use super::grammar::Grammar;
use super::highlight::{Grammars, LineState};
use super::registry::Registry;
use super::{LineTokens, SyntaxKind};

/// The built-in languages, for the regions that embed one.
fn builtins() -> &'static Registry {
    static REGISTRY: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| Registry::new(Vec::new()))
}

/// Tokens of `lines` highlighted in order with `grammar`.
fn run(grammar: &Grammar, lines: &[&str]) -> Vec<LineTokens> {
    run_in(grammar, builtins(), lines)
}

/// [`run`] with the languages of `registry` to embed.
fn run_in(grammar: &Grammar, registry: &Registry, lines: &[&str]) -> Vec<LineTokens> {
    let mut state = LineState::default();
    let grammars = Grammars {
        root: grammar,
        registry,
    };
    lines.iter().map(|l| state.advance(grammars, l)).collect()
}

/// The kinds expected at some needles: `(line index, needle, kind)`.
type Expect<'a> = &'a [(usize, &'a str, Option<SyntaxKind>)];

/// Highlights `lines` as a file named `path` (detected among the built-in
/// languages, starting in its host's grammar as the editor does) and
/// checks `expect`.
fn check_file(path: &str, lines: &[&str], expect: Expect) {
    let registry = builtins();
    let language = registry
        .detect(std::path::Path::new(path), lines[0])
        .unwrap_or_else(|| panic!("{path} not detected"));
    let root = registry.root_of(&language);
    let tokens = run_in(root.grammar().expect("grammar"), registry, lines);
    check(path, lines, &tokens, expect);
}

/// Asserts the kinds of `expect` on `tokens` of `lines`.
fn check(what: &str, lines: &[&str], tokens: &[LineTokens], expect: Expect) {
    for &(line, needle, kind) in expect {
        assert_eq!(
            kind_at(&tokens[line], lines[line], needle),
            kind,
            "{what}: {needle:?} in {:?}",
            lines[line]
        );
    }
}

/// The kind painted on the first byte of `needle` in `line`.
fn kind_at(tokens: &LineTokens, line: &str, needle: &str) -> Option<SyntaxKind> {
    let at = line
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} not in {line:?}"));
    tokens
        .iter()
        .find(|t| t.range.contains(&at))
        .map(|t| t.kind)
}

fn grammar(text: &str) -> Grammar {
    Grammar::compile(&Definition::parse(text).expect("definition")).expect("grammar")
}
