//! A [`Definition`] with its patterns compiled, ready to highlight lines.
//!
//! Regions are flattened into one vector so the line state can name the
//! open ones by index (see `highlight.rs`).

use super::SyntaxKind;
use super::definition::{Definition, RegionDef, RuleDef};
use super::template::{EndCache, Template};
use regex::{Captures, Regex, RegexBuilder};
use std::sync::Arc;

/// Compiled size limit of one pattern: bounds the memory a user pattern
/// may take (the `regex` crate already guarantees linear-time matching).
const PATTERN_SIZE_LIMIT: usize = 1 << 20;
/// Deepest nesting of regions a definition may declare.
pub const MAX_DEPTH: usize = 8;

/// Rules and sub-regions that apply at one nesting level.
#[derive(Debug, Default)]
pub struct Context {
    pub rules: Vec<Rule>,
    /// Indices into [`Grammar::regions`].
    pub children: Vec<u16>,
}

#[derive(Debug)]
pub struct Rule {
    pub kind: SyntaxKind,
    pub regex: Regex,
    /// Capture group painted (0: the whole match).
    pub group: usize,
}

#[derive(Debug)]
pub struct Region {
    pub kind: Option<SyntaxKind>,
    pub start: Regex,
    /// The end (for a captured end, `$`: used only if the rendered end does
    /// not compile).
    pub end: Regex,
    /// An end built from the start's captures when the region opens.
    pub captured_end: Option<CapturedEnd>,
    pub skip: Option<Regex>,
    /// Pattern the text before the start must match.
    pub after: Option<Regex>,
    /// Language of the inner text.
    pub include: Option<Include>,
    pub inner: Context,
}

/// An `end` with `${N}` references.
#[derive(Debug)]
pub struct CapturedEnd {
    template: Template,
    case_insensitive: bool,
    cache: EndCache,
}

/// The language a region embeds: named, or taken from the start's
/// captures (the info string of a Markdown code block).
#[derive(Debug)]
pub enum Include {
    Named(String),
    Captured(Template),
}

impl Region {
    /// Whether opening the region needs the start's capture groups.
    pub fn uses_captures(&self) -> bool {
        self.captured_end.is_some() || matches!(self.include, Some(Include::Captured(_)))
    }

    /// The name of the embedded language for a start matched with `caps`.
    pub fn include_name(&self, caps: Option<&Captures>) -> Option<String> {
        match (&self.include, caps) {
            (Some(Include::Named(name)), _) => Some(name.clone()),
            (Some(Include::Captured(t)), Some(caps)) => Some(t.name(caps)),
            _ => None,
        }
    }
}

impl CapturedEnd {
    /// The end for a start matched with `caps` (compiled once per text).
    pub fn compile(&self, caps: &Captures) -> Option<Arc<Regex>> {
        self.cache.get(self.template.pattern(caps), |p| {
            build(p, self.case_insensitive).ok()
        })
    }
}

/// A compiled language.
#[derive(Debug, Default)]
pub struct Grammar {
    pub root: Context,
    pub regions: Vec<Region>,
}

/// Compiles patterns with the definition's case option.
struct Compiler {
    case_insensitive: bool,
    regions: Vec<Region>,
}

impl Grammar {
    /// Compiles every pattern of `def`; the error names the offending
    /// rule or region.
    pub fn compile(def: &Definition) -> Result<Self, String> {
        let mut compiler = Compiler {
            case_insensitive: def.case_insensitive,
            regions: Vec::new(),
        };
        let root = compiler.context(&def.rules, &def.regions, "", 0)?;
        Ok(Self {
            root,
            regions: compiler.regions,
        })
    }

    /// A grammar without rules or regions.
    pub const fn empty() -> Self {
        Self {
            root: Context {
                rules: Vec::new(),
                children: Vec::new(),
            },
            regions: Vec::new(),
        }
    }
}

/// Compiles `pattern` with the size limit of every pattern.
fn build(pattern: &str, case_insensitive: bool) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .size_limit(PATTERN_SIZE_LIMIT)
        .build()
}

impl Compiler {
    fn regex(&self, pattern: &str, at: &str) -> Result<Regex, String> {
        build(pattern, self.case_insensitive).map_err(|e| format!("{at}: {e}"))
    }

    fn context(
        &mut self,
        rules: &[RuleDef],
        regions: &[RegionDef],
        path: &str,
        depth: usize,
    ) -> Result<Context, String> {
        let rules = rules
            .iter()
            .enumerate()
            .map(|(i, r)| self.rule(r, &format!("{path}rule {}", i + 1)))
            .collect::<Result<_, _>>()?;
        let mut children = Vec::with_capacity(regions.len());
        for (i, def) in regions.iter().enumerate() {
            let at = format!("{path}region {}", i + 1);
            if depth >= MAX_DEPTH {
                return Err(format!("{at}: regions nested deeper than {MAX_DEPTH}"));
            }
            children.push(self.region(def, &at, depth + 1)?);
        }
        Ok(Context { rules, children })
    }

    fn rule(&self, def: &RuleDef, at: &str) -> Result<Rule, String> {
        let pattern = match (&def.pattern, def.words.is_empty()) {
            (Some(pattern), true) => pattern.clone(),
            (None, false) => words_pattern(&def.words),
            _ => return Err(format!("{at}: needs either `pattern` or `words`")),
        };
        let regex = self.regex(&pattern, at)?;
        if def.group >= regex.captures_len() {
            return Err(format!("{at}: the pattern has no group {}", def.group));
        }
        Ok(Rule {
            kind: def.kind,
            regex,
            group: def.group,
        })
    }

    fn region(&mut self, def: &RegionDef, at: &str, depth: usize) -> Result<u16, String> {
        let start = self.regex(&def.start, &format!("{at} start"))?;
        let groups = start.captures_len();
        let reference = |t: &Template, field: &str| {
            (t.max_group() < groups)
                .then_some(())
                .ok_or_else(|| format!("{at} {field}: the start has no group {}", t.max_group()))
        };
        let captured_end = Template::parse(&def.end);
        let end = match &captured_end {
            Some(t) => {
                reference(t, "end")?;
                self.regex(&t.sample("x"), &format!("{at} end"))?;
                self.regex("$", at)?
            }
            None => self.regex(&def.end, &format!("{at} end"))?,
        };
        let include = def
            .include
            .as_deref()
            .map(|name| match Template::parse(name) {
                Some(t) => reference(&t, "include").map(|()| Include::Captured(t)),
                None => Ok(Include::Named(name.to_string())),
            });
        let optional = |p: &Option<String>, field: &str| {
            p.as_deref()
                .map(|p| self.regex(p, &format!("{at} {field}")))
                .transpose()
        };
        let region = Region {
            kind: def.kind,
            start,
            end,
            captured_end: captured_end.map(|template| CapturedEnd {
                template,
                case_insensitive: self.case_insensitive,
                cache: EndCache::default(),
            }),
            skip: optional(&def.skip, "skip")?,
            after: optional(&def.after, "after")?,
            include: include.transpose()?,
            inner: Context::default(),
        };
        let index = u16::try_from(self.regions.len()).map_err(|_| format!("{at}: too many"))?;
        self.regions.push(region);
        let inner = self.context(&def.rules, &def.regions, &format!("{at} > "), depth)?;
        self.regions[usize::from(index)].inner = inner;
        Ok(index)
    }
}

/// A pattern matching any of `words` as a whole word (`\b` only on the
/// sides that are word characters, so `#include` or `@media` work too).
fn words_pattern(words: &[String]) -> String {
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    words
        .iter()
        .map(|w| {
            let lead = if is_word(w.chars().next()) { r"\b" } else { "" };
            let tail = if is_word(w.chars().last()) { r"\b" } else { "" };
            format!("{lead}{}{tail}", regex::escape(w))
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(text: &str) -> Result<Grammar, String> {
        Grammar::compile(&Definition::parse(text).unwrap())
    }

    #[test]
    fn words_match_whole_words_only() {
        let re = Regex::new(&words_pattern(&["if".into(), "#include".into()])).unwrap();
        assert!(re.is_match("x if y"));
        assert!(!re.is_match("iffy"));
        assert!(re.is_match("#include <a>"));
    }

    #[test]
    fn errors_name_the_rule_or_region() {
        let bad = compile("name = \"X\"\n[[region]]\nstart = 'a'\nend = '('");
        assert!(bad.unwrap_err().starts_with("region 1 end:"));
        let both = compile("name = \"X\"\n[[rule]]\nkind = \"tag\"");
        assert!(both.unwrap_err().contains("rule 1"));
        let group = compile("name = \"X\"\n[[rule]]\nkind = \"tag\"\npattern = 'a'\ngroup = 1");
        assert!(group.unwrap_err().contains("group 1"));
    }

    #[test]
    fn flattens_nested_regions() {
        let g = compile(
            "name = \"X\"\n[[region]]\nstart = 'a'\nend = 'b'\n\
             [[region.region]]\nstart = 'c'\nend = 'd'",
        )
        .unwrap();
        assert_eq!(g.regions.len(), 2);
        assert_eq!(g.root.children, vec![0]);
        assert_eq!(g.regions[0].inner.children, vec![1]);
    }

    #[test]
    fn captured_ends_reference_existing_groups() {
        let region = |extra: &str| compile(&format!("name = \"X\"\n[[region]]\n{extra}"));
        let ok = region("start = '<<(\\w+)'\nend = '^${1}$'").unwrap();
        let heredoc = &ok.regions[0];
        assert!(heredoc.uses_captures());
        let caps = heredoc.start.captures("<<EOF").unwrap();
        let end = heredoc
            .captured_end
            .as_ref()
            .unwrap()
            .compile(&caps)
            .unwrap();
        assert!(end.is_match("EOF") && !end.is_match("EOFX"));
        let missing = region("start = 'a'\nend = '${1}'").unwrap_err();
        assert!(missing.contains("region 1 end: the start has no group 1"));
        let include = region("start = 'a'\nend = 'b'\ninclude = '${2}'").unwrap_err();
        assert!(include.contains("include"));
        let after = region("start = 'a'\nend = 'b'\nafter = '('").unwrap_err();
        assert!(after.starts_with("region 1 after:"));
    }

    #[test]
    fn nesting_depth_is_bounded() {
        let mut text = String::from("name = \"X\"\n");
        let mut table = String::from("region");
        for _ in 0..=MAX_DEPTH {
            text.push_str(&format!("[[{table}]]\nstart = 'a'\nend = 'b'\n"));
            table.push_str(".region");
        }
        assert!(compile(&text).unwrap_err().contains("nested deeper"));
    }
}
