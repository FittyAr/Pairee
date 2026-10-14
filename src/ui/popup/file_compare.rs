//! Compare files view: a summary (differences, or sizes and hashes of
//! binary files) over the shared diff box.

use crate::app::state::popup::{CompareStatus, FileCompareView};
use crate::config::localization::t;
use crate::diff::files::BinaryComparison;
use crate::ui::panel::helpers::format_file_size;
use crate::ui::popup::diff_view::DiffBox;
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
};
use std::path::Path;

impl DialogView for FileCompareView {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let title = t("compare_files_title")
            .replacen("{}", &name(&self.old), 1)
            .replacen("{}", &name(&self.new), 1);
        let empty = match self.status {
            CompareStatus::Text => t("compare_files_identical"),
            _ => String::new(),
        };
        DiffBox {
            title,
            summary: summary(self, cx.theme),
            viewer: &self.viewer,
            empty,
            hint: t("diff_view_hint"),
        }
        .render(f, cx.area, cx.theme);
    }
}

fn name(path: &Path) -> String {
    crate::fs::file_name_lossy(path)
}

/// The two paths, then what the comparison found.
fn summary(view: &FileCompareView, theme: &ResolvedTheme) -> Vec<Line<'static>> {
    let mut lines = vec![
        path_line("compare_files_old", &view.old),
        path_line("compare_files_new", &view.new),
    ];
    match &view.status {
        CompareStatus::Running => lines.push(colored(t("compare_files_running"), theme.ui.accent)),
        CompareStatus::Failed(error) => lines.push(colored(
            t("compare_files_failed").replace("{}", error),
            theme.ui.error,
        )),
        CompareStatus::Text => {
            let (added, removed) = view.viewer.doc.stats();
            if added + removed > 0 {
                lines.push(colored(
                    t("compare_files_text_summary")
                        .replace("{added}", &added.to_string())
                        .replace("{removed}", &removed.to_string()),
                    theme.ui.warning,
                ));
            }
        }
        CompareStatus::Binary(binary) => lines.extend(binary_lines(binary, theme)),
    }
    lines.push(Line::from(""));
    lines
}

fn path_line(key: &str, path: &Path) -> Line<'static> {
    Line::from(format!(
        " {}",
        t(key).replace("{}", &path.to_string_lossy())
    ))
}

fn colored(text: String, color: Color) -> Line<'static> {
    Line::from(Span::styled(format!(" {text}"), Style::default().fg(color)))
}

/// Verdict, then size and hash of each file.
fn binary_lines(binary: &BinaryComparison, theme: &ResolvedTheme) -> Vec<Line<'static>> {
    let (verdict, color) = if binary.identical {
        (t("compare_files_binary_identical"), theme.ui.success_alt)
    } else {
        (t("compare_files_binary_differ"), theme.ui.error)
    };
    let side = |key: &str, size: u64, hash: &str| {
        Line::from(format!(
            " {}",
            t(key)
                .replace("{size}", &format_file_size(size))
                .replace("{algorithm}", binary.algorithm.as_str())
                .replace("{hash}", hash)
        ))
    };
    vec![
        colored(verdict, color),
        side("compare_files_old_hash", binary.sizes.0, &binary.hashes.0),
        side("compare_files_new_hash", binary.sizes.1, &binary.hashes.1),
    ]
}
