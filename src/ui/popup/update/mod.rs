mod controls;

pub use crate::ui::wrap::wrap_lines;
pub use controls::render_controls;

use super::centered_rect_fixed;
use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::theme_apply::parse_color;
use crate::update::UpdateInfo;
use crate::update::detect::InstallMethod;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Render the "Update Available" popup.
/// Returns true if the popup was handled (consumed).
pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
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
    let size_str = download_size_suffix(info, &method);

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

    render_version_lines(f, &layout, info, &method, fg_style);

    render_notes(f, layout[2], info, theme, scroll_y, scrollbar);

    // Separator
    let sep = "─".repeat(inner.width as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(sep, Style::default().fg(muted)))),
        layout[3],
    );

    render_controls(
        f,
        &layout,
        *cursor_idx,
        install_progress.as_ref(),
        error.as_ref(),
        parse_color(&theme.popup_fg),
        parse_color(&theme.popup_bg),
    );

    // Hint line at the bottom
    render_hint(f, area, size, &method);

    true
}

/// ` (12.3 MB)` for the download this install method would fetch.
fn download_size_suffix(info: &UpdateInfo, method: &InstallMethod) -> String {
    if method.is_managed() {
        return String::new();
    }
    #[cfg(target_os = "windows")]
    let asset_name = if matches!(method, InstallMethod::InnoSetup) {
        crate::update::downloader::expected_installer_name(&info.version)
    } else {
        crate::update::downloader::expected_asset_name(&info.version)
    };
    #[cfg(not(target_os = "windows"))]
    let asset_name = crate::update::downloader::expected_asset_name(&info.version);

    info.assets
        .iter()
        .find(|a| a.name == asset_name)
        .map(|asset| format!(" ({:.1} MB)", asset.size as f64 / 1_048_576.0))
        .unwrap_or_default()
}

/// Current → new version with the install method, then the release URL.
fn render_version_lines(
    f: &mut Frame,
    layout: &[Rect],
    info: &UpdateInfo,
    method: &InstallMethod,
    fg_style: Style,
) {
    let accent = Color::Cyan;
    let muted = Style::default().fg(Color::DarkGray);
    let ver_line = Line::from(vec![
        Span::styled("Current: ", muted),
        Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), muted),
        Span::raw("  →  "),
        Span::styled(
            format!("v{}", info.version),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  ("),
        Span::styled(method.label(), muted),
        Span::raw(")"),
    ]);
    let url_line = Line::from(vec![
        Span::styled("Release info: ", muted),
        Span::styled(info.html_url.as_str(), Style::default().fg(accent)),
    ]);
    for (line, area) in [(ver_line, layout[0]), (url_line, layout[1])] {
        f.render_widget(
            Paragraph::new(line)
                .style(fg_style)
                .alignment(Alignment::Center),
            area,
        );
    }
}

/// Release notes (headings highlighted), wrapped and scrollable.
fn render_notes(
    f: &mut Frame,
    area: Rect,
    info: &UpdateInfo,
    theme: &Theme,
    scroll_y: usize,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let notes_lines: Vec<Line> = info
        .release_notes
        .lines()
        .map(|l| {
            let is_heading = l.trim_start().starts_with('#');
            let clean = l.trim_start_matches('#').trim();
            let style = if is_heading {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            Line::from(Span::styled(format!(" {}", clean), style))
        })
        .collect();

    let inner_width = (area.width as usize).saturating_sub(3);
    let wrapped_notes = wrap_lines(notes_lines, inner_width);
    let total_lines = wrapped_notes.len();
    let inner_height = area.height as usize;

    let max_scroll = total_lines.saturating_sub(inner_height);
    let clamped_scroll = scroll_y.min(max_scroll);

    let paragraph = Paragraph::new(wrapped_notes)
        .scroll((clamped_scroll as u16, 0))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));
    f.render_widget(paragraph, area);

    scrollbar::render_vertical_right(
        f,
        area,
        ScrollView {
            content_len: total_lines,
            viewport_len: inner_height,
            offset: clamped_scroll,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::UpdateNotes,
        },
    );
}

/// Below the popup: the package-manager command, or the key hint.
fn render_hint(f: &mut Frame, area: Rect, size: Rect, method: &InstallMethod) {
    let hint_y = area.y + area.height;
    if hint_y >= size.height {
        return;
    }
    let hint_area = Rect {
        x: area.x,
        y: hint_y,
        width: area.width,
        height: 1,
    };
    let line = match method.managed_upgrade_command() {
        Some(managed_cmd) => {
            let short_cmd: String = managed_cmd.chars().take(area.width as usize).collect();
            Line::from(vec![
                Span::styled(" $ ", Style::default().fg(Color::Green)),
                Span::styled(short_cmd, Style::default().fg(Color::Yellow)),
            ])
        }
        None => Line::from(Span::styled(
            " ←/→ select  Enter confirm  Esc close",
            Style::default().fg(Color::DarkGray),
        )),
    };
    f.render_widget(Paragraph::new(line), hint_area);
}
