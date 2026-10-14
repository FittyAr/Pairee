//! The command line under the panels: the text being typed, its focus, the
//! command-history position and the Tab completion in progress.

use crate::app::text_input::TextField;
use std::path::PathBuf;

/// Candidates of a Tab completion being cycled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completion {
    /// Line text before the completed word.
    pub prefix: String,
    /// Line text after the cursor (kept as is).
    pub suffix: String,
    /// Completed words, in the order Tab cycles through them.
    pub candidates: Vec<String>,
    /// Candidate shown now.
    pub index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CliLine {
    pub field: TextField,
    /// The command line was opened explicitly (`focus_cli`) and takes every
    /// printable key, even while empty.
    pub focused: bool,
    /// Command-history entry shown on the line (`cli_history_*`).
    pub history_pos: Option<usize>,
    /// Tab completion being cycled (cleared by any other key).
    pub completion: Option<Completion>,
    /// Folder the active panel showed before the last `cd` (for `cd -`).
    pub previous_dir: Option<PathBuf>,
}

/// `text` quoted as one argument of the shell the command line runs in:
/// double quotes around names with spaces on Windows (cmd.exe has no single
/// quotes), POSIX shell quoting elsewhere.
pub fn quote_arg(text: &str) -> String {
    if cfg!(windows) {
        if text.contains(char::is_whitespace) {
            format!("\"{text}\"")
        } else {
            text.to_string()
        }
    } else {
        shlex::try_quote(text).map_or_else(|_| text.to_string(), |q| q.into_owned())
    }
}

impl CliLine {
    pub fn text(&self) -> &str {
        self.field.text()
    }

    /// The line takes typed keys (it is focused or holds text).
    pub fn is_active(&self) -> bool {
        self.focused || !self.field.is_empty()
    }

    /// Replaces the text (cursor at the end).
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.field.set_text(text);
        self.completion = None;
    }

    /// Appends `text` at the end of the line, after a separating space when
    /// the line does not end with one.
    pub fn append_word(&mut self, text: &str) {
        let mut line = self.field.text().to_string();
        if !line.is_empty() && !line.ends_with(' ') {
            line.push(' ');
        }
        line.push_str(text);
        self.set_text(line);
    }

    /// Appends `text` at the cursor and drops any Tab completion in progress.
    pub fn insert_text(&mut self, text: &str) {
        self.field.insert_str(text);
        self.completion = None;
    }

    /// Empties and unfocuses the line.
    pub fn reset(&mut self) {
        self.field.clear();
        self.focused = false;
        self.history_pos = None;
        self.completion = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_word_separates_words() {
        let mut cli = CliLine::default();
        cli.append_word("ls");
        cli.append_word("a.txt");
        assert_eq!(cli.text(), "ls a.txt");
        cli.set_text("cd ");
        cli.append_word("x");
        assert_eq!(cli.text(), "cd x");
        assert!(cli.is_active());
        cli.reset();
        assert!(!cli.is_active());
    }

    #[test]
    fn quote_arg_follows_the_platform_shell() {
        assert_eq!(quote_arg("plain"), "plain");
        let quoted = quote_arg("a b");
        if cfg!(windows) {
            assert_eq!(quoted, "\"a b\"");
            assert_eq!(quote_arg(r"C:\Users\x"), r"C:\Users\x");
        } else {
            assert_eq!(quoted, "'a b'");
        }
    }

    #[test]
    fn insert_text_drops_the_completion() {
        let mut cli = CliLine {
            completion: Some(Completion::default()),
            ..CliLine::default()
        };
        cli.insert_text("x");
        assert!(cli.completion.is_none());
        assert_eq!(cli.text(), "x");
    }
}
