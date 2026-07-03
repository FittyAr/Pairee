//! Renderers for the three plugin-spawned popups (M1).
//!
//! - `PluginInputDialog` — centered modal with a single-line text
//!   input. Supports `obscure` mode (renders `*` per character).
//! - `PluginConfirmDialog` — centered modal with a `[Yes] [No]`
//!   button pair; the active button is rendered in reverse style.
//! - `PluginWhichPrompt` — when `silent = false`, lists the candidate
//!   descriptions (and their `on` keys) inside a small box. When
//!   `silent = true`, no visual is rendered but the matching key
//!   handler still listens.
//!
//! Each renderer returns `true` to indicate the popup was handled,
//! or `false` to fall through to the next renderer in the chain.

use super::super::centered_rect_fixed;
use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::PluginInputDialog {
            title,
            input,
            cursor_idx,
            obscure,
            reply_tx: _,
        } => {
            let area = centered_rect_fixed(60, 7, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title.clone())
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            // When `obscure` is true, mask the typed value.
            let display: String = if *obscure {
                std::iter::repeat('*').take(input.chars().count()).collect()
            } else {
                input.clone()
            };

            // Render the cursor: a reverse-styled `_` at the cursor
            // position (or at the end if the cursor is past the input).
            let chars: Vec<char> = display.chars().collect();
            let cursor = (*cursor_idx).min(chars.len());
            let before: String = chars[..cursor].iter().collect();
            let after: String = chars[cursor..].iter().collect();
            let cursor_char = if cursor < chars.len() {
                chars[cursor].to_string()
            } else {
                "_".to_string()
            };

            let cursor_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let line = Line::from(vec![
                Span::raw(before),
                Span::styled(cursor_char, cursor_style),
                Span::raw(after),
            ]);
            let hint = Line::from(Span::styled(
                " [Enter] Submit   [Esc] Cancel",
                Style::default().fg(Color::DarkGray),
            ));

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // input line
                    Constraint::Length(1), // spacer
                    Constraint::Length(1), // hint
                ])
                .split(block.inner(area));

            let input_para = Paragraph::new(line);
            let hint_para = Paragraph::new(hint);

            f.render_widget(block, area);
            f.render_widget(input_para, chunks[0]);
            f.render_widget(hint_para, chunks[2]);
            true
        }
        PopupType::PluginConfirmDialog {
            title,
            msg,
            cursor_idx,
            reply_tx: _,
        } => {
            let area = centered_rect_fixed(50, 9, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title.clone())
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            // Wrap the message across the inner area.
            let msg_para = Paragraph::new(msg.clone()).wrap(Wrap { trim: true });
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // message (wrapped up to 3 lines)
                    Constraint::Length(1), // spacer
                    Constraint::Length(1), // buttons
                ])
                .split(block.inner(area));

            let yes_style = if *cursor_idx == 0 {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            let no_style = if *cursor_idx == 1 {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Red)
            };
            let buttons = Line::from(vec![
                Span::raw("  "),
                Span::styled(" [Yes] ", yes_style),
                Span::raw("  "),
                Span::styled(" [No] ", no_style),
                Span::raw("   "),
                Span::styled(
                    "\u{2190}\u{2192} switch   [Enter] Confirm   [Esc] Cancel",
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            f.render_widget(block, area);
            f.render_widget(msg_para, chunks[0]);
            f.render_widget(Paragraph::new(buttons), chunks[2]);
            true
        }
        PopupType::PluginWhichPrompt {
            candidates,
            silent,
            reply_tx: _,
        } => {
            if *silent {
                // Silent prompts render nothing; the handler still
                // listens for candidate keys.
                return true;
            }
            let count = candidates.len() as u16;
            let height = (count + 4).min(size.height);
            let area = centered_rect_fixed(50, height, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Press a key ")
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let mut lines: Vec<Line> = Vec::new();
            for cand in candidates {
                let keys = cand.on.join(" / ");
                let desc = cand.desc.clone().unwrap_or_default();
                if desc.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", keys),
                        Style::default().fg(Color::Cyan),
                    )));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {:<10}", keys),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw(desc),
                    ]));
                }
            }
            let para = Paragraph::new(lines);
            f.render_widget(&block, area);
            f.render_widget(para, block.inner(area));
            true
        }
        _ => false,
    }
}
