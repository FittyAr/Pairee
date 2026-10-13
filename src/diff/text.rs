//! Line diff of two texts ([`similar`]), as hunks with [`CONTEXT`] lines of
//! context around each change.

use super::{DiffDocument, DiffKind, DiffLine};
use similar::{ChangeTag, DiffOp, TextDiff};
use std::time::Duration;

/// Unchanged lines shown around each change.
pub const CONTEXT: usize = 3;
/// Past this the diff algorithm settles for a correct but longer diff.
const TIMEOUT: Duration = Duration::from_secs(2);

/// The hunks that turn `old` into `new` (empty when they are equal).
pub fn diff_texts(old: &str, new: &str) -> DiffDocument {
    let diff = TextDiff::configure().timeout(TIMEOUT).diff_lines(old, new);
    let mut lines = Vec::new();
    for group in diff.grouped_ops(CONTEXT) {
        lines.push(hunk_header(&group));
        for op in &group {
            for change in diff.iter_changes(op) {
                let kind = match change.tag() {
                    ChangeTag::Equal => DiffKind::Context,
                    ChangeTag::Delete => DiffKind::Removed,
                    ChangeTag::Insert => DiffKind::Added,
                };
                lines.push(DiffLine {
                    kind,
                    text: change.value().trim_end_matches(['\n', '\r']).to_string(),
                    old_no: change.old_index().map(|i| i + 1),
                    new_no: change.new_index().map(|i| i + 1),
                });
            }
        }
    }
    DiffDocument { lines }
}

/// `@@ -start,count +start,count @@` of a group of operations.
fn hunk_header(group: &[DiffOp]) -> DiffLine {
    let (Some(first), Some(last)) = (group.first(), group.last()) else {
        return DiffLine::header(String::new());
    };
    let old = first.old_range().start..last.old_range().end;
    let new = first.new_range().start..last.new_range().end;
    DiffLine {
        kind: DiffKind::Hunk,
        ..DiffLine::header(format!(
            "@@ -{},{} +{},{} @@",
            old.start + 1,
            old.len(),
            new.start + 1,
            new.len()
        ))
    }
}
