use super::centered_rect_fixed;
use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

/// Render the "Update Available" popup.
/// Returns true if the popup was handled (consumed).
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    let (info, cursor_idx, install_progress, error, scroll_y) = match popup {
        PopupType::UpdateAvailable {
            info,
            cursor_idx,
            install_progress,
            error,
            scroll_y,
        } => (info, cursor_idx, install_progress, error, *scroll_y),
        _ => return false,
    };

    let width: u16 = 80.min(size.width.saturating_sub(4));
    let height: u16 = 24.min(size.height.saturating_sub(4));
    let area = centered_rect_fixed(width, height, size);

    f.render_widget(Clear, area);

    let border_style = Style::default().fg(parse_color(&theme.popup_border));
    let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
    let fg_style = Style::default()
        .fg(parse_color(&theme.popup_fg))
        .bg(parse_color(&theme.popup_bg));

    let method = crate::update::detect::detect_install_method();
    let size_str = if method.is_managed() {
        "".to_string()
    } else {
        #[cfg(target_os = "windows")]
        let asset_name = if matches!(method, crate::update::detect::InstallMethod::InnoSetup) {
            crate::update::downloader::expected_installer_name(&info.version)
        } else {
            crate::update::downloader::expected_asset_name(&info.version)
        };
        #[cfg(not(target_os = "windows"))]
        let asset_name = crate::update::downloader::expected_asset_name(&info.version);

        if let Some(asset) = info.assets.iter().find(|a| a.name == asset_name) {
            format!(" ({:.1} MB)", asset.size as f64 / 1_048_576.0)
        } else {
            "".to_string()
        }
    };

    let title = format!(" 🎉 New version available: v{}{} ", info.version, size_str);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .style(bg_style);

    f.render_widget(block, area);

    // Inner area
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let accent = Color::Cyan;
    let muted = Color::DarkGray;

    // Layout: version line + url line + notes + separator + buttons + optional progress
    let progress_height: u16 = if install_progress.is_some() { 3 } else { 0 };
    let error_height: u16 = if error.is_some() { 2 } else { 0 };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // current version line
            Constraint::Length(1), // release URL line
            Constraint::Min(4),    // release notes
            Constraint::Length(1), // separator
            Constraint::Length(3), // buttons
            Constraint::Length(progress_height),
            Constraint::Length(error_height),
        ])
        .split(inner);

    // Current version info
    let current = env!("CARGO_PKG_VERSION");
    let ver_line = Line::from(vec![
        Span::styled("Current: ", Style::default().fg(muted)),
        Span::styled(format!("v{}", current), Style::default().fg(muted)),
        Span::raw("  →  "),
        Span::styled(
            format!("v{}", info.version),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  ("),
        Span::styled(method.label(), Style::default().fg(muted)),
        Span::raw(")"),
    ]);
    f.render_widget(
        Paragraph::new(ver_line)
            .style(fg_style)
            .alignment(Alignment::Center),
        layout[0],
    );

    // Release URL info
    let url_line = Line::from(vec![
        Span::styled("Release info: ", Style::default().fg(muted)),
        Span::styled(&info.html_url, Style::default().fg(accent)),
    ]);
    f.render_widget(
        Paragraph::new(url_line)
            .style(fg_style)
            .alignment(Alignment::Center),
        layout[1],
    );

    // Release notes
    let mut notes_lines = Vec::new();
    for l in info.release_notes.lines() {
        let is_heading = l.trim_start().starts_with('#');
        let clean = l.trim_start_matches('#').trim();
        let style = if is_heading {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        notes_lines.push(Line::from(Span::styled(format!(" {}", clean), style)));
    }

    let inner_width = (layout[2].width as usize).saturating_sub(3);
    let wrapped_notes = wrap_lines(notes_lines, inner_width);
    let total_lines = wrapped_notes.len();
    let inner_height = layout[2].height as usize;

    let max_scroll = total_lines.saturating_sub(inner_height);
    let clamped_scroll = scroll_y.min(max_scroll);

    let paragraph = Paragraph::new(wrapped_notes)
        .scroll((clamped_scroll as u16, 0))
        .style(bg_style);
    f.render_widget(paragraph, layout[2]);

    // Render scrollbar if needed
    if total_lines > inner_height {
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(clamped_scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let scrollbar_area = Rect {
            x: layout[2].x + layout[2].width.saturating_sub(1),
            y: layout[2].y,
            width: 1,
            height: layout[2].height,
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // Separator
    let sep = "─".repeat(inner.width as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(sep, Style::default().fg(muted)))),
        layout[3],
    );

    // Detect if this is a managed install method to show correct buttons
    let method = crate::update::detect::detect_install_method();
    let is_managed = method.is_managed();

    // Buttons
    let buttons = if is_managed {
        vec![
            ("Copy command", 0),
            ("Remind later", 1),
            ("Ignore version", 2),
        ]
    } else {
        vec![
            ("Update now", 0),
            ("Remind later", 1),
            ("Ignore version", 2),
        ]
    };

    let btn_constraints = vec![Constraint::Ratio(1, buttons.len() as u32); buttons.len()];
    let btn_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(btn_constraints)
        .split(layout[4]);

    for (i, (label, idx)) in buttons.iter().enumerate() {
        let is_selected = cursor_idx == idx;
        let btn_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(parse_color(&theme.popup_fg))
                .bg(parse_color(&theme.popup_bg))
        };
        let btn_text = if is_selected {
            format!("[ {} ]", label)
        } else {
            format!("  {}  ", label)
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(btn_text, btn_style)))
                .alignment(Alignment::Center),
            btn_cols[i],
        );
    }

    // Progress bar (during download/install)
    if let Some(progress) = install_progress {
        let gauge_area = layout[5];
        if gauge_area.height > 0 {
            let label = format!("Downloading... {:.0}%", progress * 100.0);
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::NONE))
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
                .ratio((*progress as f64).clamp(0.0, 1.0))
                .label(label);
            f.render_widget(gauge, gauge_area);
        }
    }

    // Error message
    if let Some(err) = error {
        let err_area = layout[6];
        if err_area.height > 0 {
            let short_err: String = err
                .chars()
                .take((err_area.width as usize).saturating_sub(2))
                .collect();
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" ⚠ {}", short_err),
                    Style::default().fg(Color::Red),
                ))),
                err_area,
            );
        }
    }

    // Hint line at the bottom
    if let Some(managed_cmd) = method.managed_upgrade_command() {
        // Show the command in a hint below the popup
        let hint_y = area.y + area.height;
        if hint_y < size.height {
            let hint_area = Rect {
                x: area.x,
                y: hint_y,
                width: area.width,
                height: 1,
            };
            let short_cmd: String = managed_cmd.chars().take(area.width as usize).collect();
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(" $ ", Style::default().fg(Color::Green)),
                    Span::styled(short_cmd, Style::default().fg(Color::Yellow)),
                ])),
                hint_area,
            );
        }
    } else {
        // Hint for keyboard nav
        let hint_y = area.y + area.height;
        if hint_y < size.height {
            let hint_area = Rect {
                x: area.x,
                y: hint_y,
                width: area.width,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " ←/→ select  Enter confirm  Esc close",
                    Style::default().fg(muted),
                ))),
                hint_area,
            );
        }
    }

    true
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
