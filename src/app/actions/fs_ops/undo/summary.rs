//! Texts of the undo/redo confirmation and of the "nothing left" report.

use crate::config::localization::t;
use crate::fs::file_name_lossy;
use crate::fs::journal::{Direction, FsCommand, Skipped};
use std::path::Path;

/// Entries listed by name; the rest are counted.
const LISTED: usize = 6;

/// Question, the entries that will be reversed and the ones skipped.
pub(super) fn confirm_lines(
    label: &str,
    direction: Direction,
    runnable: &FsCommand,
    skipped: &[Skipped],
) -> Vec<String> {
    let key = match direction {
        Direction::Undo => "journal_confirm_undo",
        Direction::Redo => "journal_confirm_redo",
    };
    let mut lines = vec![t(key).replacen("{}", label, 1), String::new()];
    let items: Vec<String> = runnable
        .items()
        .iter()
        .map(|(path, to)| item_line(path, to.as_deref()))
        .collect();
    lines.extend(listed(items));
    if !skipped.is_empty() {
        lines.push(String::new());
        lines.push(t("journal_will_skip"));
        lines.extend(listed(skipped.iter().map(skip_line).collect()));
    }
    lines
}

/// Report of an entry that can no longer be reversed at all.
pub(super) fn nothing_left(label: &str, skipped: &[Skipped]) -> String {
    let mut lines = vec![t("journal_nothing_left").replacen("{}", label, 1)];
    lines.extend(listed(skipped.iter().map(skip_line).collect()));
    lines.push(t("journal_removed_from_history"));
    lines.join("\n")
}

/// The first [`LISTED`] lines, then "... and N more".
fn listed(mut lines: Vec<String>) -> Vec<String> {
    if lines.len() > LISTED {
        let more = lines.len() - LISTED;
        lines.truncate(LISTED);
        lines.push(format!(
            "  {}",
            t("journal_more").replacen("{}", &more.to_string(), 1)
        ));
    }
    lines
}

/// "  a.txt → b.txt" (the full new path when the folder changes).
fn item_line(path: &Path, to: Option<&Path>) -> String {
    match to {
        None => format!("  {}", file_name_lossy(path)),
        Some(to) if to.parent() == path.parent() => {
            format!("  {} → {}", file_name_lossy(path), file_name_lossy(to))
        }
        Some(to) => format!("  {} → {}", file_name_lossy(path), to.to_string_lossy()),
    }
}

fn skip_line(skipped: &Skipped) -> String {
    format!(
        "  {}: {}",
        file_name_lossy(&skipped.path),
        skipped.reason.text()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::journal::check::SkipReason;
    use std::path::PathBuf;

    #[test]
    fn long_lists_are_cut_and_counted() {
        let lines: Vec<String> = (0..10).map(|i| i.to_string()).collect();
        let out = listed(lines);
        assert_eq!(out.len(), LISTED + 1);
        assert!(out[LISTED].contains('4'));
    }

    #[test]
    fn report_names_skipped_entries_and_reasons() {
        let skipped = [Skipped {
            path: PathBuf::from("dir").join("a.txt"),
            reason: SkipReason::Changed,
        }];
        let text = nothing_left("Move «a.txt»", &skipped);
        assert!(text.contains("a.txt") && text.contains(&SkipReason::Changed.text()));
    }

    #[test]
    fn rename_in_place_shows_names_only() {
        let line = item_line(Path::new("d/a"), Some(Path::new("d/b")));
        assert_eq!(line, "  a → b");
    }
}
