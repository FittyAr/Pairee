//! The connection manager dialog: saved connections on the left, the
//! fields of the shown one on the right, the buttons below.

use super::super::centered_rect_fixed;
use crate::app::state::ConnectionPromptState;
use crate::app::state::popup::ConnectionField;
use crate::config::localization::t;
use crate::config::settings::Connection;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Width of the list column.
const LIST_WIDTH: u16 = 25;
/// Width of a field label (`" {label:<15} "`).
const LABEL_WIDTH: u16 = 17;

impl DialogView for ConnectionPromptState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, saved) = (cx.theme, &cx.context.config.settings.connections);
        let rows = ConnectionField::ALL.len() as u16;
        // Borders, title row, fields, separator and buttons.
        let area = centered_rect_fixed(78, rows + 5, cx.area);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ui.accent))
            .title(t("prompt_conn_title"))
            .style(Style::default().bg(theme.popup_bg));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [form_area, sep_area, buttons_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(rows + 1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(inner);
        let [list_area, bar_area, fields_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(LIST_WIDTH),
                Constraint::Length(1),
                Constraint::Min(30),
            ])
            .areas(form_area);

        let styles = kit::FocusStyles::from_theme(theme);
        let form = kit::mouse::FormFocus::new(FORM, self.cursor_idx);
        let list = List::new(list_items(self, saved, styles, theme))
            .block(Block::default().title(format!(" {} ", t("prompt_conn_list_title").trim())))
            .style(Style::default().bg(theme.popup_bg));
        f.render_widget(list, list_area);
        register_list(list_area, self, saved.len(), form);

        let bar = ratatui::symbols::line::VERTICAL;
        for y in bar_area.y..bar_area.bottom() {
            f.render_widget(
                Paragraph::new(bar).style(Style::default().fg(theme.ui.accent)),
                Rect::new(bar_area.x, y, 1, 1),
            );
        }
        render_fields(f, fields_area, self, (styles, form), theme);
        f.render_widget(
            kit::separator(inner.width, Style::default().fg(theme.ui.accent)),
            sep_area,
        );
        let buttons = [
            "btn_connect_braced",
            "btn_save_preset",
            "btn_delete_preset",
            "btn_cancel_bracket",
        ]
        .map(|key| format!(" {} ", t(key)));
        kit::form_buttons(f, buttons_area, &buttons, form, styles);
    }
}

/// Focus order of the dialog for the mouse: the list, the fields, then the
/// buttons (Tab walks them in that order).
const FORM: crate::app::form::FormLayout = crate::app::form::FormLayout::new(
    ConnectionPromptState::ROW_COUNT,
    ConnectionPromptState::BUTTON_CONNECT,
);

/// A click on a saved connection focuses the list and walks to it with the
/// arrows (which load it).
fn register_list(
    area: Rect,
    prompt: &ConnectionPromptState,
    count: usize,
    form: kit::mouse::FormFocus,
) {
    use crossterm::event::KeyCode;
    let current = prompt.selected_idx.unwrap_or(0) as i64;
    // The list's title takes the first row.
    for (i, y) in (0..count).zip(area.y + 1..area.bottom()) {
        let mut keys = form.keys_to(ConnectionPromptState::ROW_LIST);
        keys.extend(kit::mouse::presses(
            i as i64 - current,
            KeyCode::Down,
            KeyCode::Up,
        ));
        kit::mouse::keys(Rect::new(area.x, y, area.width, 1), keys);
    }
}

/// Saved connections with their protocol; the shown one is highlighted
/// (strongly while the list has focus).
fn list_items(
    prompt: &ConnectionPromptState,
    saved: &[Connection],
    styles: kit::FocusStyles,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) -> Vec<ListItem<'static>> {
    if saved.is_empty() {
        let none = format!(" {} ", t("prompt_conn_none"));
        return vec![ListItem::new(Line::from(Span::styled(
            none,
            Style::default().fg(theme.ui.muted),
        )))];
    }
    let list_focused = prompt.cursor_idx == ConnectionPromptState::ROW_LIST;
    saved
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let current = Some(i) == prompt.selected_idx;
            let style = match (current, list_focused) {
                (true, true) => styles.active,
                (true, false) => Style::default().bg(theme.ui.muted).fg(theme.ui.strong),
                _ => styles.normal,
            };
            let text = format!(" {} ({}) ", c.name, t(c.kind.label_key()));
            ListItem::new(Line::from(Span::styled(text, style)))
        })
        .collect()
}

/// Title and one row per field.
fn render_fields(
    f: &mut Frame,
    area: Rect,
    prompt: &ConnectionPromptState,
    (styles, form): (kit::FocusStyles, kit::mouse::FormFocus),
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    f.render_widget(
        Paragraph::new(format!(" {}", t("prompt_conn_details_title")))
            .style(Style::default().fg(theme.ui.warning)),
        Rect::new(area.x, area.y, area.width, 1),
    );
    for (field, y) in ConnectionField::ALL
        .into_iter()
        .zip(area.y + 1..area.bottom())
    {
        let row = Rect::new(area.x, y, area.width, 1);
        let (line, hit) = field_line(prompt, field, styles, row);
        f.render_widget(line, row);
        match hit {
            Some(hit) => form.field(row, field.row(), hit),
            None => form.option(row, field.row()),
        }
    }
}

/// `" Label:          value"` for one row drawn at `row` (password
/// masked, protocol between arrows), with a cursor when focused, and where
/// a text value went (for clicks).
fn field_line(
    prompt: &ConnectionPromptState,
    field: ConnectionField,
    styles: kit::FocusStyles,
    row: Rect,
) -> (Paragraph<'static>, Option<kit::FieldHit>) {
    let focused = prompt.cursor_idx == field.row();
    let style = styles.pick(focused);
    let mut label = t(field.label_key(prompt.kind)).trim().to_string();
    if !label.ends_with(':') {
        label.push(':');
    }
    let width = usize::from(LABEL_WIDTH - 2);
    let mut spans = vec![Span::styled(format!(" {label:<width$} "), style)];
    let room = usize::from(row.width.saturating_sub(LABEL_WIDTH));
    let text_x = row.x + LABEL_WIDTH;
    let hit = match (field, prompt.text_field(field)) {
        (ConnectionField::Kind, _) | (_, None) => {
            let kind = t(prompt.kind.label_key());
            spans.push(Span::styled(format!("< {kind} >"), style));
            None
        }
        (ConnectionField::Password, Some(value)) => {
            let shown = kit::masked(value);
            let view = kit::FieldView::new(&shown, room);
            spans.extend(view.spans(style, styles.cursor(), focused));
            Some(view.hit(text_x))
        }
        (_, Some(value)) => {
            let view = kit::FieldView::new(value, room);
            spans.extend(view.spans(style, styles.cursor(), focused));
            Some(view.hit(text_x))
        }
    };
    (Paragraph::new(Line::from(spans)), hit)
}
