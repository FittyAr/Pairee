use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::MkDirPrompt {
        input,
        cursor_idx,
        process_multiple,
    } = popup
    {
        let area = centered_rect_fixed(50, 9, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(parse_color(&theme.popup_border)))
            .title(t("prompt_mkdir_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .split(inner);

        let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
        let normal_style = Style::default().fg(parse_color(&theme.popup_fg));

        let input_style = if *cursor_idx == 0 {
            active_style
        } else {
            normal_style
        };
        let display_input = if *cursor_idx == 0 {
            format!("{}_", input)
        } else {
            input.clone()
        };
        f.render_widget(
            Paragraph::new(format!("{}\n > {}", t("prompt_mkdir_to"), display_input))
                .style(input_style),
            chunks[0],
        );

        let chk = if *process_multiple { "[x]" } else { "[ ]" };
        let multi_style = if *cursor_idx == 1 {
            active_style
        } else {
            normal_style
        };
        f.render_widget(
            Paragraph::new(format!("{} {}", chk, t("prompt_process_multiple_names")))
                .style(multi_style),
            chunks[1],
        );

        let btn1 = if *cursor_idx == 2 {
            active_style
        } else {
            normal_style
        };
        let btn2 = if *cursor_idx == 3 {
            active_style
        } else {
            normal_style
        };
        let btns = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(t("btn_ok_bracket"), btn1),
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(t("btn_cancel_bracket"), btn2),
        ]);
        f.render_widget(
            Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
            chunks[2],
        );
        true
    } else {
        false
    }
}
