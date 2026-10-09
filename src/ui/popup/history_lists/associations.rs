use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::scrollbar::ScrollbarUiState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub(super) fn truncate_str(s: &str, max_len: usize) -> String {
    crate::ui::text_width::truncate_to_width(s, max_len)
}

type Theme = crate::config::theme::Theme;

/// Widths of the mask, open command and view command columns.
struct Columns {
    mask: usize,
    open: usize,
    view: usize,
}

impl Columns {
    /// Splits `width` 46/26/rest after the separators.
    fn for_width(width: usize) -> Self {
        let available_width = width.saturating_sub(8);
        let mask = available_width * 46 / 100;
        let open = available_width * 26 / 100;
        let view = available_width.saturating_sub(mask).saturating_sub(open);
        Self { mask, open, view }
    }

    fn row(&self, mask: &str, open: &str, view: &str) -> String {
        format!(
            " {:<mw$} | {:<ow$} | {:<vw$} ",
            truncate_str(mask, self.mask),
            truncate_str(open, self.open),
            truncate_str(view, self.view),
            mw = self.mask,
            ow = self.open,
            vw = self.view
        )
    }
}

pub(super) fn render_associations(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let PopupType::FileAssociationsDialog {
        rules,
        cursor_idx,
        editing_idx,
        editing_field,
        edit_buffer,
        original_rule: _,
    } = popup
    else {
        return false;
    };
    let area = centered_rect(75, 60, size);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(t("prompt_associations_title"))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let editing = editing_idx.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if editing {
            vec![
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
        } else {
            vec![Constraint::Min(0), Constraint::Length(1)]
        })
        .split(inner);

    // 1. Renderizar Listado de Reglas
    let list_paragraph = Paragraph::new(rule_lines(rules, *cursor_idx, chunks[0], theme))
        .style(Style::default().fg(parse_color(&theme.popup_fg)));
    f.render_widget(list_paragraph, chunks[0]);

    // 2. Renderizar Cuadro de Edición
    if editing {
        render_edit_box(f, chunks[1], *editing_field, edit_buffer, theme);
    }

    // 3. Renderizar Leyenda/Hint de Teclas al pie
    let hint_text = if editing {
        t("associations_hint_edit")
    } else if rules.is_empty() {
        t("associations_hint_nav_empty")
    } else {
        t("associations_hint_nav")
    };
    let hint_paragraph = Paragraph::new(Span::styled(
        hint_text,
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(hint_paragraph, chunks[if editing { 2 } else { 1 }]);
    true
}

/// Header and the rules visible around the cursor (or a "no rules" note).
fn rule_lines(
    rules: &[crate::config::associations::AssocRule],
    cursor_idx: usize,
    area: Rect,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let columns = Columns::for_width(area.width as usize);
    let mut list_lines = vec![Line::from(vec![Span::styled(
        columns.row(
            &t("col_mask"),
            &t("col_open_command"),
            &t("col_view_command"),
        ),
        Style::default().add_modifier(Modifier::UNDERLINED),
    )])];

    if rules.is_empty() {
        list_lines.push(Line::from(""));
        list_lines.push(Line::from(Span::styled(
            format!("   {}", t("associations_no_rules")),
            Style::default().fg(parse_color(&theme.popup_fg)),
        )));
        return list_lines;
    }
    let list_height = area.height.saturating_sub(1) as usize;
    let scroll_start = cursor_idx.saturating_sub(list_height / 2);
    let same_as_open = t("associations_same_as_open");
    let cursor_style = Style::default()
        .bg(parse_color(&theme.selection_bg))
        .fg(parse_color(&theme.selection_fg))
        .add_modifier(Modifier::BOLD);
    let normal = Style::default().fg(parse_color(&theme.popup_fg));
    for (i, rule) in rules
        .iter()
        .enumerate()
        .skip(scroll_start)
        .take(list_height)
    {
        let view_cmd_str = rule.view_cmd.as_deref().unwrap_or(&same_as_open);
        let line_str = columns.row(&rule.mask, &rule.open_cmd, view_cmd_str);
        let style = if i == cursor_idx {
            cursor_style
        } else {
            normal
        };
        list_lines.push(Line::from(Span::styled(line_str, style)));
    }
    list_lines
}

/// Box editing one field of the rule under the cursor.
fn render_edit_box(
    f: &mut Frame,
    area: Rect,
    editing_field: usize,
    edit_buffer: &crate::app::text_input::TextField,
    theme: &Theme,
) {
    let field_label = match editing_field {
        0 => t("associations_editing_mask"),
        1 => t("associations_editing_open"),
        2 => t("associations_editing_view"),
        _ => String::new(),
    };
    let style = crate::ui::popup::kit::popup_fg(theme);
    let mut spans = vec![Span::styled(format!(" {} ", field_label), style)];
    spans.extend(crate::ui::popup::kit::field_spans(
        edit_buffer,
        style,
        crate::ui::popup::kit::selection(theme),
        true,
    ));
    let edit_paragraph = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(edit_paragraph, area);
}
