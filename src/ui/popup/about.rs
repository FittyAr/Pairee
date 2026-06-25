use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Rect, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

pub fn render_about_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::About { scroll_y } = popup {
        // Center the rectangle (width: 70 columns, height: 20 lines)
        let area = super::centered_rect_fixed(70, 20, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(parse_color(&theme.popup_border));
        let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
        let text_style = Style::default().fg(parse_color(&theme.popup_fg));
        let bold_style = text_style.add_modifier(Modifier::BOLD);
        let link_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED);
        let title_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(t("about_title"))
            .style(bg_style);

        // Build list of lines
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Pairee", title_style.fg(Color::LightCyan)),
            Span::styled(" - Terminal File Manager", title_style),
        ]));
        lines.push(Line::from(Span::styled("==============================", Style::default().fg(Color::DarkGray))));
        lines.push(Line::from(""));

        // Version
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_version")), bold_style),
            Span::styled(env!("CARGO_PKG_VERSION"), text_style),
        ]));

        // Website
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_website")), bold_style),
            Span::styled("pairee.fitty.ar", link_style),
        ]));

        // GitHub
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_github")), bold_style),
            Span::styled("https://github.com/FittyAr/Pairee", link_style),
        ]));

        // License
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_license")), bold_style),
            Span::styled("GNU General Public License v3.0", text_style),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(t("about_libraries"), bold_style)));
        lines.push(Line::from(Span::styled("---------------------------", Style::default().fg(Color::DarkGray))));

        // List of dependencies
        let libs = vec![
            ("ratatui", "MIT", "https://github.com/ratatui/ratatui"),
            ("crossterm", "MIT", "https://github.com/crossterm-rs/crossterm"),
            ("tokio", "MIT", "https://github.com/tokio-rs/tokio"),
            ("serde", "MIT/Apache-2.0", "https://github.com/serde-rs/serde"),
            ("serde_json", "MIT/Apache-2.0", "https://github.com/serde-rs/json"),
            ("toml", "MIT/Apache-2.0", "https://github.com/toml-rs/toml"),
            ("directories", "MIT/Apache-2.0", "https://github.com/dirs-dev/directories-rs"),
            ("anyhow", "MIT/Apache-2.0", "https://github.com/dtolnay/anyhow"),
            ("thiserror", "MIT/Apache-2.0", "https://github.com/dtolnay/thiserror"),
            ("log", "MIT/Apache-2.0", "https://github.com/rust-lang/log"),
            ("simplelog", "MIT/Apache-2.0", "https://github.com/dignifiedquire/simplelog"),
            ("chrono", "MIT/Apache-2.0", "https://github.com/chronotope/chrono"),
            ("zip", "MIT", "https://github.com/zip-rs/zip2"),
            ("tar", "MIT/Apache-2.0", "https://github.com/alexcrichton/tar-rs"),
            ("flate2", "MIT/Apache-2.0", "https://github.com/rust-lang/flate2-rs"),
            ("sevenz-rust", "Apache-2.0", "https://github.com/dyxushuai/sevenz-rust"),
            ("pulldown-cmark", "MIT", "https://github.com/raphlinus/pulldown-cmark"),
            ("image", "MIT/Apache-2.0", "https://github.com/image-rs/image"),
            ("git2", "MIT/Apache-2.0", "https://github.com/rust-lang/git2-rs"),
            ("futures-util", "MIT/Apache-2.0", "https://github.com/rust-lang/futures-rs"),
            ("reqwest", "MIT/Apache-2.0", "https://github.com/seanmonstar/reqwest"),
            ("windows-sys", "MIT/Apache-2.0", "https://github.com/microsoft/windows-rs"),
            ("ssh2", "MIT/Apache-2.0", "https://github.com/alexcrichton/ssh2-rs"),
            ("rustls", "Apache-2.0/MIT/ISC", "https://github.com/rustls/rustls"),
            ("tempfile", "MIT/Apache-2.0", "https://github.com/Stebalien/tempfile"),
        ];

        for (name, license, url) in libs {
            lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Cyan)),
                Span::styled(name, bold_style.fg(Color::LightGreen)),
                Span::styled(format!(" ({})", license), text_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", text_style),
                Span::styled(url, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
            ]));
        }

        let inner_width = area.width.saturating_sub(4) as usize;
        let wrapped_lines = wrap_lines(lines, inner_width);
        let total_lines = wrapped_lines.len();
        let inner_height = area.height.saturating_sub(4) as usize; // reserve space for borders and bottom hint

        // Clamp scroll_y to valid range
        let max_scroll = total_lines.saturating_sub(inner_height);
        let clamped_scroll = (*scroll_y).min(max_scroll);

        // Sub-layout: Content (top) and Hint (bottom)
        let popup_chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(area.height.saturating_sub(3)),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(block.inner(area));

        let paragraph = Paragraph::new(wrapped_lines)
            .scroll((clamped_scroll as u16, 0))
            .style(text_style);
        f.render_widget(paragraph, popup_chunks[0]);

        // Render scrollbar if needed
        if total_lines > inner_height {
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(clamped_scroll);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(3),
            };
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }

        // Render bottom hint
        let hint_text = t("about_hint");
        let hint_para = Paragraph::new(hint_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(hint_para, popup_chunks[1]);

        // Render block border and title
        f.render_widget(block, area);
        true
    } else {
        false
    }
}

// Simple word-wrapping helper
fn wrap_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
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
            let text = span.content.into_owned();
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
