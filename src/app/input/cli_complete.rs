//! Tab completion on the command line: the word before the cursor is
//! completed with the names of the folder it points into (the active
//! panel's folder for a bare name); pressing the key again shows the next
//! match.

use crate::app::state::AppState;
use crate::app::state::cli_line::Completion;
use std::path::{Path, PathBuf};

/// Completes the word before the cursor, or shows the next match of the
/// completion in progress. Returns `false` when nothing matches.
pub fn complete(state: &mut AppState) -> bool {
    if let Some(completion) = state.cli.completion.as_mut() {
        completion.index = (completion.index + 1) % completion.candidates.len();
        apply(state);
        return true;
    }
    let text = state.cli.text().to_string();
    let (head, suffix) = text.split_at(state.cli.field.cursor());
    let start = word_start(head);
    let (line_prefix, word) = head.split_at(start);
    let word = word.trim_start_matches('"');
    let (dir_part, name_prefix) = split_dir(word);
    let Some(base) = completion_base(state, dir_part) else {
        return false;
    };
    let candidates: Vec<String> = names_in(state, &base)
        .into_iter()
        .filter(|(name, _)| starts_with_ignore_case(name, name_prefix))
        .map(|(name, is_dir)| {
            let separator = if is_dir {
                std::path::MAIN_SEPARATOR_STR
            } else {
                ""
            };
            format!("{dir_part}{name}{separator}")
        })
        .collect();
    if candidates.is_empty() {
        return false;
    }
    state.cli.completion = Some(Completion {
        prefix: line_prefix.to_string(),
        suffix: suffix.to_string(),
        candidates,
        index: 0,
    });
    apply(state);
    true
}

/// Shows the current candidate of the completion in the line.
fn apply(state: &mut AppState) {
    let Some(c) = &state.cli.completion else {
        return;
    };
    let word = &c.candidates[c.index];
    let word = if word.contains(' ') {
        format!("\"{word}\"")
    } else {
        word.clone()
    };
    let before = format!("{}{word}", c.prefix);
    let cursor = before.len();
    state.cli.field.set_text(format!("{before}{}", c.suffix));
    state.cli.field.set_cursor(cursor);
}

/// Byte offset where the last word of `head` starts: after the last space
/// outside double quotes, or at the opening quote of an unclosed one.
fn word_start(head: &str) -> usize {
    let mut start = 0;
    let mut quote = None;
    for (i, c) in head.char_indices() {
        match c {
            '"' if quote.is_some() => quote = None,
            '"' => quote = Some(i),
            c if c.is_whitespace() && quote.is_none() => start = i + c.len_utf8(),
            _ => {}
        }
    }
    quote.unwrap_or(start)
}

/// `word` split after its last path separator: (folder part, name prefix).
fn split_dir(word: &str) -> (&str, &str) {
    let is_separator = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    match word.rfind(is_separator) {
        Some(i) => word.split_at(i + 1),
        None => ("", word),
    }
}

/// The folder whose names complete a word starting with `dir_part`.
fn completion_base(state: &AppState, dir_part: &str) -> Option<PathBuf> {
    let current = &state.get_active_panel().current_path;
    if dir_part.is_empty() {
        return Some(current.clone());
    }
    let home = crate::app::session::restore::home_dir();
    super::cli_cd::resolve(dir_part, current, None, &home)
}

/// Names (and whether each is a folder) in `dir`: read from disk on a local
/// panel, taken from the listing on other sources (only for the folder they
/// show, so Tab never waits for the network).
fn names_in(state: &AppState, dir: &Path) -> Vec<(String, bool)> {
    let panel = state.get_active_panel();
    let mut names: Vec<(String, bool)> = if panel.source.locate(dir).is_local() {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| {
                let is_dir = e.file_type().is_ok_and(|t| t.is_dir()) || e.path().is_dir();
                (e.file_name().to_string_lossy().into_owned(), is_dir)
            })
            .collect()
    } else if dir == panel.current_path {
        panel
            .entries
            .iter()
            .filter(|e| e.name != "..")
            .map(|e| (e.name.clone(), e.is_dir))
            .collect()
    } else {
        Vec::new()
    };
    names.sort_by_key(|(name, _)| name.to_lowercase());
    names
}

fn starts_with_ignore_case(name: &str, prefix: &str) -> bool {
    name.to_lowercase().starts_with(&prefix.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_word_being_typed() {
        assert_eq!(word_start("cd do"), 3);
        assert_eq!(word_start("cd \"My Do"), 3);
        assert_eq!(word_start("cp \"a b\" c"), 9);
        assert_eq!(split_dir("sub/na"), ("sub/", "na"));
        assert_eq!(split_dir("na"), ("", "na"));
    }

    #[test]
    fn completes_and_cycles_names_of_the_panel_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("Data.txt"), "").unwrap();
        std::fs::write(dir.path().join("my file.txt"), "").unwrap();
        let mut state = AppState::new(dir.path().to_path_buf(), dir.path().to_path_buf());
        state.cli.set_text("cd d");
        assert!(complete(&mut state));
        assert_eq!(state.cli.text(), "cd Data.txt");
        assert!(complete(&mut state));
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(state.cli.text(), format!("cd docs{sep}"));
        state.cli.set_text("type my");
        assert!(complete(&mut state));
        assert_eq!(state.cli.text(), "type \"my file.txt\"");
        state.cli.set_text("zz");
        assert!(!complete(&mut state));
    }
}
