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

pub(super) fn render_associations(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    match popup {
        PopupType::FileAssociationsDialog {
            rules,
            cursor_idx,
            editing_idx,
            editing_field,
            edit_buffer,
            original_rule: _,
        } => {
            let area = centered_rect(75, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("prompt_associations_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(if editing_idx.is_some() {
                    vec![
                        Constraint::Min(0),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ]
                } else {
                    vec![Constraint::Min(0), Constraint::Length(1)]
                })
                .split(inner);

            let width_total = chunks[0].width as usize;
            let available_width = width_total.saturating_sub(8);
            let mask_width = available_width * 46 / 100;
            let open_width = available_width * 26 / 100;
            let view_width = available_width
                .saturating_sub(mask_width)
                .saturating_sub(open_width);

            // 1. Renderizar Listado de Reglas
            let mut list_lines = Vec::new();
            list_lines.push(Line::from(vec![Span::styled(
                format!(
                    " {:<mw$} | {:<ow$} | {:<vw$} ",
                    truncate_str(&t("col_mask"), mask_width),
                    truncate_str(&t("col_open_command"), open_width),
                    truncate_str(&t("col_view_command"), view_width),
                    mw = mask_width,
                    ow = open_width,
                    vw = view_width
                ),
                Style::default().add_modifier(Modifier::UNDERLINED),
            )]));

            if rules.is_empty() {
                list_lines.push(Line::from(""));
                list_lines.push(Line::from(Span::styled(
                    format!("   {}", t("associations_no_rules")),
                    Style::default().fg(parse_color(&theme.popup_fg)),
                )));
            } else {
                let list_height = chunks[0].height.saturating_sub(1) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let same_as_open = t("associations_same_as_open");

                for (i, rule) in rules
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let view_cmd_str = rule.view_cmd.as_deref().unwrap_or(&same_as_open);
                    let mask_truncated = truncate_str(&rule.mask, mask_width);
                    let open_truncated = truncate_str(&rule.open_cmd, open_width);
                    let view_truncated = truncate_str(view_cmd_str, view_width);
                    let line_str = format!(
                        " {:<mw$} | {:<ow$} | {:<vw$} ",
                        mask_truncated,
                        open_truncated,
                        view_truncated,
                        mw = mask_width,
                        ow = open_width,
                        vw = view_width
                    );
                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    list_lines.push(Line::from(Span::styled(line_str, style)));
                }
            }

            let list_paragraph =
                Paragraph::new(list_lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(list_paragraph, chunks[0]);

            // 2. Renderizar Cuadro de Edición
            if editing_idx.is_some() {
                let field_label = match editing_field {
                    0 => t("associations_editing_mask"),
                    1 => t("associations_editing_open"),
                    2 => t("associations_editing_view"),
                    _ => String::new(),
                };

                // Cursor simulado
                let cursor_char = if (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    / 500)
                    .is_multiple_of(2)
                {
                    "_"
                } else {
                    " "
                };

                let edit_text = format!(" {} {}{}", field_label, edit_buffer, cursor_char);
                let edit_paragraph = Paragraph::new(edit_text).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                );
                f.render_widget(edit_paragraph, chunks[1]);
            }

            // 3. Renderizar Leyenda/Hint de Teclas al pie
            let hint_text = if editing_idx.is_some() {
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
            let hint_area = if editing_idx.is_some() {
                chunks[2]
            } else {
                chunks[1]
            };
            f.render_widget(hint_paragraph, hint_area);

            true
        }
        _ => false,
    }
}
