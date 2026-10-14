//! Highlighting one line with a [`Grammar`], carrying the open regions to
//! the next line.
//!
//! Outside regions (or inside the innermost open one) the line is split at
//! the next region boundary: whichever comes first of the open region's
//! end, the end of an enclosing region that embeds another language, and
//! a sub-region's start. Each stretch takes the region's kind and then its
//! rules, in order, so later rules paint over earlier ones.
//!
//! A region that embeds a language (`include`) highlights its inner text
//! with that language's grammar; its own end still closes it whatever the
//! embedded language left open (a `</script>` inside a JavaScript string).
//!
//! Work per line is bounded: only the first [`MAX_LINE_BYTES`] are
//! analysed, a line that needs more than `MAX_SEARCHES` pattern searches
//! keeps the rest plain, and languages embed at most [`MAX_EMBED_DEPTH`]
//! deep.

use super::LineTokens;
use super::grammar::{Context, Grammar, Region};
use super::language::Language;
use super::painter::Painter;
use super::registry::Registry;
use regex::Regex;
use std::ops::Range;
use std::sync::Arc;

/// Longer lines are highlighted up to here (minified files, data dumps).
pub const MAX_LINE_BYTES: usize = 4 * 1024;
/// Deepest nesting of embedded languages (HTML > PHP > …).
pub const MAX_EMBED_DEPTH: usize = 4;

/// The grammar of a language whose grammar did not compile (never embedded,
/// but a frame must still name one).
static EMPTY: Grammar = Grammar::empty();

/// One open region.
#[derive(Debug, Clone)]
struct Frame {
    /// Language declaring the region (`None`: the root grammar).
    owner: Option<Arc<Language>>,
    region: u16,
    /// End built from the start's captures.
    end: Option<Arc<Regex>>,
    /// Language of the inner text.
    embed: Option<Arc<Language>>,
}

impl PartialEq for Frame {
    fn eq(&self, other: &Self) -> bool {
        fn same<T>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }
        self.region == other.region
            && same(&self.owner, &other.owner)
            && same(&self.embed, &other.embed)
            && self.end.as_ref().map(|r| r.as_str()) == other.end.as_ref().map(|r| r.as_str())
    }
}

impl Eq for Frame {}

/// The regions open before a line (innermost last).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineState {
    stack: Vec<Frame>,
}

/// What lines are highlighted with: the file's grammar and the languages
/// regions may embed.
#[derive(Clone, Copy)]
pub struct Grammars<'a> {
    pub root: &'a Grammar,
    pub registry: &'a Registry,
}

/// What ends a stretch of the line, and where the next one begins.
enum Boundary {
    /// The frame at this depth (and every one inside it) closes.
    End(usize, usize),
    Start(Frame, usize),
}

impl LineState {
    /// Tokens of `line` (without its line break); advances the state to
    /// the next line.
    pub fn advance(&mut self, grammars: Grammars, line: &str) -> LineTokens {
        let cut = line.floor_char_boundary(MAX_LINE_BYTES);
        let truncated = cut < line.len();
        let line = &line[..cut];
        let mut painter = Painter::new(line);
        let mut pos = 0;
        // The shallowest the stack got: regions opened past it on this
        // line may close in the part never analysed.
        let mut settled = self.stack.len();
        while let Some(next) = grammars.step(&mut painter, &self.stack, pos) {
            match next {
                Boundary::End(depth, at) => {
                    self.stack.truncate(depth);
                    settled = settled.min(depth);
                    pos = at;
                }
                Boundary::Start(frame, at) => {
                    self.stack.push(frame);
                    pos = at;
                }
            }
        }
        // A cut line or an exhausted budget hid where the regions it opened
        // end: carrying them on would turn every following line inside
        // out (a string opened in a long base64 line), so they close here.
        if truncated || painter.exhausted() {
            self.stack.truncate(settled);
        }
        painter.tokens()
    }

    /// How many regions are open.
    #[cfg(test)]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl<'g> Grammars<'g> {
    fn grammar(&self, language: Option<&'g Arc<Language>>) -> &'g Grammar {
        match language {
            Some(language) => language.grammar().unwrap_or(&EMPTY),
            None => self.root,
        }
    }

    fn region(&self, frame: &'g Frame) -> Option<&'g Region> {
        let grammar = self.grammar(frame.owner.as_ref());
        grammar.regions.get(usize::from(frame.region))
    }

    /// The language, grammar and context that apply inside `stack`.
    fn context(&self, stack: &'g [Frame]) -> (Option<&'g Arc<Language>>, &'g Grammar, &'g Context) {
        let Some(frame) = stack.last() else {
            return (None, self.root, &self.root.root);
        };
        if let Some(embed) = &frame.embed {
            let grammar = self.grammar(Some(embed));
            return (Some(embed), grammar, &grammar.root);
        }
        let owner = frame.owner.as_ref();
        let inner = self.region(frame).map_or(&EMPTY.root, |r| &r.inner);
        (owner, self.grammar(owner), inner)
    }

    /// Paints from `pos` to the next region boundary and returns it
    /// (`None` at the end of the line or when out of budget).
    fn step(&self, painter: &mut Painter, stack: &'g [Frame], pos: usize) -> Option<Boundary> {
        let (owner, grammar, context) = self.context(stack);
        let end = self.earliest_end(painter, stack, pos);
        let limit = end.as_ref().map_or(painter.line_len(), |(_, m)| m.start);
        let child = painter.first_start(grammar, context, pos, limit);
        let stop = child.as_ref().map_or(limit, |(_, m)| m.start);
        let inner_kind = stack
            .last()
            .and_then(|f| self.region(f))
            .filter(|r| r.include.is_none())
            .and_then(|r| r.kind);
        painter.fill(pos..stop, inner_kind);
        painter.rules(context, pos..stop);
        if let Some((index, m)) = child {
            let region = grammar.regions.get(usize::from(index))?;
            painter.fill(m.clone(), region.kind);
            let frame = self.enter(painter, stack, owner, (index, region), &m);
            return Some(Boundary::Start(frame, m.end));
        }
        let (depth, m) = end?;
        let kind = self.region(&stack[depth]).and_then(|r| r.kind);
        painter.fill(m.clone(), kind);
        Some(Boundary::End(depth, m.end))
    }

    /// The first end from `pos` on among the innermost open region and the
    /// enclosing ones that embed a language (on a tie, the outermost).
    fn earliest_end(
        &self,
        painter: &mut Painter,
        stack: &'g [Frame],
        pos: usize,
    ) -> Option<(usize, Range<usize>)> {
        let mut best: Option<(usize, Range<usize>)> = None;
        for (depth, frame) in stack.iter().enumerate().rev() {
            let Some(region) = self.region(frame) else {
                continue;
            };
            if depth + 1 != stack.len() && region.include.is_none() {
                continue;
            }
            let end = match frame.end.as_deref() {
                Some(captured) => (captured, true),
                None => (&region.end, false),
            };
            let Some(m) = painter.find_end(end, region.skip.as_ref(), pos) else {
                continue;
            };
            if best.as_ref().is_none_or(|(_, b)| m.start <= b.start) {
                best = Some((depth, m));
            }
        }
        best
    }

    /// The frame of `region` (number `index` of the grammar of `owner`),
    /// whose start matched `m`: its end and embedded language come from
    /// the start's captures.
    fn enter(
        &self,
        painter: &mut Painter,
        stack: &[Frame],
        owner: Option<&Arc<Language>>,
        (index, region): (u16, &Region),
        m: &Range<usize>,
    ) -> Frame {
        let caps = region
            .uses_captures()
            .then(|| painter.captures(&region.start, m.start))
            .flatten();
        let end = region
            .captured_end
            .as_ref()
            .zip(caps.as_ref())
            .and_then(|(end, caps)| end.compile(caps));
        let embedded = stack.iter().filter(|f| f.embed.is_some()).count();
        let embed = region
            .include_name(caps.as_ref())
            .filter(|_| embedded < MAX_EMBED_DEPTH)
            .and_then(|name| self.registry.embedded(&name));
        Frame {
            owner: owner.cloned(),
            region: index,
            end,
            embed,
        }
    }
}
