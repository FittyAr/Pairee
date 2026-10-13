use crate::app::state::Glob;
use crate::fs::FileEntry;
use crate::ui::theme_apply::parse_color;
use ratatui::style::{Color, Style};

use serde::{Deserialize, Serialize};

/// Mask of the rule that colors folders.
const DIR_MASK: &str = "[DIR]";

/// A file highlight rule: files matching the glob mask get a specific foreground color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighlightRule {
    /// Glob pattern (e.g. "*.rs", "*.{zip,tar}")
    pub mask: String,
    /// Named color string (e.g. "Green", "Yellow", "Red")
    pub color: String,
}

impl HighlightRule {
    pub fn new(mask: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            mask: mask.into(),
            color: color.into(),
        }
    }
}

/// Returns the built-in default highlight rules.
/// These match the classic NC/Far Manager color scheme.
pub fn default_highlight_rules() -> Vec<HighlightRule> {
    vec![
        // Directories — bright cyan
        HighlightRule::new(DIR_MASK, "LightCyan"),
        // Source code — green
        HighlightRule::new("*.{rs,py,js,ts,c,cpp,h,go,java,kt,rb,swift}", "LightGreen"),
        // Archives — red / magenta
        HighlightRule::new("*.{zip,tar,gz,bz2,xz,7z,rar,zst}", "LightRed"),
        // Images — yellow
        HighlightRule::new("*.{jpg,jpeg,png,gif,bmp,svg,webp,ico}", "LightYellow"),
        // Documents
        HighlightRule::new("*.{md,txt,pdf,doc,docx,odt}", "White"),
        // Executables
        HighlightRule::new("*.{sh,exe,bat,AppImage}", "LightGreen"),
        // Config / data
        HighlightRule::new("*.{toml,yaml,yml,json,ini,cfg,conf}", "Cyan"),
    ]
}

/// Highlight rules with masks expanded and colors parsed once (when the
/// rules change), ready to style every panel row.
#[derive(Debug, Clone)]
pub struct Highlighter {
    dir: Color,
    files: Vec<(Glob, Color)>,
}

impl Highlighter {
    pub fn new(rules: &[HighlightRule]) -> Self {
        let dir = rules
            .iter()
            .find(|r| r.mask == DIR_MASK)
            .map_or(Color::Cyan, |r| parse_color(&r.color));
        let files = rules
            .iter()
            .filter(|r| r.mask != DIR_MASK)
            .map(|r| (Glob::new(&r.mask), parse_color(&r.color)))
            .collect();
        Self { dir, files }
    }

    /// `base` with the foreground of the first rule matching `entry`
    /// (folders always take the `[DIR]` color).
    pub fn style(&self, entry: &FileEntry, base: Style) -> Style {
        if entry.is_dir {
            return base.fg(self.dir);
        }
        self.files
            .iter()
            .find(|(glob, _)| glob.matches(&entry.name, false))
            .map_or(base, |(_, color)| base.fg(*color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(name: &str, is_dir: bool) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{}", name)),
            is_dir,
            is_symlink: false,
            size: 0,
            modified: None,
        }
    }

    fn style(rules: &[HighlightRule], name: &str, is_dir: bool) -> Style {
        Highlighter::new(rules).style(&make_entry(name, is_dir), Style::default())
    }

    #[test]
    fn test_dir_highlight() {
        let rules = default_highlight_rules();
        assert_eq!(style(&rules, "my_dir", true).fg, Some(Color::LightCyan));
        assert_eq!(style(&[], "my_dir", true).fg, Some(Color::Cyan));
    }

    #[test]
    fn test_rs_file_highlight() {
        let rules = default_highlight_rules();
        assert_eq!(style(&rules, "main.rs", false).fg, Some(Color::LightGreen));
    }

    #[test]
    fn test_unknown_file_falls_back() {
        let rules = default_highlight_rules();
        assert_eq!(style(&rules, "unknown.xyz", false), Style::default());
    }

    #[test]
    fn user_rules_replace_the_defaults() {
        let rules = vec![HighlightRule::new("*.xyz", "Magenta")];
        assert_eq!(style(&rules, "a.XYZ", false).fg, Some(Color::Magenta));
        assert_eq!(style(&rules, "main.rs", false), Style::default());
    }
}
