use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::RenamePrompt {
        input,
        original,
        src_path,
        parent_dir,
        cursor_idx,
    } = popup
    {
        let area = centered_rect_fixed(60, 9, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(t("prompt_rename_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Length(1), // input field
                Constraint::Length(1), // collision warning
                Constraint::Length(1), // separator
                Constraint::Length(1), // buttons
                Constraint::Length(1), // hint
            ])
            .split(inner);

        let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
        let normal_style = Style::default().fg(parse_color(&theme.popup_fg));

        f.render_widget(
            Paragraph::new(format!("{} {}", t("prompt_rename_to"), original)).style(normal_style),
            chunks[0],
        );

        let display_input = if *cursor_idx == 0 {
            format!("{}_", input)
        } else {
            input.clone()
        };
        f.render_widget(
            Paragraph::new(format!("> {}", display_input)).style(if *cursor_idx == 0 {
                active_style
            } else {
                normal_style
            }),
            chunks[1],
        );

        // Live collision warning: only if the typed name actually differs from
        // the original AND a sibling with that name already exists.
        let collision = if input != original {
            let target = parent_dir.join(input);
            target != *src_path && target.exists()
        } else {
            false
        };

        if collision {
            let warn_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
            f.render_widget(
                Paragraph::new(format!("[!] {}", t("prompt_rename_collision"))).style(warn_style),
                chunks[2],
            );
        }

        let sep_str = ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
        let sep_style = Style::default().fg(Color::Yellow);
        f.render_widget(Paragraph::new(sep_str).style(sep_style), chunks[3]);

        let btn_ok = if *cursor_idx == 1 {
            active_style
        } else {
            normal_style
        };
        let btn_cancel = if *cursor_idx == 2 {
            active_style
        } else {
            normal_style
        };
        let btns = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(t("btn_ok_bracket"), btn_ok),
            ratatui::text::Span::raw("    "),
            ratatui::text::Span::styled(t("btn_cancel_bracket"), btn_cancel),
        ]);
        f.render_widget(
            Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
            chunks[4],
        );

        let hint = ratatui::text::Line::from(ratatui::text::Span::styled(
            t("prompt_rename_hint"),
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(
            Paragraph::new(hint).alignment(ratatui::layout::Alignment::Center),
            chunks[5],
        );

        true
    } else {
        false
    }
}
