pub mod highlight;
pub mod widget;

pub use widget::render_editor_widget;

use super::{centered_rect, centered_rect_fixed};
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_editor_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::EditorSearchPrompt {
            query,
            case_sensitive,
            cursor_idx,
        } => {
            let search_area = centered_rect_fixed(50, 9, size);
            f.render_widget(Clear, search_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("editor_search_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(search_area);
            f.render_widget(block, search_area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // 0: Search query prompt + input line
                    Constraint::Length(1), // 1: Separator line
                    Constraint::Length(1), // 2: Case sensitive checkbox
                    Constraint::Min(1),    // 3: Spacer
                    Constraint::Length(1), // 4: Buttons
                ])
                .split(inner);

            let act_style = Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg));
            let norm_style = Style::default().fg(parse_color(&theme.popup_fg));

            // Search query text
            let q_pref = if *cursor_idx == 0 { "► " } else { "  " };
            let q_style = if *cursor_idx == 0 {
                act_style
            } else {
                norm_style
            };
            let q_text = if *cursor_idx == 0 {
                format!("{}_", query)
            } else {
                query.clone()
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{}{}\n   > {}",
                    q_pref,
                    t("search_query_label"),
                    q_text
                ))
                .style(q_style),
                chunks[0],
            );

            // Separator
            let sep_style = Style::default().fg(parse_color(&theme.popup_border));
            let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
            f.render_widget(Paragraph::new(sep_str).style(sep_style), chunks[1]);

            // Case sensitive checkbox
            let cs_pref = if *cursor_idx == 1 { "► " } else { "  " };
            let cs_style = if *cursor_idx == 1 {
                act_style
            } else {
                norm_style
            };
            let cs_chk = if *case_sensitive { "[x]" } else { "[ ]" };
            f.render_widget(
                Paragraph::new(format!("{}{} {}", cs_pref, cs_chk, t("sys_case_sensitive")))
                    .style(cs_style),
                chunks[2],
            );

            // Buttons
            let b1_style = if *cursor_idx == 2 {
                act_style
            } else {
                norm_style
            };
            let b2_style = if *cursor_idx == 3 {
                act_style
            } else {
                norm_style
            };
            let btns = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(t("btn_search_bracket"), b1_style),
                ratatui::text::Span::raw("  "),
                ratatui::text::Span::styled(t("btn_cancel_bracket"), b2_style),
            ]);
            f.render_widget(
                Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
                chunks[4],
            );

            true
        }
        PopupType::ConfirmDiscardEditorChanges => {
            let area = centered_rect(50, 20, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(parse_color(&theme.popup_border)))
                .title(t("editor_discard_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));
            let text = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                t("editor_discard_prompt"),
                Style::default().fg(parse_color(&theme.popup_fg)),
            )]);
            let para = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(para, area);
            true
        }
        _ => false,
    }
}
