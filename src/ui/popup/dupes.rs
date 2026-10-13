//! Duplicate finder: the options form and the results, grouped (size and
//! wasted space per group, a mark and the modification time per file).

use crate::app::dupes::DupesState;
use crate::app::state::popup::forms::DUPE_OPTIONS_FORM;
use crate::app::state::popup::{DupeOptionsPrompt, DuplicatesView};
use crate::config::localization::t;
use crate::fs::dupes::{DupeProgress, DupeStage};
use crate::ui::panel::helpers::{format_date, format_file_size};
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{self, FocusStyles, ListPopup, Scroll, mouse::FormFocus};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

impl DialogView for DupeOptionsPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        let inner = kit::dialog_frame(
            f,
            cx.area,
            (64, 12),
            t("dupes_title"),
            kit::fg(theme.popup_border),
            theme,
        );
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(inner);
        let scope = kit::items_label(&self.roots, "dupes_scope_one", "dupes_scope_many");
        f.render_widget(
            Paragraph::new(vec![Line::from(scope), Line::from(t("dupes_links_note"))])
                .style(theme.popup_fg()),
            rows[0],
        );
        let styles = FocusStyles::from_theme(theme);
        let form = FormFocus::new(DUPE_OPTIONS_FORM, self.cursor_idx);
        kit::form_inline_field(
            f,
            rows[1],
            (&t("dupes_min_size"), &self.min_size),
            0,
            form,
            styles,
        );
        let algorithm = t("dupes_algorithm").replace("{}", self.algorithm.as_str());
        kit::form_option(f, rows[2], &algorithm, 1, form, styles);
        kit::form_buttons(
            f,
            rows[4],
            &[t("dupes_start_bracket"), t("btn_cancel_bracket")],
            form,
            styles,
        );
    }
}

impl DialogView for DuplicatesView {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let dupes = &cx.state.dupes;
        let (rows, items) = rows(dupes, cx);
        let cursor_row = items
            .iter()
            .position(|item| *item == Some(dupes.cursor))
            .unwrap_or(0);
        ListPopup {
            area: centered_rect_in(85, 85, cx.area),
            title: t("dupes_title"),
            border: Color::Cyan,
            empty: Some(empty_text(dupes)),
            header: header(dupes),
            rows,
            cursor: cursor_row,
            scroll: Scroll::HalfPage,
            hint: Some(t("dupes_hint")),
            scrollbar: None,
        }
        .render_items(f, cx.theme, Some((&items, dupes.cursor)));
    }
}

/// Group headings and file rows, with each row's file index (`None` for
/// headings).
fn rows(dupes: &DupesState, cx: &ViewCtx<'_>) -> (Vec<(String, Style)>, Vec<Option<usize>>) {
    let heading = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let (mut rows, mut items) = (Vec::new(), Vec::new());
    let mut index = 0;
    for group in dupes.groups() {
        let text = t("dupes_group")
            .replace("{count}", &group.files.len().to_string())
            .replace("{size}", &format_file_size(group.size))
            .replace("{wasted}", &format_file_size(group.wasted()));
        rows.push((format!(" {text}"), heading));
        items.push(None);
        for file in &group.files {
            let marked = dupes.is_marked(file);
            let style = if marked {
                Style::default().fg(Color::LightRed)
            } else {
                cx.theme.popup_fg()
            };
            let text = format!(
                "   {} {}  {}",
                kit::checkbox(marked),
                format_date(dupes.modified(file)),
                file.to_string_lossy()
            );
            rows.push((text, style));
            items.push(Some(index));
            index += 1;
        }
    }
    (rows, items)
}

/// Totals (or search progress) and the "incomplete" note.
fn header(dupes: &DupesState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if dupes.is_scanning() {
        lines.push(Line::from(progress_text(dupes.progress())));
    } else if let Some(report) = dupes.report() {
        let wasted: u64 = report.groups.iter().map(|g| g.wasted()).sum();
        let totals = t("dupes_totals")
            .replace("{groups}", &report.groups.len().to_string())
            .replace("{files}", &dupes.file_count().to_string())
            .replace("{wasted}", &format_file_size(wasted))
            .replace("{marked}", &dupes.marked().len().to_string())
            .replace("{marked_size}", &format_file_size(dupes.marked_bytes()));
        lines.push(Line::from(Span::styled(
            format!(" {totals}"),
            Style::default().fg(Color::Yellow),
        )));
        if report.partial {
            lines.push(Line::from(Span::styled(
                format!(" {}", t("dupes_partial_note")),
                Style::default().fg(Color::LightRed),
            )));
        }
    }
    lines.push(Line::from(""));
    lines
}

/// Shown instead of the rows: progress, or "no duplicates".
fn empty_text(dupes: &DupesState) -> String {
    if dupes.is_scanning() {
        progress_text(dupes.progress())
    } else {
        format!(" {}", t("dupes_none"))
    }
}

fn progress_text(progress: Option<DupeProgress>) -> String {
    let Some(progress) = progress else {
        return format!(" {}", t("dupes_starting"));
    };
    let key = match progress.stage {
        DupeStage::Scanning => "dupes_scanning",
        DupeStage::PartialHash => "dupes_hashing_partial",
        DupeStage::FullHash => "dupes_hashing_full",
    };
    format!(
        " {}",
        t(key)
            .replace("{done}", &progress.done.to_string())
            .replace("{total}", &progress.total.to_string())
            .replace("{path}", &progress.current.to_string_lossy())
    )
}
