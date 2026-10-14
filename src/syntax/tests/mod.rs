//! Tests of the highlighter as a whole: the line engine and every
//! built-in language.

mod engine;
mod languages;
mod samples;

use super::definition::Definition;
use super::grammar::Grammar;
use super::highlight::LineState;
use super::{LineTokens, SyntaxKind};

/// Tokens of `lines` highlighted in order with `grammar`.
fn run(grammar: &Grammar, lines: &[&str]) -> Vec<LineTokens> {
    let mut state = LineState::default();
    lines.iter().map(|l| state.advance(grammar, l)).collect()
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
