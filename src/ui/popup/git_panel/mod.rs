//! Main Git panel popup rendering.

mod lines;

use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    if let PopupType::GitPanel(crate::app::state::GitPanelState {
        active_tab,
        cursor_idx,
        scroll,
        status_entries,
        log_entries,
        branch_entries,
        stash_entries,
        current_branch,
        repo_path,
        ..
    }) = popup
    {
        let area = centered_rect(85, 88, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(Color::Cyan);
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let title = format!(" Git: {} [{}] ", repo_name, current_branch);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // ── Layout ──────────────────────────────────────────────────────────
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // tab header
                Constraint::Length(1), // separator
                Constraint::Min(3),    // content
                Constraint::Length(1), // hint bar
            ])
            .split(inner);

        let header_area = chunks[0];
        let sep_area = chunks[1];
        let content_area = chunks[2];
        let hint_area = chunks[3];

        // ── Tab headers ──────────────────────────────────────────────────────
        let tab_names = [
            crate::config::localization::t("git_tab_status"),
            crate::config::localization::t("git_tab_log"),
            crate::config::localization::t("git_tab_branches"),
            crate::config::localization::t("git_tab_stash"),
        ];
        let mut tab_spans: Vec<Span> = Vec::new();
        for (i, name) in tab_names.iter().enumerate() {
            let is_active = i == *active_tab;
            let style = if is_active {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            tab_spans.push(Span::styled(format!("  [ {} ]  ", name), style));
        }
        f.render_widget(Paragraph::new(Line::from(tab_spans)), header_area);

        f.render_widget(
            Paragraph::new("─".repeat(inner.width as usize))
                .style(Style::default().fg(Color::DarkGray)),
            sep_area,
        );

        // ── Content ─────────────────────────────────────────────────────────
        let list_height = content_area.height as usize;
        let effective_scroll = {
            if *cursor_idx < *scroll {
                *cursor_idx
            } else if *cursor_idx >= scroll + list_height {
                cursor_idx.saturating_sub(list_height - 1)
            } else {
                *scroll
            }
        };

        let lines: Vec<Line> = match active_tab {
            0 => lines::render_status_lines(
                status_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            1 => lines::render_log_lines(
                log_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            2 => lines::render_branch_lines(
                branch_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            3 => lines::render_stash_lines(
                stash_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            _ => Vec::new(),
        };

        if lines.is_empty() {
            let empty_msg = crate::config::localization::t("git_no_changes");
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("  {}", empty_msg),
                    Style::default().fg(Color::DarkGray),
                )),
                content_area,
            );
        } else {
            f.render_widget(Paragraph::new(lines), content_area);

            let total = match active_tab {
                0 => status_entries.len(),
                1 => log_entries.len(),
                2 => branch_entries.len(),
                3 => stash_entries.len(),
                _ => 0,
            };
            scrollbar::render_vertical_right(
                f,
                content_area,
                total,
                list_height,
                effective_scroll,
                theme,
                ScrollbarSurface::Popup,
                scrollbar,
                ScrollTargetId::GitList,
            );
        }

        // ── Hint bar ─────────────────────────────────────────────────────────
        let hint = match active_tab {
            0 => crate::config::localization::t("git_hint_status"),
            1 => crate::config::localization::t("git_hint_log"),
            2 => crate::config::localization::t("git_hint_branches"),
            _ => crate::config::localization::t("git_hint_stash"),
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::Yellow))),
            hint_area,
        );

        true
    } else {
        false
    }
}
