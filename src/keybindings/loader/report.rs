use crate::config::localization::t;

/// Issues found while loading a keymap (invalid chords, conflicts, unknown actions).
#[derive(Debug, Default, Clone)]
pub struct KeymapLoadReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Chords a later layer took from another action (expected with
    /// `extends` and overrides; informative only).
    pub displaced: Vec<String>,
    /// Essential actions without a chord every terminal can send.
    pub robustness: Vec<String>,
    pub bound_count: usize,
}

impl KeymapLoadReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// One-line status for the Settings → Interface keymap row.
    pub fn summary_line(&self) -> String {
        if self.ok() && self.warnings.is_empty() {
            format!("{} ({})", t("int_keymap_ok"), self.bound_count)
        } else {
            format!("{} ({})", t("int_keymap_bad"), self.errors.len())
        }
    }

    /// Multi-line report for the nested keymap-issues overlay.
    pub fn detail_lines(&self) -> Vec<String> {
        let mut lines = vec![self.summary_line()];
        let sections = [
            ("!", &self.errors),
            ("*", &self.warnings),
            ("≈", &self.robustness),
            ("~", &self.displaced),
        ];
        for (mark, items) in sections {
            lines.extend(items.iter().map(|item| format!("{mark} {item}")));
        }
        lines.push(t("int_keymap_gray"));
        lines
    }
}
