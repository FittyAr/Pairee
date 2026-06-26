use super::centered_rect_fixed;
use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
};

/// Render the "Update Available" popup.
/// Returns true if the popup was handled (consumed).
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    let (info, cursor_idx, install_progress, error) = match popup {
        PopupType::UpdateAvailable {
            info,
            cursor_idx,
            install_progress,
            error,
        } => (info, cursor_idx, install_progress, error),
        _ => return false,
    };

    let width: u16 = 62.min(size.width.saturating_sub(4));
    let height: u16 = 20.min(size.height.saturating_sub(4));
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
    let max_lines = layout[2].height as usize;
    let notes_text: Vec<Line> = info
        .release_notes
        .lines()
        .take(max_lines)
        .map(|l| {
            // Strip simple markdown like "## " headings
            let clean = l.trim_start_matches('#').trim();
            Line::from(Span::styled(
                format!(" {}", clean),
                Style::default().fg(parse_color(&theme.popup_fg)),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(notes_text).style(bg_style), layout[2]);

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
