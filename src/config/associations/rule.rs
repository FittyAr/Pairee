use serde::{Deserialize, Serialize};

/// A single file association rule: maps a glob mask to open/view commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssocRule {
    /// Glob mask, e.g. "*.rs" or "*.{jpg,png}"
    pub mask: String,
    /// Shell command to open the file (replaces `%f` with the file path).
    /// Example: "code %f"
    pub open_cmd: String,
    /// Optional viewer command for F3 (replaces `%f`). Falls back to open_cmd if None.
    pub view_cmd: Option<String>,
}

impl AssocRule {
    /// Returns true if the given filename matches this rule's mask.
    pub fn matches(&self, filename: &str) -> bool {
        crate::app::state::glob_matches(&self.mask, filename)
    }

    /// Returns the resolved open command as a `(program, args)` pair with the
    /// file path substituted for `%f`.
    pub fn resolve_open_cmd(&self, path: &std::path::Path) -> (String, Vec<String>) {
        resolve_template(&self.open_cmd, path)
    }

    /// Returns the resolved view command as a `(program, args)` pair.
    /// Falls back to `open_cmd` if `view_cmd` is not set.
    pub fn resolve_view_cmd(&self, path: &std::path::Path) -> (String, Vec<String>) {
        let template = self.view_cmd.as_deref().unwrap_or(&self.open_cmd);
        resolve_template(template, path)
    }
}

/// Splits a command template into `(program, args)` and substitutes `%f` with the file path.
fn resolve_template(template: &str, path: &std::path::Path) -> (String, Vec<String>) {
    let path_str = path.to_string_lossy().into_owned();
    const SENTINEL: &str = "\u{1f}PaireeFileSentinel\u{1f}";
    let substituted = template.replace("%f", SENTINEL);
    let mut parts =
        crate::app::actions::fs_ops::helper::split_command_line(&substituted).into_iter();
    let program = parts.next().unwrap_or_default();
    let args: Vec<String> = parts.map(|s| s.replace(SENTINEL, &path_str)).collect();
    (program.replace(SENTINEL, &path_str), args)
}
