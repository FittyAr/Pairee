use crate::ui::resolved_theme::{ResolvedTheme, UiPalette};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn parse_markdown_to_lines(text: &str, theme: &ResolvedTheme) -> Vec<Line<'static>> {
    let mut renderer = MarkdownLines {
        ui: theme.ui,
        lines: Vec::new(),
        current_spans: Vec::new(),
        bold: false,
        italic: false,
        link: false,
    };
    for event in Parser::new(text) {
        renderer.event(event);
    }
    renderer.flush();
    renderer.lines
}

/// Builds styled lines from Markdown events.
struct MarkdownLines {
    ui: UiPalette,
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    bold: bool,
    italic: bool,
    link: bool,
}

impl MarkdownLines {
    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => {
                let style = self.text_style();
                self.current_spans
                    .push(Span::styled(t.into_string(), style));
            }
            Event::Code(c) => {
                self.current_spans.push(Span::styled(
                    format!(" `{}` ", c),
                    Style::default().fg(self.ui.special),
                ));
            }
            Event::SoftBreak | Event::HardBreak => self.flush(),
            _ => {}
        }
    }

    /// Ends the current line, if it has text.
    fn flush(&mut self) {
        if !self.current_spans.is_empty() {
            self.lines
                .push(Line::from(std::mem::take(&mut self.current_spans)));
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush();
                if !self.lines.is_empty() {
                    self.lines.push(Line::from(""));
                }
                let prefix = match level {
                    HeadingLevel::H1 => "# ",
                    HeadingLevel::H2 => "## ",
                    HeadingLevel::H3 => "### ",
                    _ => "#### ",
                };
                self.current_spans.push(Span::styled(
                    prefix,
                    Style::default()
                        .fg(self.ui.warning)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Tag::Paragraph => self.flush(),
            Tag::Emphasis => self.italic = true,
            Tag::Strong => self.bold = true,
            Tag::Link { .. } => self.link = true,
            Tag::Item => {
                self.flush();
                self.current_spans
                    .push(Span::styled("• ", Style::default().fg(self.ui.accent)));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                for span in &mut self.current_spans {
                    span.style = span.style.fg(self.ui.warning).add_modifier(Modifier::BOLD);
                }
                self.flush();
                self.lines.push(Line::from(""));
            }
            TagEnd::Paragraph => {
                self.flush();
                self.lines.push(Line::from(""));
            }
            TagEnd::Emphasis => self.italic = false,
            TagEnd::Strong => self.bold = false,
            TagEnd::Link => self.link = false,
            TagEnd::Item => self.flush(),
            _ => {}
        }
    }

    /// Bold/italic as open; links underlined in the link color, other text strong.
    fn text_style(&self) -> Style {
        let mut style = Style::default();
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.link {
            style.fg(self.ui.link).add_modifier(Modifier::UNDERLINED)
        } else {
            style.fg(self.ui.strong)
        }
    }
}
