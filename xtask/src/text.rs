//! Line-based text files that keep their line endings (the working copies may
//! be CRLF on Windows and LF elsewhere).

use std::path::{Path, PathBuf};

use crate::Result;

pub struct TextFile {
    path: PathBuf,
    pub lines: Vec<String>,
    newline: &'static str,
    trailing_newline: bool,
}

impl TextFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Ok(Self {
            newline: if content.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            },
            trailing_newline: content.ends_with('\n'),
            lines: content.lines().map(str::to_string).collect(),
            path,
        })
    }

    pub fn save(&self) -> Result {
        let mut content = self.lines.join(self.newline);
        if self.trailing_newline {
            content.push_str(self.newline);
        }
        std::fs::write(&self.path, content)
            .map_err(|e| format!("cannot write {}: {e}", self.path.display()).into())
    }

    /// Replaces the first line accepted by `matches` with `replacement`.
    /// Returns whether a line was found.
    pub fn replace_line(
        &mut self,
        matches: impl Fn(&str) -> bool,
        replacement: impl FnOnce(&str) -> String,
    ) -> bool {
        match self.lines.iter_mut().find(|l| matches(l)) {
            Some(line) => {
                *line = replacement(line);
                true
            }
            None => false,
        }
    }
}
