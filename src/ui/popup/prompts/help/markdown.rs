use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn parse_markdown_to_lines(text: &str) -> Vec<Line<'static>> {
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
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
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
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                }
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Link { .. } => link = true,
                Tag::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
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
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Link => link = false,
                TagEnd::Item if !current_spans.is_empty() => {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
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
            Event::SoftBreak | Event::HardBreak if !current_spans.is_empty() => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(std::mem::take(&mut current_spans)));
    }

    lines
}

pub fn wrap_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
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
                    wrapped.push(Line::from(current_line_spans));
                    current_line_spans = Vec::new();
                    current_width = 0;
                }

                if w_len > width {
                    let chars: Vec<char> = w.chars().collect();
                    for chunk in chars.chunks(width) {
                        let chunk_str: String = chunk.iter().collect();
                        wrapped.push(Line::from(vec![Span::styled(chunk_str, style)]));
                    }
                    continue;
                }

                current_line_spans.push(Span::styled(w, style));
                current_width += w_len;
            }
        }
        if !current_line_spans.is_empty() {
            wrapped.push(Line::from(current_line_spans));
        }
    }
    wrapped
}
