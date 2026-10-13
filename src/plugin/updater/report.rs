//! What a plugin updater operation did, returned to the caller instead of
//! printed: the CLI prints it, the TUI shows it in a toast. Nothing in the
//! updater writes to stdout/stderr (that would corrupt the TUI).

use std::fmt;

/// Lines describing the outcome of an operation, plus warnings the user
/// should see even when it succeeded (e.g. an unsigned registry).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
    /// A registry file used by the operation had no signature.
    pub unsigned: bool,
}

impl Report {
    /// Adds an informational line (also logged).
    pub fn line(&mut self, text: impl Into<String>) {
        let text = text.into();
        tracing::info!("{text}");
        self.lines.push(text);
    }

    /// Adds a warning (also logged).
    pub fn warn(&mut self, text: impl Into<String>) {
        let text = text.into();
        tracing::warn!("{text}");
        if !self.warnings.contains(&text) {
            self.warnings.push(text);
        }
    }

    /// Records that the registry file `name` was not signed.
    pub fn mark_unsigned(&mut self, name: &str) {
        self.unsigned = true;
        self.warn(format!(
            "{name} is not signed; its integrity rests on HTTPS and the registry alone"
        ));
    }

    /// Appends the lines and warnings of `other`.
    pub fn absorb(&mut self, other: Report) {
        self.unsigned |= other.unsigned;
        self.lines.extend(other.lines);
        for w in other.warnings {
            if !self.warnings.contains(&w) {
                self.warnings.push(w);
            }
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let all: Vec<String> = self
            .lines
            .iter()
            .cloned()
            .chain(self.warnings.iter().map(|w| format!("WARNING: {w}")))
            .collect();
        f.write_str(&all.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorb_keeps_lines_and_dedups_warnings() {
        let mut a = Report::default();
        a.line("one");
        a.warn("unsigned");
        let mut b = Report::default();
        b.line("two");
        b.warn("unsigned");
        a.absorb(b);
        assert_eq!(a.lines, vec!["one", "two"]);
        assert_eq!(a.warnings, vec!["unsigned"]);
        assert_eq!(a.to_string(), "one\ntwo\nWARNING: unsigned");
    }
}
