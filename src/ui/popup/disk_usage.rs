//! Disk usage view: the children of a scanned folder, largest first, with
//! their share of the folder as a percentage and a bar.

use crate::app::disk_usage::DiskUsageState;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::fs::du::DuNode;
use crate::ui::panel::helpers::{dir_size_text, format_file_size};
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{self, ListPopup, Scroll};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};

/// Width of the usage bar, in cells.
const BAR_WIDTH: usize = 20;
const BAR_FULL: char = '█';
const BAR_EMPTY: char = '░';

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    state: &AppState,
    theme: &Theme,
    size: Rect,
) -> bool {
    if !matches!(popup, PopupType::DiskUsage) {
        return false;
    }
    let du = &state.disk_usage;
    let current = du.current();
    let total = current.map_or(0, |dir| dir.size.bytes);
    let rows = current
        .map(|dir| {
            dir.children
                .iter()
                .map(|child| (row_text(child, total), row_style(child, theme)))
                .collect()
        })
        .unwrap_or_default();
    ListPopup {
        area: centered_rect(80, 80, size),
        title: t("du_title").replace("{}", &du.current_path().to_string_lossy()),
        border: Color::Cyan,
        empty: Some(empty_text(du)),
        header: header(du, current),
        rows,
        cursor: du.cursor,
        scroll: Scroll::HalfPage,
        hint: Some(t("du_hint")),
        scrollbar: None,
    }
    .render(f, theme);
    true
}

/// `"  12.0 MB  45.3% [█████░░░░░] name/"`.
fn row_text(node: &DuNode, total: u64) -> String {
    let percent = node.percent_of(total);
    let filled = ((percent / 100.0) * BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let bar: String = std::iter::repeat_n(BAR_FULL, filled)
        .chain(std::iter::repeat_n(BAR_EMPTY, BAR_WIDTH - filled))
        .collect();
    let suffix = if node.is_dir { "/" } else { "" };
    format!(
        " {:>11} {:>5.1}% [{}] {}{}",
        dir_size_text(&node.size),
        percent,
        bar,
        node.name,
        suffix
    )
}

fn row_style(node: &DuNode, theme: &Theme) -> Style {
    if node.is_dir {
        kit::popup_fg(theme)
    } else {
        Style::default().fg(Color::Gray)
    }
}

/// Totals of the folder on screen, plus the "incomplete" note.
fn header(du: &DiskUsageState, current: Option<&DuNode>) -> Vec<Line<'static>> {
    let Some(dir) = current else {
        return Vec::new();
    };
    let totals = t("du_total")
        .replace("{size}", &dir_size_text(&dir.size))
        .replace("{files}", &dir.size.files.to_string())
        .replace("{dirs}", &dir.size.dirs.to_string());
    let mut lines = vec![Line::from(Span::styled(
        format!(" {}", totals),
        Style::default().fg(Color::Yellow),
    ))];
    if dir.size.partial {
        lines.push(Line::from(Span::styled(
            format!(" {}", t("du_partial_note")),
            Style::default().fg(Color::LightRed),
        )));
    }
    if du.is_scanning() {
        lines.push(Line::from(scanning_text(du)));
    }
    lines.push(Line::from(""));
    lines
}

/// Shown instead of the rows: scan progress, or "empty folder".
fn empty_text(du: &DiskUsageState) -> String {
    if du.is_scanning() {
        scanning_text(du)
    } else {
        format!(" {}", t("du_empty"))
    }
}

fn scanning_text(du: &DiskUsageState) -> String {
    let progress = du.progress().unwrap_or_default();
    format!(
        " {}",
        t("du_scanning")
            .replace("{files}", &progress.files.to_string())
            .replace("{size}", &format_file_size(progress.bytes))
            .replace("{path}", &progress.current.to_string_lossy())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::du::DirSize;

    fn node(name: &str, bytes: u64, is_dir: bool, partial: bool) -> DuNode {
        DuNode {
            name: name.into(),
            is_dir,
            size: DirSize {
                bytes,
                files: 1,
                dirs: 0,
                partial,
            },
            children: Vec::new(),
        }
    }

    #[test]
    fn row_shows_percent_bar_and_folder_slash() {
        let text = row_text(&node("docs", 512, true, false), 1024);
        assert!(text.contains("50.0%"), "{text}");
        assert!(text.contains(&"█".repeat(10)), "{text}");
        assert!(text.contains(&"░".repeat(10)), "{text}");
        assert!(text.ends_with("docs/"), "{text}");
    }

    #[test]
    fn row_of_empty_total_has_empty_bar() {
        let text = row_text(&node("a", 0, false, false), 0);
        assert!(text.contains("0.0%"));
        assert!(text.contains(&"░".repeat(BAR_WIDTH)));
        assert!(text.ends_with(" a"));
    }

    #[test]
    fn partial_sizes_are_marked() {
        let partial = node("x", 10, true, true);
        let expected = t("dir_size_partial").replace("{}", "10 B");
        assert_eq!(dir_size_text(&partial.size), expected);
        assert_eq!(dir_size_text(&node("x", 10, true, false).size), "10 B");
    }
}
