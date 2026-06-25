use super::super::centered_rect;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::Help {
        mode,
        docs,
        cursor_idx,
        scroll_y,
        active_content,
    } = popup
    {
        let area = centered_rect(90, 85, size); // Expand to 90% width, 85% height
        f.render_widget(Clear, area);

        use ratatui::text::Span;

        // Split into Left (list) and Right (content viewer)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // 25% for document list
                Constraint::Percentage(75), // 75% for content
            ])
            .split(area);
        let left_area = chunks[0];
        let right_area = chunks[1];

        // 1. Render Left panel (document selection list)
        let left_title = format!(" {} ", t("prompt_help_title").trim());
        let left_border_color = if *mode == 0 {
            Color::Yellow
        } else {
            parse_color(&theme.popup_border)
        };
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(left_border_color))
            .title(left_title)
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let mut list_items = Vec::new();
        for (i, (doc_title, _)) in docs.iter().enumerate() {
            let style = if i == *cursor_idx {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            list_items.push(ListItem::new(ratatui::text::Line::from(vec![
                Span::styled(format!("  {}  ", doc_title), style),
            ])));
        }

        let list = List::new(list_items)
            .block(left_block)
            .style(Style::default().bg(parse_color(&theme.popup_bg)));
        f.render_widget(list, left_area);

        // 2. Render Right panel (content viewer)
        let doc_title = docs
            .get(*cursor_idx)
            .map(|(t, _)| t.as_str())
            .unwrap_or(" Documentation ");
        let right_border_color = if *mode == 1 {
            Color::Yellow
        } else {
            parse_color(&theme.popup_border)
        };
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(right_border_color))
            .title(format!(" {} ", doc_title))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        if let Some(content) = active_content {
            let parsed_lines = parse_markdown_to_lines(content);
            let inner_width = (right_area.width.saturating_sub(4)) as usize;
            let wrapped_lines = wrap_lines(parsed_lines, inner_width);

            let paragraph = Paragraph::new(wrapped_lines.clone())
                .block(right_block)
                .scroll((*scroll_y as u16, 0))
                .style(Style::default().fg(parse_color(&theme.popup_fg)));
            f.render_widget(paragraph, right_area);

            // Render scrollbar if text is longer than panel height
            let total_lines = wrapped_lines.len();
            let inner_height = right_area.height.saturating_sub(2) as usize;
            if total_lines > inner_height {
                let mut scrollbar_state =
                    ScrollbarState::new(total_lines.saturating_sub(inner_height))
                        .position((*scroll_y).min(total_lines.saturating_sub(inner_height)));
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
                let scrollbar_area = Rect {
                    x: right_area.x + right_area.width.saturating_sub(1),
                    y: right_area.y + 1,
                    width: 1,
                    height: right_area.height.saturating_sub(2),
                };
                f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
            }
        } else {
            let empty_paragraph = Paragraph::new(" No document loaded ").block(right_block);
            f.render_widget(empty_paragraph, right_area);
        }

        // 3. Render help hint at the bottom
        let hint_area = Rect {
            x: area.x + 2,
            y: area.y + area.height - 2,
            width: area.width.saturating_sub(4),
            height: 1,
        };
        let hint_text = " [Tab] Switch Panels  [Up/Down/j/k] Navigate/Scroll  [Esc] Close ";
        f.render_widget(
            Paragraph::new(hint_text)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            hint_area,
        );
        true
    } else {
        false
    }
}

fn parse_markdown_to_lines(text: &str) -> Vec<ratatui::text::Line<'static>> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let parser = Parser::new(text);
    let mut lines = Vec::new();
    let mut current_spans = Vec::new();

    let mut bold = false;
    let mut italic = false;
    let code = false;
    let mut link = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }

                    let prefix = match level {
                        HeadingLevel::H1 => "# ",
                        HeadingLevel::H2 => "## ",
                        HeadingLevel::H3 => "### ",
                        _ => "#### ",
                    };
                    current_spans.push(Span::styled(
                        prefix,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Tag::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Link { .. } => link = true,
                Tag::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    current_spans.push(Span::styled("• ", Style::default().fg(Color::Cyan)));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    if !current_spans.is_empty() {
                        for span in &mut current_spans {
                            span.style = span.style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                        }
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Link => link = false,
                TagEnd::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                let mut style = Style::default();
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if code {
                    style = style.fg(Color::Magenta);
                } else if link {
                    style = style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED);
                } else {
                    style = style.fg(Color::White);
                }
                current_spans.push(Span::styled(t.into_string(), style));
            }
            Event::Code(c) => {
                current_spans.push(Span::styled(
                    format!(" `{}` ", c),
                    Style::default().fg(Color::Magenta),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
    }

    lines
}

fn wrap_lines(lines: Vec<ratatui::text::Line<'static>>, width: usize) -> Vec<ratatui::text::Line<'static>> {
    let mut wrapped = Vec::new();
    for line in lines {
        let total_chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if total_chars <= width {
            wrapped.push(line);
            continue;
        }

        let mut current_line_spans = Vec::new();
        let mut current_width = 0;

        for span in line.spans {
            let text = span.content;
            let style = span.style;

            let mut words = Vec::new();
            let mut word = String::new();
            for c in text.chars() {
                if c.is_whitespace() {
                    if !word.is_empty() {
                        words.push((word.clone(), false));
                        word.clear();
                    }
                    words.push((c.to_string(), true));
                } else {
                    word.push(c);
                }
            }
            if !word.is_empty() {
                words.push((word, false));
            }

            for (w, is_space) in words {
                let w_len = w.chars().count();
                if current_width + w_len > width && !is_space && current_width > 0 {
                    wrapped.push(ratatui::text::Line::from(current_line_spans));
                    current_line_spans = Vec::new();
                    current_width = 0;
                }

                if w_len > width {
                    let chars: Vec<char> = w.chars().collect();
                    for chunk in chars.chunks(width) {
                        let chunk_str: String = chunk.iter().collect();
                        wrapped.push(ratatui::text::Line::from(vec![ratatui::text::Span::styled(chunk_str, style)]));
                    }
                    continue;
                }

                current_line_spans.push(ratatui::text::Span::styled(w, style));
                current_width += w_len;
            }
        }
        if !current_line_spans.is_empty() {
            wrapped.push(ratatui::text::Line::from(current_line_spans));
        }
    }
    wrapped
}
