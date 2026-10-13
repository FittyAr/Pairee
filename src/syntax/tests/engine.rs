//! The line engine: rules, regions, state carried between lines and the
//! bounds on the work per line.

use super::{grammar, kind_at, run};
use crate::syntax::SyntaxKind::{self, Comment, Constant, Function, Keyword, String as Str};
use crate::syntax::highlight::{LineState, MAX_LINE_BYTES};
use std::time::{Duration, Instant};

const C_LIKE: &str = r#"
name = "T"
[[rule]]
kind = "function"
pattern = '\b(\w+)\('
group = 1
[[rule]]
kind = "keyword"
words = ["if", "return"]
[[region]]
kind = "comment"
start = '/\*'
end = '\*/'
[[region]]
kind = "string"
start = '"'
end = '"'
skip = '\\.'
[[region.rule]]
kind = "constant"
pattern = '\\.'
[[region]]
kind = "constant"
start = "'.'"
end = ''
"#;

#[test]
fn regions_carry_over_lines() {
    let g = grammar(C_LIKE);
    let lines = ["x /* open", "still */ if y", "\"a", "b\" return"];
    let toks = run(&g, &lines);
    assert_eq!(kind_at(&toks[0], lines[0], "x"), None);
    assert_eq!(kind_at(&toks[0], lines[0], "open"), Some(Comment));
    assert_eq!(kind_at(&toks[1], lines[1], "still"), Some(Comment));
    assert_eq!(kind_at(&toks[1], lines[1], "if"), Some(Keyword));
    assert_eq!(kind_at(&toks[2], lines[2], "a"), Some(Str));
    assert_eq!(kind_at(&toks[3], lines[3], "b"), Some(Str));
    assert_eq!(kind_at(&toks[3], lines[3], "return"), Some(Keyword));
}

#[test]
fn skips_escapes_and_paints_inner_rules() {
    let g = grammar(C_LIKE);
    let line = r#"f("a\"b") if"#;
    let toks = &run(&g, &[line])[0];
    assert_eq!(kind_at(toks, line, "f("), Some(Function));
    assert_eq!(kind_at(toks, line, "("), None, "group 1 only");
    assert_eq!(kind_at(toks, line, "\\\""), Some(Constant));
    assert_eq!(kind_at(toks, line, "b\""), Some(Str));
    assert_eq!(kind_at(toks, line, " if"), None);
    assert_eq!(kind_at(toks, line, "if"), Some(Keyword));
}

#[test]
fn single_token_regions_shield_quotes() {
    let g = grammar(C_LIKE);
    let line = r#"c = '"'; if"#;
    let toks = &run(&g, &[line])[0];
    assert_eq!(kind_at(toks, line, "'\"'"), Some(Constant));
    assert_eq!(kind_at(toks, line, "if"), Some(Keyword), "no string opened");
}

#[test]
fn later_rules_paint_over_earlier_ones() {
    let g = grammar(
        "name = \"T\"\n[[rule]]\nkind = \"function\"\npattern = '\\w+'\n\
         [[rule]]\nkind = \"keyword\"\nwords = [\"fn\"]",
    );
    let line = "fn main";
    let toks = &run(&g, &[line])[0];
    assert_eq!(kind_at(toks, line, "fn"), Some(Keyword));
    assert_eq!(kind_at(toks, line, "main"), Some(Function));
}

#[test]
fn nested_regions_close_innermost_first() {
    let g = grammar(
        "name = \"T\"\n[[region]]\nkind = \"string\"\nstart = '`'\nend = '`'\n\
         [[region.region]]\nkind = \"constant\"\nstart = '\\$\\{'\nend = '\\}'",
    );
    let lines = ["`a ${b", "c} d` e"];
    let toks = run(&g, &lines);
    assert_eq!(kind_at(&toks[0], lines[0], "a"), Some(Str));
    assert_eq!(kind_at(&toks[0], lines[0], "b"), Some(Constant));
    assert_eq!(kind_at(&toks[1], lines[1], "c"), Some(Constant));
    assert_eq!(kind_at(&toks[1], lines[1], "d"), Some(Str));
    assert_eq!(kind_at(&toks[1], lines[1], "e"), None);
}

#[test]
fn tokens_are_ordered_and_on_char_boundaries() {
    let g = grammar(C_LIKE);
    let line = "if \"ñandú 🎉\" /* é */ return";
    let toks = &run(&g, &[line])[0];
    assert!(toks.windows(2).all(|w| w[0].range.end <= w[1].range.start));
    for t in toks {
        assert!(line.is_char_boundary(t.range.start) && line.is_char_boundary(t.range.end));
    }
    assert_eq!(kind_at(toks, line, "é"), Some(Comment));
}

/// Highlights `line` with `g`, asserting it takes little time.
fn bounded(g: &crate::syntax::grammar::Grammar, line: &str) -> Vec<crate::syntax::Token> {
    let started = Instant::now();
    let tokens = LineState::default().advance(g, line);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
    tokens
}

#[test]
fn long_lines_are_analysed_up_to_the_cap() {
    let g = grammar(C_LIKE);
    let line = "é if ".repeat(200_000);
    let toks = bounded(&g, &line);
    assert!(toks.iter().all(|t| t.range.end <= MAX_LINE_BYTES));
    assert_eq!(toks[0].kind, Keyword);
}

#[test]
fn pathological_patterns_stay_bounded() {
    // Empty matches everywhere, nested quantifiers and a region whose start
    // can match the empty string.
    let g = grammar(
        r#"
        name = "T"
        [[rule]]
        kind = "keyword"
        pattern = '\b'
        [[rule]]
        kind = "constant"
        pattern = '(a*)*b'
        [[rule]]
        kind = "function"
        pattern = '(?:a|aa)+$'
        [[region]]
        kind = "string"
        start = 'x*'
        end = 'y*'
        "#,
    );
    let toks = bounded(&g, &"a".repeat(100_000));
    assert!(toks.iter().all(|t| t.range.end <= MAX_LINE_BYTES));
    let toks = bounded(&g, &"xa".repeat(50_000));
    assert!(toks.iter().all(|t| t.range.end <= MAX_LINE_BYTES));
}

#[test]
fn every_kind_has_a_definition_name() {
    for kind in SyntaxKind::ALL {
        let name = match kind {
            SyntaxKind::TypeName => "type_name".to_string(),
            other => format!("{other:?}").to_lowercase(),
        };
        let text = format!("name = \"T\"\n[[rule]]\nkind = \"{name}\"\npattern = 'a'");
        let toks = run(&grammar(&text), &["a"]);
        assert_eq!(toks[0][0].kind, kind);
    }
}
