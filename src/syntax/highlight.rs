//! Highlighting one line with a [`Grammar`], carrying the open regions to
//! the next line.
//!
//! Outside regions (or inside the innermost open one) the line is split at
//! the next region boundary: whichever comes first of the open region's
//! end and a sub-region's start. Each stretch takes the region's kind and
//! then its rules, in order, so later rules paint over earlier ones.
//!
//! Work per line is bounded: only the first [`MAX_LINE_BYTES`] are
//! analysed, and a line that needs more than [`MAX_SEARCHES`] pattern
//! searches keeps the rest plain.

use super::grammar::{Context, Grammar, Region, Rule};
use super::{LineTokens, SyntaxKind, Token};
use std::ops::Range;

/// Longer lines are highlighted up to here (minified files, data dumps).
pub const MAX_LINE_BYTES: usize = 4 * 1024;
/// Pattern searches allowed per line.
pub const MAX_SEARCHES: usize = 64 * 1024;

/// The regions open before a line (innermost last).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineState {
    stack: Vec<u16>,
}

/// One line being painted.
struct Painter<'a> {
    grammar: &'a Grammar,
    line: &'a str,
    paint: Vec<Option<SyntaxKind>>,
    /// Searches left before giving up on the line.
    budget: usize,
}

impl LineState {
    /// Tokens of `line` (without its line break); advances the state to
    /// the next line.
    pub fn advance(&mut self, grammar: &Grammar, line: &str) -> LineTokens {
        let line = &line[..line.floor_char_boundary(MAX_LINE_BYTES)];
        let mut painter = Painter {
            grammar,
            line,
            paint: vec![None; line.len()],
            budget: MAX_SEARCHES,
        };
        let mut pos = 0;
        while let Some(next) = painter.step(&self.stack, pos) {
            match next {
                Boundary::End(at) => {
                    self.stack.pop();
                    pos = at;
                }
                Boundary::Start(region, at) => {
                    self.stack.push(region);
                    pos = at;
                }
            }
        }
        painter.tokens()
    }
}

/// What ends a stretch of the line, and where the next one begins.
enum Boundary {
    End(usize),
    Start(u16, usize),
}

impl Painter<'_> {
    /// Paints from `pos` to the next region boundary and returns it
    /// (`None` at the end of the line or when out of budget).
    fn step(&mut self, stack: &[u16], pos: usize) -> Option<Boundary> {
        let grammar = self.grammar;
        let open = stack
            .last()
            .and_then(|&i| grammar.regions.get(usize::from(i)));
        let end = open.and_then(|r| self.find_end(r, pos));
        let limit = end.as_ref().map_or(self.line.len(), |e| e.start);
        let context = grammar.context(stack);
        let child = self.first_start(context, pos, limit);
        let stop = child.as_ref().map_or(limit, |(_, m)| m.start);
        self.fill(pos..stop, open.and_then(|r| r.kind));
        self.rules(context, pos..stop);
        if let Some((index, m)) = child {
            let kind = grammar.regions[usize::from(index)].kind;
            self.fill(m.clone(), kind);
            return Some(Boundary::Start(index, m.end));
        }
        let end = end?;
        self.fill(end.clone(), open.and_then(|r| r.kind));
        Some(Boundary::End(end.end))
    }

    /// One search of `regex` from `at`, or `None` once the budget is spent.
    fn search(&mut self, regex: &regex::Regex, at: usize) -> Option<Range<usize>> {
        self.budget = self.budget.checked_sub(1)?;
        regex.find_at(self.line, at).map(|m| m.range())
    }

    /// Where `region` ends from `pos` on, passing over its `skip` matches.
    fn find_end(&mut self, region: &Region, pos: usize) -> Option<Range<usize>> {
        let mut at = pos;
        loop {
            let end = self.search(&region.end, at)?;
            let skip = match &region.skip {
                Some(skip) => self.search(skip, at),
                None => None,
            };
            match skip {
                Some(s) if s.start < end.start && !s.is_empty() => at = s.end,
                _ => return Some(end),
            }
        }
    }

    /// The first sub-region of `context` starting in `pos..limit` (empty
    /// start matches are ignored; ties go to the first declared).
    fn first_start(
        &mut self,
        context: &Context,
        pos: usize,
        limit: usize,
    ) -> Option<(u16, Range<usize>)> {
        let mut best: Option<(u16, Range<usize>)> = None;
        for &index in &context.children {
            let bound = best.as_ref().map_or(limit, |(_, m)| m.start);
            let start = &self.grammar.regions[usize::from(index)].start;
            let mut at = pos;
            while let Some(m) = self.search(start, at).filter(|m| m.start < bound) {
                if !m.is_empty() {
                    best = Some((index, m));
                    break;
                }
                at = self.next_char(m.start)?;
            }
        }
        best
    }

    /// Paints the matches of `context`'s rules inside `span`.
    fn rules(&mut self, context: &Context, span: Range<usize>) {
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
        self.budget = self.budget.checked_sub(1)?;
        let caps = rule.regex.captures_at(self.line, at)?;
        let whole = caps.get(0)?.range();
        Some((whole, caps.get(rule.group).map(|m| m.range())))
    }

    /// The byte after the character at `at`.
    fn next_char(&self, at: usize) -> Option<usize> {
        let c = self.line.get(at..)?.chars().next()?;
        Some(at + c.len_utf8())
    }

    fn fill(&mut self, range: Range<usize>, kind: Option<SyntaxKind>) {
        if let Some(cells) = self.paint.get_mut(range) {
            cells.fill(kind);
        }
    }

    /// The painted cells as runs of one kind.
    fn tokens(self) -> LineTokens {
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
