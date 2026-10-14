//! A [`Definition`] with its patterns compiled, ready to highlight lines.
//!
//! Regions are flattened into one vector so the line state can name the
//! open ones by index (see `highlight.rs`).

use super::SyntaxKind;
use super::definition::{Definition, RegionDef, RuleDef};
use regex::{Regex, RegexBuilder};

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
    pub end: Regex,
    pub skip: Option<Regex>,
    pub inner: Context,
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

    /// The context of the innermost open region in `stack` (the root when
    /// none is open).
    pub fn context(&self, stack: &[u16]) -> &Context {
        stack
            .last()
            .and_then(|&i| self.regions.get(usize::from(i)))
            .map_or(&self.root, |r| &r.inner)
    }
}

impl Compiler {
    fn regex(&self, pattern: &str, at: &str) -> Result<Regex, String> {
        RegexBuilder::new(pattern)
            .case_insensitive(self.case_insensitive)
            .size_limit(PATTERN_SIZE_LIMIT)
            .build()
            .map_err(|e| format!("{at}: {e}"))
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
        let region = Region {
            kind: def.kind,
            start: self.regex(&def.start, &format!("{at} start"))?,
            end: self.regex(&def.end, &format!("{at} end"))?,
            skip: def
                .skip
                .as_deref()
                .map(|s| self.regex(s, &format!("{at} skip")))
                .transpose()?,
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
        assert_eq!(g.context(&[0]).children, vec![1]);
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
