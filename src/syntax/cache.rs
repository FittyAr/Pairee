//! Viewport highlighting with remembered parser states.
//!
//! Highlighting line `n` needs the parser state after lines `0..n`. The
//! cache keeps that state every [`CHECKPOINT_EVERY`] lines, so painting the
//! visible lines parses at most one stretch between checkpoints plus the
//! screen. An edit drops the checkpoints after the edited line.
//!
//! A jump far past the last checkpoint (End in a huge file) does not parse
//! everything before it: parsing restarts [`RESYNC_LOOKBACK`] lines above
//! the screen from a fresh state, which is exact for nearly all text (a
//! block comment opened further up is the exception) and keeps every frame
//! cheap.

use super::LineTokens;
use super::highlight::{Grammars, LineState};
use super::language::Language;
use super::registry::{self, Registry};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

/// Lines between remembered parser states.
const CHECKPOINT_EVERY: usize = 64;
/// Farthest a checkpoint may lie above the screen and still be parsed
/// forward exactly; past it the parser restarts near the screen.
const MAX_EXACT_GAP: usize = 4_000;
/// Lines parsed above the screen after a restart.
const RESYNC_LOOKBACK: usize = 200;

/// Incremental highlighter of one document.
#[derive(Debug, Clone)]
pub struct SyntaxCache {
    language: Arc<Language>,
    /// The grammar lines start in (the host's for an embedded language).
    root: Arc<Language>,
    /// The languages regions may embed.
    registry: Arc<Registry>,
    /// Parser state before line `key`.
    checkpoints: BTreeMap<usize, LineState>,
    /// Tokens of the lines last asked for.
    window: Option<(Range<usize>, Vec<LineTokens>)>,
}

impl SyntaxCache {
    /// A highlighter for `path` (whose first line is `first_line`), or
    /// `None` when no language knows the file (or its grammar does not
    /// compile).
    pub fn detect(path: &Path, first_line: &str) -> Option<Self> {
        let registry = registry::active();
        let language = registry.detect(path, first_line)?;
        language.grammar()?;
        let root = registry.root_of(&language);
        Some(Self {
            language,
            root,
            registry,
            checkpoints: BTreeMap::new(),
            window: None,
        })
    }

    /// Name of the language in use (status line).
    pub fn language(&self) -> &str {
        self.language.name()
    }

    /// Forgets everything that depends on line `line` or later (call after
    /// that line changed).
    pub fn invalidate_from(&mut self, line: usize) {
        self.checkpoints.split_off(&(line + 1));
        self.window = None;
    }

    /// Where parsing must start to highlight from line `start`, and the
    /// state there.
    fn origin(&self, start: usize) -> (usize, LineState) {
        match self.checkpoints.range(..=start).next_back() {
            Some((&line, state)) if start - line <= MAX_EXACT_GAP => (line, state.clone()),
            _ if start <= MAX_EXACT_GAP => (0, LineState::default()),
            _ => (start - RESYNC_LOOKBACK, LineState::default()),
        }
    }

    /// Tokens of lines `range`. `fetch` returns the text of the lines in
    /// the range it is given (fewer at the end of the document); it is only
    /// called when the lines are not cached.
    pub fn highlight<S, F>(&mut self, range: Range<usize>, fetch: F) -> &[LineTokens]
    where
        S: AsRef<str>,
        F: FnOnce(Range<usize>) -> Vec<S>,
    {
        if self.window.as_ref().is_none_or(|(r, _)| *r != range) {
            let tokens = self.parse(range.clone(), fetch);
            self.window = Some((range, tokens));
        }
        self.window.as_ref().map_or(&[], |(_, tokens)| tokens)
    }

    fn parse<S, F>(&mut self, range: Range<usize>, fetch: F) -> Vec<LineTokens>
    where
        S: AsRef<str>,
        F: FnOnce(Range<usize>) -> Vec<S>,
    {
        let Some(root) = self.root.grammar() else {
            return Vec::new();
        };
        let grammars = Grammars {
            root,
            registry: &self.registry,
        };
        let (origin, mut state) = self.origin(range.start);
        let mut out = Vec::with_capacity(range.len());
        for (i, line) in fetch(origin..range.end).iter().enumerate() {
            let y = origin + i;
            if y > origin && y % CHECKPOINT_EVERY == 0 {
                self.checkpoints.entry(y).or_insert_with(|| state.clone());
            }
            let tokens = state.advance(grammars, line.as_ref());
            if y >= range.start {
                out.push(tokens);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::SyntaxKind;
    use std::cell::Cell;

    fn rust() -> SyntaxCache {
        SyntaxCache::detect(Path::new("a.rs"), "").unwrap()
    }

    fn doc(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("let v{i} = {i}; // c")).collect()
    }

    /// `highlight` over `lines`, counting the lines handed to the parser.
    fn run(
        cache: &mut SyntaxCache,
        lines: &[String],
        range: Range<usize>,
        fetched: &Cell<usize>,
    ) -> Vec<LineTokens> {
        cache
            .highlight(range, |r| {
                let r = r.start..r.end.min(lines.len());
                fetched.set(fetched.get() + r.len());
                lines[r].to_vec()
            })
            .to_vec()
    }

    #[test]
    fn highlights_only_the_viewport_and_reuses_it() {
        let lines = doc(1_000);
        let mut cache = rust();
        let fetched = Cell::new(0);
        let tokens = run(&mut cache, &lines, 10..20, &fetched);
        assert_eq!(tokens.len(), 10);
        assert_eq!(fetched.get(), 20, "parsed from the top to the screen end");
        assert_eq!(tokens[0][0].kind, SyntaxKind::Keyword);
        run(&mut cache, &lines, 10..20, &fetched);
        assert_eq!(fetched.get(), 20, "same window: nothing re-parsed");
        run(&mut cache, &lines, 300..310, &fetched);
        run(&mut cache, &lines, 320..330, &fetched);
        assert_eq!(
            fetched.get(),
            20 + 310 + (330 - 256),
            "starts from the checkpoint at line 256"
        );
    }

    #[test]
    fn edits_drop_later_checkpoints() {
        let mut lines = doc(400);
        let mut cache = rust();
        let fetched = Cell::new(0);
        run(&mut cache, &lines, 390..400, &fetched);
        assert!(cache.checkpoints.contains_key(&384));
        lines[100] = "/* open comment".into();
        cache.invalidate_from(100);
        assert!(cache.checkpoints.keys().all(|&k| k <= 100));
        let tokens = run(&mut cache, &lines, 390..400, &fetched);
        assert!(
            tokens.iter().all(|t| t[0].kind == SyntaxKind::Comment),
            "the new comment reaches the screen"
        );
    }

    #[test]
    fn embedded_languages_carry_over_checkpoints() {
        let mut lines = vec!["<script>".to_string()];
        lines.extend(doc(300));
        lines.push("</script><p>".into());
        let mut cache = SyntaxCache::detect(Path::new("a.html"), "").unwrap();
        let fetched = Cell::new(0);
        run(&mut cache, &lines, 250..260, &fetched);
        assert!(cache.checkpoints.contains_key(&192));
        let tokens = run(&mut cache, &lines, 200..202, &fetched);
        assert_eq!(fetched.get(), 260 + 202 - 192, "resumed at line 192");
        assert_eq!(tokens[0][0].kind, SyntaxKind::Keyword, "still JavaScript");
        let tokens = run(&mut cache, &lines, 300..302, &fetched);
        assert_eq!(tokens[1][0].kind, SyntaxKind::Tag, "back in HTML");
        let php = SyntaxCache::detect(Path::new("a.php"), "<p>").unwrap();
        assert_eq!(php.language(), "PHP", "named after the file, HTML inside");
    }

    #[test]
    fn far_jumps_restart_near_the_screen() {
        let lines = doc(20_000);
        let mut cache = rust();
        let fetched = Cell::new(0);
        let tokens = run(&mut cache, &lines, 19_990..20_010, &fetched);
        assert_eq!(tokens.len(), 10, "clipped at the end of the document");
        assert_eq!(fetched.get(), RESYNC_LOOKBACK + 10);
        assert_eq!(tokens[9][0].kind, SyntaxKind::Keyword);
    }
}
