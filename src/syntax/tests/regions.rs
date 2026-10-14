//! The engine features of regions: ends built from the start's captures,
//! `after` conditions and embedded languages (with their bounds).

use super::{check, grammar, run, run_in};
use crate::syntax::SyntaxKind::{Comment, Constant, Keyword, String as Str, Tag};
use crate::syntax::definition::Definition;
use crate::syntax::grammar::Grammar;
use crate::syntax::highlight::{Grammars, LineState, MAX_EMBED_DEPTH};
use crate::syntax::language::Language;
use crate::syntax::registry::Registry;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn language(text: &str) -> Arc<Language> {
    Arc::new(Language::new(Definition::parse(text).expect("definition")).expect("language"))
}

const HEREDOC: &str = r#"
name = "T"
[[rule]]
kind = "keyword"
words = ["if"]
[[region]]
kind = "string"
start = '<<(\S+)'
end = '^${1}$'
[[region]]
kind = "comment"
start = '#'
end = '$'
"#;

#[test]
fn captured_ends_match_the_start_literally() {
    let g = grammar(HEREDOC);
    let lines = ["x <<A.B", "A.B2", "AxB", "A.B", "if", "<<Z", "Z", "if"];
    let toks = run(&g, &lines);
    check(
        "heredoc",
        &lines,
        &toks,
        &[
            (0, "<<", Some(Str)),
            (1, "A", Some(Str)),
            (2, "AxB", Some(Str)),
            (3, "A.B", Some(Str)),
            (4, "if", Some(Keyword)),
            (6, "Z", Some(Str)),
            (7, "if", Some(Keyword)),
        ],
    );
}

#[test]
fn after_limits_where_a_region_starts() {
    let g = grammar(
        r#"
        name = "T"
        [[region]]
        kind = "string"
        start = '/'
        end = '/|$'
        after = '(?:^|[(=])\s*$'
        "#,
    );
    let lines = ["a = /x/ / b / c", "/y/", "f(/z/)"];
    let toks = run(&g, &lines);
    check(
        "after",
        &lines,
        &toks,
        &[
            (0, "x", Some(Str)),
            (0, " b", None),
            (0, "b", None),
            (1, "y", Some(Str)),
            (2, "z", Some(Str)),
        ],
    );
    // A pattern that may need the line start sees the whole line.
    let indented = format!("{}/w/", " ".repeat(1_000));
    let toks = run(&g, &[&indented]);
    assert_eq!(super::kind_at(&toks[0], &indented, "w"), Some(Str));
    let far = format!("a = {}/w/", " ".repeat(1_000));
    let toks = run(&g, &[&far]);
    assert_eq!(super::kind_at(&toks[0], &far, "w"), Some(Str));
}

#[test]
fn unanchored_after_patterns_look_only_shortly_back() {
    let g = grammar(
        r#"
        name = "T"
        [[region]]
        kind = "string"
        start = '/'
        end = '/|$'
        after = '[(=]\s*$'
        "#,
    );
    let near = "a = /w/";
    let toks = run(&g, &[near]);
    assert_eq!(super::kind_at(&toks[0], near, "w"), Some(Str));
    let far = format!("a = {}/w/", " ".repeat(1_000));
    let toks = run(&g, &[&far]);
    assert_eq!(super::kind_at(&toks[0], &far, "w"), None);
}

const OUTER: &str = r#"
name = "Outer"
extensions = ["outer"]
[[region]]
kind = "tag"
start = '<x>'
end = '</x>'
include = "Inner"
[[region]]
kind = "tag"
start = '<(\w*)>'
end = '</>'
include = '${1}'
[[region.rule]]
kind = "constant"
pattern = 'k'
"#;

const INNER: &str = r#"
name = "Inner"
extensions = ["in"]
[[rule]]
kind = "keyword"
words = ["k"]
[[region]]
kind = "string"
start = '"'
end = '"'
[[region]]
kind = "comment"
start = '/\*'
end = '\*/'
"#;

fn outer_registry() -> Registry {
    Registry::new(vec![language(OUTER), language(INNER)])
}

#[test]
fn included_languages_highlight_the_inner_text() {
    let registry = outer_registry();
    let outer = registry.embedded("outer").expect("by name, any case");
    let lines = [
        "k <x>k \"s",
        "s</x> k",
        "<x>/* c",
        "c */ k</x>",
        "<in>k</> <>k</>",
    ];
    let toks = run_in(outer.grammar().unwrap(), &registry, &lines);
    check(
        "include",
        &lines,
        &toks,
        &[
            (0, "k ", None),
            (0, "<x>", Some(Tag)),
            (0, "k \"", Some(Keyword)),
            (0, "\"s", Some(Str)),
            (1, "s<", Some(Str)),
            (1, "</x>", Some(Tag)),
            (1, " k", None),
            (2, "/* c", Some(Comment)),
            (3, "c */", Some(Comment)),
            (3, "k<", Some(Keyword)),
            (4, "k</>", Some(Keyword)),
            (4, "<>", Some(Tag)),
        ],
    );
    let fallback = &toks[4];
    let at = lines[4].rfind('k').unwrap();
    let kind = fallback
        .iter()
        .find(|t| t.range.contains(&at))
        .map(|t| t.kind);
    assert_eq!(kind, Some(Constant), "unknown language: the region's rules");
}

#[test]
fn embedding_is_bounded() {
    let nest = language(
        "name = \"Nest\"\n[[region]]\nkind = \"tag\"\nstart = '<'\nend = '>'\ninclude = \"Nest\"",
    );
    let registry = Registry::new(vec![Arc::clone(&nest)]);
    let grammars = Grammars {
        root: nest.grammar().unwrap(),
        registry: &registry,
    };
    let mut state = LineState::default();
    state.advance(grammars, &"<".repeat(1_000));
    assert_eq!(state.depth(), MAX_EMBED_DEPTH + 1);
    state.advance(grammars, &">".repeat(1_000));
    assert_eq!(state.depth(), 0);
}

#[test]
fn line_states_compare_by_open_regions() {
    let registry = outer_registry();
    let outer = registry.embedded("Outer").unwrap();
    let grammars = Grammars {
        root: outer.grammar().unwrap(),
        registry: &registry,
    };
    let (mut a, mut b) = (LineState::default(), LineState::default());
    a.advance(grammars, "<x>\"");
    b.advance(grammars, "<x> \"");
    assert_eq!(a, b);
    b.advance(grammars, "\"");
    assert_ne!(a, b);
}

/// Highlights `lines` with `g`, asserting it takes little time.
fn bounded(g: &Grammar, registry: &Registry, lines: &[&str]) {
    let started = Instant::now();
    run_in(g, registry, lines);
    let took = started.elapsed();
    assert!(took < Duration::from_secs(5), "took {took:?}");
}

#[test]
fn pathological_delimiters_stay_bounded() {
    // Every heredoc word different (the cache of ends keeps turning over),
    // and lines full of region starts.
    let g = grammar(HEREDOC);
    let words: String = (0..2_000).map(|i| format!("<<w{i}\nw{i}\n")).collect();
    let lines: Vec<&str> = words.lines().collect();
    bounded(&g, &Registry::new(Vec::new()), &lines);
    let registry = outer_registry();
    let outer = registry.embedded("Outer").unwrap();
    let starts = "<x><in><Outer><x>\"".repeat(500);
    bounded(outer.grammar().unwrap(), &registry, &[&starts, &starts]);
}
