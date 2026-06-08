use crate::fs::FileEntry;
use crate::ui::theme_apply::parse_color;
use ratatui::style::{Color, Style};

/// A file highlight rule: files matching the glob mask get a specific foreground color.
#[derive(Debug, Clone)]
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
        HighlightRule::new("[DIR]", "Cyan"),
        // Source code — green
        HighlightRule::new("*.{rs,py,js,ts,c,cpp,h,go,java,kt,rb,swift}", "Green"),
        // Archives — red / magenta
        HighlightRule::new("*.{zip,tar,gz,bz2,xz,7z,rar,zst}", "Red"),
        // Images — yellow
        HighlightRule::new("*.{jpg,jpeg,png,gif,bmp,svg,webp,ico}", "Yellow"),
        // Documents
        HighlightRule::new("*.{md,txt,pdf,doc,docx,odt}", "White"),
        // Executables
        HighlightRule::new("*.{sh,exe,bat,AppImage}", "LightGreen"),
        // Config / data
        HighlightRule::new("*.{toml,yaml,yml,json,ini,cfg,conf}", "LightCyan"),
    ]
}

/// Returns the appropriate Ratatui Style for a FileEntry based on highlight rules.
/// Falls back to the base style if no rule matches.
pub fn style_for_entry(entry: &FileEntry, rules: &[HighlightRule], base: Style) -> Style {
    if entry.is_dir {
        // Directories always get their dedicated highlight
        if let Some(rule) = rules.iter().find(|r| r.mask == "[DIR]") {
            return base.fg(parse_color(&rule.color));
        }
        return base.fg(Color::Cyan);
    }

    for rule in rules {
        if rule.mask == "[DIR]" {
            continue;
        }
        if crate::app::state::glob_matches(&rule.mask, &entry.name) {
            return base.fg(parse_color(&rule.color));
        }
    }

    base
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

    #[test]
    fn test_dir_highlight() {
        let rules = default_highlight_rules();
        let entry = make_entry("my_dir", true);
        let style = style_for_entry(&entry, &rules, Style::default());
        // Directory should get a non-default foreground
        assert_ne!(style, Style::default());
    }

    #[test]
    fn test_rs_file_highlight() {
        let rules = default_highlight_rules();
        let entry = make_entry("main.rs", false);
        let style = style_for_entry(&entry, &rules, Style::default());
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_unknown_file_falls_back() {
        let rules = default_highlight_rules();
        let entry = make_entry("unknown.xyz", false);
        let style = style_for_entry(&entry, &rules, Style::default());
        assert_eq!(style, Style::default());
    }
}
