//! The searches and the paint of one line, within its search budget (see
//! `highlight.rs` for how the line is split into stretches).

use super::grammar::{Context, Grammar, Rule};
use super::{LineTokens, SyntaxKind, Token};
use regex::{Captures, Regex};
use std::ops::Range;

/// Pattern searches allowed per line.
pub const MAX_SEARCHES: usize = 64 * 1024;
/// How far back an `after` pattern looks (bytes before the start).
const AFTER_WINDOW: usize = 256;

/// One line being painted.
pub struct Painter<'a> {
    line: &'a str,
    paint: Vec<Option<SyntaxKind>>,
    /// Searches left before giving up on the line.
    budget: usize,
}

impl<'a> Painter<'a> {
    pub fn new(line: &'a str) -> Self {
        Self {
            line,
            paint: vec![None; line.len()],
            budget: MAX_SEARCHES,
        }
    }

    pub fn line_len(&self) -> usize {
        self.line.len()
    }

    /// Takes one search from the budget (`None` once it is spent).
    fn spend(&mut self) -> Option<()> {
        self.budget = self.budget.checked_sub(1)?;
        Some(())
    }

    /// One search of `regex` from `at`, or `None` once the budget is spent.
    pub fn search(&mut self, regex: &Regex, at: usize) -> Option<Range<usize>> {
        self.spend()?;
        regex.find_at(self.line, at).map(|m| m.range())
    }

    /// The groups of the match of `regex` starting at `at`.
    pub fn captures(&mut self, regex: &Regex, at: usize) -> Option<Captures<'a>> {
        self.spend()?;
        regex.captures_at(self.line, at)
    }

    /// Where a region ends from `pos` on (its `end`), passing over the
    /// matches of `skip`.
    pub fn find_end(
        &mut self,
        end: &Regex,
        skip: Option<&Regex>,
        pos: usize,
    ) -> Option<Range<usize>> {
        let mut at = pos;
        loop {
            let found = self.search(end, at)?;
            let skipped = match skip {
                Some(skip) => self.search(skip, at),
                None => None,
            };
            match skipped {
                Some(s) if s.start < found.start && !s.is_empty() => at = s.end,
                _ => return Some(found),
            }
        }
    }

    /// Whether the text before `at` satisfies `after` (always without one).
    fn preceded(&mut self, after: Option<&Regex>, at: usize) -> bool {
        let Some(after) = after else { return true };
        if self.spend().is_none() {
            return false;
        }
        let before = &self.line[..at];
        let from = before.floor_char_boundary(at.saturating_sub(AFTER_WINDOW));
        after.is_match_at(before, from)
    }

    /// The first sub-region of `context` starting in `pos..limit` (empty
    /// start matches and starts whose `after` fails are ignored; ties go to
    /// the first declared).
    pub fn first_start(
        &mut self,
        grammar: &Grammar,
        context: &Context,
        pos: usize,
        limit: usize,
    ) -> Option<(u16, Range<usize>)> {
        let mut best: Option<(u16, Range<usize>)> = None;
        for &index in &context.children {
            let Some(region) = grammar.regions.get(usize::from(index)) else {
                continue;
            };
            let bound = best.as_ref().map_or(limit, |(_, m)| m.start);
            let mut at = pos;
            while let Some(m) = self.search(&region.start, at).filter(|m| m.start < bound) {
                if !m.is_empty() && self.preceded(region.after.as_ref(), m.start) {
                    best = Some((index, m));
                    break;
                }
                let Some(next) = self.next_char(m.start) else {
                    break;
                };
                at = next;
            }
        }
        best
    }

    /// Paints the matches of `context`'s rules inside `span`.
    pub fn rules(&mut self, context: &Context, span: Range<usize>) {
        for rule in &context.rules {
            let mut at = span.start;
            while at < span.end {
                let Some((whole, painted)) = self.rule_match(rule, at) else {
                    break;
                };
                if whole.start >= span.end {
                    break;
                }
                if let Some(m) = painted {
                    self.fill(m.start..m.end.min(span.end), Some(rule.kind));
                }
                at = if whole.is_empty() {
                    self.next_char(whole.start).unwrap_or(span.end)
                } else {
                    whole.end
                };
            }
        }
    }

    /// The next match of `rule` from `at`: the whole match and the part
    /// painted (its group, which may not have taken part).
    fn rule_match(
        &mut self,
        rule: &Rule,
        at: usize,
    ) -> Option<(Range<usize>, Option<Range<usize>>)> {
        if rule.group == 0 {
            let whole = self.search(&rule.regex, at)?;
            return Some((whole.clone(), Some(whole)));
        }
        let caps = self.captures(&rule.regex, at)?;
        let whole = caps.get(0)?.range();
        Some((whole, caps.get(rule.group).map(|m| m.range())))
    }

    /// The byte after the character at `at`.
    fn next_char(&self, at: usize) -> Option<usize> {
        let c = self.line.get(at..)?.chars().next()?;
        Some(at + c.len_utf8())
    }

    pub fn fill(&mut self, range: Range<usize>, kind: Option<SyntaxKind>) {
        if let Some(cells) = self.paint.get_mut(range) {
            cells.fill(kind);
        }
    }

    /// The painted cells as runs of one kind.
    pub fn tokens(self) -> LineTokens {
        let mut tokens = LineTokens::new();
        for (at, kind) in self.paint.into_iter().enumerate() {
            let Some(kind) = kind else { continue };
            match tokens.last_mut() {
                Some(last) if last.kind == kind && last.range.end == at => last.range.end = at + 1,
                _ => tokens.push(Token {
                    range: at..at + 1,
                    kind,
                }),
            }
        }
        tokens
    }
}
