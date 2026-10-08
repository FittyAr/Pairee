//! Options form of the Synchronize folders dialog.

use crate::app::state::popup::SyncDialog;
use crate::app::state::popup::sync::{OPTIONS_FORM, rows};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::kit::{self, FocusStyles};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

pub(super) fn render(f: &mut Frame, dialog: &SyncDialog, theme: &Theme, size: Rect) {
    let inner = kit::dialog_frame(
        f,
        size,
        (72, 12),
        super::title(dialog.options.direction),
        Style::default().fg(Color::Yellow),
        theme,
    );
    let mut constraints = vec![Constraint::Length(1); 6];
    constraints.extend([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);
    let styles = FocusStyles::from_theme(theme);
    let fg = kit::popup_fg(theme);
    let path_line = |key: &str, path: &std::path::Path| {
        Paragraph::new(t(key).replacen("{}", &path.display().to_string(), 1)).style(fg)
    };
    f.render_widget(path_line("sync_left_path", &dialog.left), chunks[0]);
    f.render_widget(path_line("sync_right_path", &dialog.right), chunks[1]);
    f.render_widget(kit::separator(inner.width, fg), chunks[2]);

    let focus = dialog.focus;
    let direction =
        t("sync_direction_row").replacen("{}", &t(dialog.options.direction.label_key()), 1);
    let content = t("sync_compare_content").replacen("{}", dialog.algorithm.as_str(), 1);
    let option_rows = [
        (rows::DIRECTION, direction),
        (
            rows::CONTENT,
            kit::checkbox_row(dialog.options.content_hash.is_some(), &content),
        ),
        (
            rows::HIDDEN,
            kit::checkbox_row(dialog.options.ignore_hidden, &t("sync_ignore_hidden")),
        ),
    ];
    for (row, text) in option_rows {
        f.render_widget(
            kit::marked_row(&text, focus == row, styles),
            chunks[3 + row],
        );
    }
    f.render_widget(
        kit::labelled_field(&t("sync_mask"), &dialog.mask, focus == rows::MASK, styles),
        chunks[6],
    );
    f.render_widget(kit::separator(inner.width, fg), chunks[7]);
    let buttons = [t("sync_btn_compare"), t("btn_cancel_bracket")];
    f.render_widget(
        kit::button_bar(&buttons, OPTIONS_FORM.focused_button(focus), styles),
        chunks[8],
    );
}
