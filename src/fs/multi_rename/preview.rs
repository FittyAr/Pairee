//! Live preview: the new name of every source and what is wrong with it.

use super::RenameSource;
use super::names::TargetFs;
use super::rules::CompiledRules;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Why a row cannot be renamed. Ordered by precedence (first wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    /// The new name is empty.
    Empty,
    /// The new name has characters (or is a name) the filesystem refuses.
    InvalidName,
    /// Two sources would get the same name.
    Duplicate,
    /// Another entry of the folder (not being renamed) has that name.
    Exists,
}

impl Issue {
    /// Translation key of the issue's label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Empty => "multi_rename_issue_empty",
            Self::InvalidName => "multi_rename_issue_invalid",
            Self::Duplicate => "multi_rename_issue_duplicate",
            Self::Exists => "multi_rename_issue_exists",
        }
    }
}

/// One line of the preview table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewRow {
    pub old: String,
    pub new: String,
    pub issue: Option<Issue>,
}

impl PreviewRow {
    pub fn is_changed(&self) -> bool {
        self.old != self.new
    }
}

/// Preview of a whole batch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preview {
    pub rows: Vec<PreviewRow>,
}

impl Preview {
    /// New names of `sources`. `siblings` are the names of every entry of
    /// the folder (sources included), used to detect clashes with files
    /// that are not renamed.
    pub fn build(
        sources: &[RenameSource],
        rules: &CompiledRules,
        siblings: &[String],
        fs: TargetFs,
    ) -> Self {
        let mut rows: Vec<PreviewRow> = sources
            .iter()
            .enumerate()
            .map(|(i, source)| PreviewRow {
                old: source.name(),
                new: rules.new_name(source, i),
                issue: None,
            })
            .collect();

        let source_keys: HashSet<String> = rows.iter().map(|r| fs.key(&r.old)).collect();
        let others: HashSet<String> = siblings
            .iter()
            .map(|name| fs.key(name))
            .filter(|key| !source_keys.contains(key))
            .collect();
        let mut target_count: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            *target_count.entry(fs.key(&row.new)).or_default() += 1;
        }

        for row in &mut rows {
            let key = fs.key(&row.new);
            row.issue = if row.new.is_empty() {
                Some(Issue::Empty)
            } else if !fs.is_valid_name(&row.new) {
                Some(Issue::InvalidName)
            } else if target_count.get(&key).copied().unwrap_or(0) > 1 {
                Some(Issue::Duplicate)
            } else if others.contains(&key) {
                Some(Issue::Exists)
            } else {
                None
            };
        }
        Self { rows }
    }

    /// Number of rows with an issue.
    pub fn conflicts(&self) -> usize {
        self.rows.iter().filter(|r| r.issue.is_some()).count()
    }

    /// Number of rows whose name changes.
    pub fn changes(&self) -> usize {
        self.rows.iter().filter(|r| r.is_changed()).count()
    }

    /// `true` when there is something to rename and nothing blocks it.
    pub fn is_runnable(&self) -> bool {
        self.conflicts() == 0 && self.changes() > 0
    }

    /// `(old path, new path)` of every changed row.
    pub fn moves(&self, sources: &[RenameSource]) -> Vec<(PathBuf, PathBuf)> {
        sources
            .iter()
            .zip(&self.rows)
            .filter(|(_, row)| row.is_changed())
            .map(|(source, row)| (source.path.clone(), source.path.with_file_name(&row.new)))
            .collect()
    }
}
