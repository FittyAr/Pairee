//! Line diffs shown by the diff views: one model ([`DiffDocument`]) built
//! either from a unified patch (Git) or from two texts ([`text`], Compare
//! files), with a side-by-side layout ([`DiffDocument::side_by_side`]).
//! No UI code: the views only paint these lines.

pub mod files;
pub mod text;

#[cfg(test)]
mod tests;

/// What a diff line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// File headers (`diff --git`, `---`, `+++`, `index`) and notes.
    Header,
    /// `@@ -a,b +c,d @@`.
    Hunk,
    /// Unchanged line.
    Context,
    /// Line only in the old file.
    Removed,
    /// Line only in the new file.
    Added,
}

/// One line of a diff. `text` has no `+` / `-` / space prefix for the
/// lines of a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
    /// Line number in the old file (1-based), for context and removed lines.
    pub old_no: Option<usize>,
    /// Line number in the new file (1-based), for context and added lines.
    pub new_no: Option<usize>,
}

impl DiffLine {
    /// A header or note line.
    pub fn header(text: impl Into<String>) -> Self {
        Self {
            kind: DiffKind::Header,
            text: text.into(),
            old_no: None,
            new_no: None,
        }
    }

    /// The line as a unified patch shows it (with its `+` / `-` / space).
    pub fn unified(&self) -> String {
        let prefix = match self.kind {
            DiffKind::Header | DiffKind::Hunk => return self.text.clone(),
            DiffKind::Context => ' ',
            DiffKind::Removed => '-',
            DiffKind::Added => '+',
        };
        format!("{prefix}{}", self.text)
    }
}

/// A row of the side-by-side layout: indices into [`DiffDocument::lines`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideRow {
    /// A header or hunk line across both sides.
    Full(usize),
    /// The old line (left) and the new line (right); a side is empty where
    /// a line was only added or only removed.
    Pair(Option<usize>, Option<usize>),
}

/// A whole diff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffDocument {
    pub lines: Vec<DiffLine>,
}

impl DiffDocument {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Lines added and removed.
    pub fn stats(&self) -> (usize, usize) {
        self.lines
            .iter()
            .fold((0, 0), |(add, del), line| match line.kind {
                DiffKind::Added => (add + 1, del),
                DiffKind::Removed => (add, del + 1),
                _ => (add, del),
            })
    }

    /// Parses a unified patch (as `git diff` prints it). Lines outside the
    /// hunks (file headers, notes) are headers, so `--- a/x` is never read
    /// as a removed line.
    pub fn from_unified(patch: &str) -> Self {
        let mut parser = UnifiedParser::default();
        Self {
            lines: patch.lines().map(|line| parser.line(line)).collect(),
        }
    }

    /// The side-by-side layout: context lines on both sides, each run of
    /// removed lines paired with the added lines that follow it.
    pub fn side_by_side(&self) -> Vec<SideRow> {
        let mut rows = Vec::with_capacity(self.lines.len());
        let mut removed: Vec<usize> = Vec::new();
        let mut added: Vec<usize> = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            match line.kind {
                DiffKind::Removed if added.is_empty() => removed.push(i),
                DiffKind::Removed => {
                    flush_changes(&mut rows, &mut removed, &mut added);
                    removed.push(i);
                }
                DiffKind::Added => added.push(i),
                DiffKind::Context => {
                    flush_changes(&mut rows, &mut removed, &mut added);
                    rows.push(SideRow::Pair(Some(i), Some(i)));
                }
                DiffKind::Header | DiffKind::Hunk => {
                    flush_changes(&mut rows, &mut removed, &mut added);
                    rows.push(SideRow::Full(i));
                }
            }
        }
        flush_changes(&mut rows, &mut removed, &mut added);
        rows
    }
}

/// Pairs the pending removed and added lines, row by row.
fn flush_changes(rows: &mut Vec<SideRow>, removed: &mut Vec<usize>, added: &mut Vec<usize>) {
    let len = removed.len().max(added.len());
    rows.extend((0..len).map(|i| SideRow::Pair(removed.get(i).copied(), added.get(i).copied())));
    removed.clear();
    added.clear();
}

/// State of [`DiffDocument::from_unified`]: inside a hunk, the lines left
/// on each side and the next line numbers.
#[derive(Default)]
struct UnifiedParser {
    old_left: usize,
    new_left: usize,
    old_no: usize,
    new_no: usize,
}

impl UnifiedParser {
    fn in_hunk(&self) -> bool {
        self.old_left > 0 || self.new_left > 0
    }

    fn line(&mut self, line: &str) -> DiffLine {
        if let Some((old, new)) = hunk_ranges(line) {
            (self.old_no, self.old_left) = old;
            (self.new_no, self.new_left) = new;
            return DiffLine {
                kind: DiffKind::Hunk,
                ..DiffLine::header(line)
            };
        }
        if !self.in_hunk() {
            return DiffLine::header(line);
        }
        let (kind, text) = match line.split_at_checked(1) {
            Some(("+", rest)) => (DiffKind::Added, rest),
            Some(("-", rest)) => (DiffKind::Removed, rest),
            Some((" ", rest)) => (DiffKind::Context, rest),
            // "\ No newline at end of file" and anything unexpected.
            _ => return DiffLine::header(line),
        };
        let old_no = (kind != DiffKind::Added).then(|| self.take_old());
        let new_no = (kind != DiffKind::Removed).then(|| self.take_new());
        DiffLine {
            kind,
            text: text.to_string(),
            old_no,
            new_no,
        }
    }

    fn take_old(&mut self) -> usize {
        self.old_left = self.old_left.saturating_sub(1);
        self.old_no += 1;
        self.old_no - 1
    }

    fn take_new(&mut self) -> usize {
        self.new_left = self.new_left.saturating_sub(1);
        self.new_no += 1;
        self.new_no - 1
    }
}

/// `((old start, old count), (new start, new count))` of a hunk header.
fn hunk_ranges(line: &str) -> Option<((usize, usize), (usize, usize))> {
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(" +")?;
    Some((range(old)?, range(new)?))
}

/// `start,count` (count 1 when missing).
fn range(text: &str) -> Option<(usize, usize)> {
    match text.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((text.parse().ok()?, 1)),
    }
}
