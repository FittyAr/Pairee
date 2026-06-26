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
    if let PopupType::ConfirmDelete { paths, cursor_idx } = popup {
        let area = centered_rect_fixed(50, 8, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(t("prompt_delete_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Length(2)])
            .split(inner);

        let mut folders_count = 0;
        let mut files_count = 0;
        for p in paths {
            if p.is_dir()
                && !p
                    .symlink_metadata()
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
            {
                folders_count += 1;
            } else {
                files_count += 1;
            }
        }

        let part_files = if files_count > 0 {
            Some(t("delete_confirm_files").replacen("{}", &files_count.to_string(), 1))
        } else {
            None
        };

        let part_folders = if folders_count > 0 {
            Some(t("delete_confirm_folders").replacen("{}", &folders_count.to_string(), 1))
        } else {
            None
        };

        let target_desc = match (part_files, part_folders) {
            (Some(f), Some(d)) => format!("{}{}{}", f, t("delete_confirm_conjunction"), d),
            (Some(f), None) => f,
            (None, Some(d)) => d,
            (None, None) => "0 items".to_string(),
        };

        let text = t("delete_confirm_msg").replacen("{}", &target_desc, 1);

        f.render_widget(
            Paragraph::new(text)
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(parse_color(&theme.popup_fg))),
            chunks[0],
        );

        let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
        let normal_style = Style::default().fg(parse_color(&theme.popup_fg));
        let btn1 = if *cursor_idx == 0 {
            active_style
        } else {
            normal_style
        };
        let btn2 = if *cursor_idx == 1 {
            active_style
        } else {
            normal_style
        };
        let btns = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(t("btn_delete_bracket"), btn1),
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(t("btn_cancel_bracket"), btn2),
        ]);
        f.render_widget(
            Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
        true
    } else {
        false
    }
}
