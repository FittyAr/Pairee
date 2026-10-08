//! Multi-rename dialog: rule fields on top, live preview table below.

mod preview;

use crate::app::state::popup::{MultiRenameState as Dialog, RenameField};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::{centered_rect, kit};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

/// Width / height of the dialog in percent of the screen.
const SIZE_PERCENT: (u16, u16) = (90, 85);

pub fn render(f: &mut Frame, dialog: &Dialog, theme: &Theme, size: Rect) -> bool {
    let area = centered_rect(SIZE_PERCENT.0, SIZE_PERCENT.1, size);
    let mut title = t("multi_rename_title");
    if dialog.running {
        title.push_str(&t("multi_rename_running"));
    }
    let inner = kit::frame_in(f, area, title, kit::fg(Color::Cyan), theme);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 0: summary / regex error
            Constraint::Length(1), // 1: name mask | extension mask
            Constraint::Length(1), // 2: search | replace
            Constraint::Length(1), // 3: regex, ignore case, case
            Constraint::Length(1), // 4: counter start, step, digits
            Constraint::Length(1), // 5: separator
            Constraint::Min(3),    // 6: preview
            Constraint::Length(1), // 7: placeholder help
            Constraint::Length(1), // 8: buttons
        ])
        .split(inner);
    let styles = kit::FocusStyles::from_theme(theme);

    f.render_widget(summary(dialog, styles.normal), rows[0]);
    render_fields(
        f,
        dialog,
        styles,
        rows[1],
        &[RenameField::NameMask, RenameField::ExtMask],
    );
    render_fields(
        f,
        dialog,
        styles,
        rows[2],
        &[RenameField::Search, RenameField::Replace],
    );
    render_options(f, dialog, styles, rows[3]);
    render_fields(
        f,
        dialog,
        styles,
        rows[4],
        &[
            RenameField::CounterStart,
            RenameField::CounterStep,
            RenameField::CounterDigits,
        ],
    );
    f.render_widget(kit::separator(inner.width, kit::fg(Color::Cyan)), rows[5]);
    preview::render(f, dialog, theme, rows[6]);
    f.render_widget(kit::hint(t("multi_rename_placeholders")), rows[7]);
    let rename_style = if dialog.can_run() {
        styles
    } else {
        kit::FocusStyles {
            normal: kit::fg(Color::DarkGray),
            ..styles
        }
    };
    f.render_widget(
        kit::button_bar(
            &[t("multi_rename_button"), t("btn_cancel_bracket")],
            Dialog::FORM.focused_button(dialog.focus),
            rename_style,
        ),
        rows[8],
    );
    true
}

/// "N files, M to rename, K conflicts", or the regex error in red.
fn summary(dialog: &Dialog, normal: Style) -> Paragraph<'static> {
    if let Some(error) = &dialog.error {
        let text =
            t("multi_rename_regex_error").replacen("{}", error.lines().last().unwrap_or(""), 1);
        return Paragraph::new(format!(" {text}")).style(kit::fg(Color::LightRed));
    }
    let preview = &dialog.preview;
    let text = t("multi_rename_summary")
        .replacen("{}", &dialog.sources.len().to_string(), 1)
        .replacen("{}", &preview.changes().to_string(), 1)
        .replacen("{}", &preview.conflicts().to_string(), 1);
    let style = if preview.conflicts() > 0 {
        kit::fg(Color::LightRed)
    } else {
        normal
    };
    Paragraph::new(format!(" {text}")).style(style)
}

/// `fields` side by side in equal columns.
fn render_fields(
    f: &mut Frame,
    dialog: &Dialog,
    styles: kit::FocusStyles,
    area: Rect,
    fields: &[RenameField],
) {
    let columns = equal_columns(area, fields.len());
    for (field, column) in fields.iter().zip(columns.iter()) {
        let label = t(field.label_key());
        let focused = dialog.focus == field.row();
        f.render_widget(
            kit::inline_field(&label, dialog.field(*field), focused, styles),
            *column,
        );
    }
}

/// Regex and ignore-case checkboxes and the case selector.
fn render_options(f: &mut Frame, dialog: &Dialog, styles: kit::FocusStyles, area: Rect) {
    let columns = equal_columns(area, 3);
    let cells = [
        (
            Dialog::ROW_REGEX,
            kit::checkbox_row(dialog.regex, &t("multi_rename_regex")),
        ),
        (
            Dialog::ROW_IGNORE_CASE,
            kit::checkbox_row(dialog.ignore_case, &t("multi_rename_ignore_case")),
        ),
        (
            Dialog::ROW_CASE,
            format!(
                "{} < {} >",
                t("multi_rename_case"),
                t(dialog.case.label_key())
            ),
        ),
    ];
    for ((row, text), column) in cells.iter().zip(columns.iter()) {
        f.render_widget(kit::marked_row(text, dialog.focus == *row, styles), *column);
    }
}

fn equal_columns(area: Rect, count: usize) -> std::rc::Rc<[Rect]> {
    let count = u32::try_from(count.max(1)).unwrap_or(1);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints((0..count).map(|_| Constraint::Ratio(1, count)))
        .split(area)
}
