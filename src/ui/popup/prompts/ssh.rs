use super::super::centered_rect_fixed;
use crate::app::context::AppContext;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    context: &AppContext,
) -> bool {
    if let PopupType::SshConnectPrompt {
        panel: _,
        input_name,
        input_host,
        input_port,
        input_user,
        input_pass,
        input_key_path,
        cursor_idx,
        selected_preset_idx,
    } = popup
    {
        let area = centered_rect_fixed(75, 12, size);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(t("prompt_ssh_title"))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),    // Form columns
                Constraint::Length(1), // Separator
                Constraint::Length(1), // Buttons
            ])
            .split(inner);

        let form_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // Presets list
                Constraint::Length(1),  // Vertical separator
                Constraint::Min(30),    // Inputs
            ])
            .split(main_chunks[0]);

        let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
        let normal_style = Style::default().fg(parse_color(&theme.popup_fg));

        // Left column: Presets List
        let presets = &context.config.settings.ssh_presets;
        let mut list_items = Vec::new();
        if presets.is_empty() {
            list_items.push(ListItem::new(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(
                    " <No Presets> ",
                    Style::default().fg(Color::DarkGray),
                ),
            ])));
        } else {
            for (i, p) in presets.iter().enumerate() {
                let is_current = Some(i) == *selected_preset_idx;
                let is_active_field = *cursor_idx == 0;

                let style = if is_current && is_active_field {
                    active_style
                } else if is_current {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    normal_style
                };

                list_items.push(ListItem::new(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("  {}  ", p.name), style),
                ])));
            }
        }

        let presets_block = Block::default()
            .borders(Borders::NONE)
            .title(format!(" {} ", t("ssh_presets_title").trim()));
        let list = List::new(list_items)
            .block(presets_block)
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        f.render_widget(list, form_chunks[0]);

        // Vertical separator
        let sep_str_vertical = ratatui::symbols::line::VERTICAL;
        for y in form_chunks[1].y..(form_chunks[1].y + form_chunks[1].height) {
            f.render_widget(
                Paragraph::new(sep_str_vertical).style(Style::default().fg(Color::Cyan)),
                Rect::new(form_chunks[1].x, y, 1, 1),
            );
        }

        // Right column: Inputs
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer / Title
                Constraint::Length(1), // Name
                Constraint::Length(1), // Host
                Constraint::Length(1), // Port
                Constraint::Length(1), // Username
                Constraint::Length(1), // Password
                Constraint::Length(1), // Key Path
            ])
            .split(form_chunks[2]);

        let pad_label = |lbl: &str, width: usize| {
            let mut s = lbl.to_string();
            if !s.ends_with(':') {
                s.push(':');
            }
            format!(" {:<width$} ", s, width = width)
        };

        let l_name = pad_label(&t("prompt_ssh_name").trim(), 14);
        let l_host = pad_label(&t("prompt_ssh_host").trim(), 14);
        let l_port = pad_label(&t("prompt_ssh_port").trim(), 14);
        let l_user = pad_label(&t("prompt_ssh_user").trim(), 14);
        let l_pass = pad_label(&t("prompt_ssh_pass").trim(), 14);
        let l_key = pad_label(&t("prompt_ssh_key_path").trim(), 14);

        let render_input_line =
            |f: &mut Frame, chunk: Rect, label: &str, val: &str, idx: usize, is_pass: bool| {
                let is_active = *cursor_idx == idx;
                let style = if is_active {
                    active_style
                } else {
                    normal_style
                };

                let val_disp = if is_pass {
                    "*".repeat(val.len())
                } else {
                    val.to_string()
                };

                let text = if is_active {
                    format!("{}{}_", label, val_disp)
                } else {
                    format!("{}{}", label, val_disp)
                };

                f.render_widget(Paragraph::new(text).style(style), chunk);
            };

        f.render_widget(
            Paragraph::new(format!(" {}", t("ssh_details_title")))
                .style(Style::default().fg(Color::Yellow)),
            input_chunks[0],
        );

        render_input_line(f, input_chunks[1], &l_name, input_name, 1, false);
        render_input_line(f, input_chunks[2], &l_host, input_host, 2, false);
        render_input_line(f, input_chunks[3], &l_port, input_port, 3, false);
        render_input_line(f, input_chunks[4], &l_user, input_user, 4, false);
        render_input_line(f, input_chunks[5], &l_pass, input_pass, 5, true);
        render_input_line(f, input_chunks[6], &l_key, input_key_path, 6, false);

        // Bottom horizontal separator
        let sep_str_horizontal =
            ratatui::symbols::line::HORIZONTAL.repeat(inner.width as usize);
        f.render_widget(
            Paragraph::new(sep_str_horizontal).style(Style::default().fg(Color::Cyan)),
            main_chunks[1],
        );

        // Buttons at the bottom
        let b_connect = if *cursor_idx == 7 {
            active_style
        } else {
            normal_style
        };
        let b_save = if *cursor_idx == 8 {
            active_style
        } else {
            normal_style
        };
        let b_delete = if *cursor_idx == 9 {
            active_style
        } else {
            normal_style
        };
        let b_cancel = if *cursor_idx == 10 {
            active_style
        } else {
            normal_style
        };

        let btns = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!(" {} ", t("btn_connect_braced")), b_connect),
            ratatui::text::Span::raw("    "),
            ratatui::text::Span::styled(format!(" {} ", t("btn_save_preset")), b_save),
            ratatui::text::Span::raw("    "),
            ratatui::text::Span::styled(format!(" {} ", t("btn_delete_preset")), b_delete),
            ratatui::text::Span::raw("    "),
            ratatui::text::Span::styled(format!(" {} ", t("btn_cancel_bracket")), b_cancel),
        ]);
        f.render_widget(
            Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
            main_chunks[2],
        );

        true
    } else {
        false
    }
}
