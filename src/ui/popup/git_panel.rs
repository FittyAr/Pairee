use crate::app::state::PopupType;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Renders the main Git panel popup with Status / Log / Branches tabs.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    if let PopupType::GitPanel {
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
    } = popup
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
            // Keep cursor in view
            if *cursor_idx < *scroll {
                *cursor_idx
            } else if *cursor_idx >= scroll + list_height {
                cursor_idx.saturating_sub(list_height - 1)
            } else {
                *scroll
            }
        };

        let lines: Vec<Line> = match active_tab {
            0 => render_status_lines(
                status_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            1 => render_log_lines(
                log_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            2 => render_branch_lines(
                branch_entries,
                *cursor_idx,
                effective_scroll,
                list_height,
                theme,
            ),
            3 => render_stash_lines(
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

fn status_color(kind: &crate::git::status::StatusKind) -> Color {
    use crate::git::status::StatusKind;
    match kind {
        StatusKind::Modified => Color::Yellow,
        StatusKind::Added => Color::Green,
        StatusKind::Deleted => Color::Red,
        StatusKind::Untracked => Color::DarkGray,
        StatusKind::Renamed => Color::Cyan,
        StatusKind::Conflicted => Color::Magenta,
    }
}

fn render_status_lines(
    entries: &[crate::git::status::GitFileStatus],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, entry)| {
            let is_cursor = i == cursor_idx;
            let label_color = status_color(&entry.kind);
            let bg = if is_cursor {
                parse_color(&theme.selection_bg)
            } else {
                parse_color(&theme.popup_bg)
            };
            let fg = if is_cursor {
                parse_color(&theme.selection_fg)
            } else {
                parse_color(&theme.popup_fg)
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.kind.label()),
                    Style::default()
                        .fg(label_color)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", entry.path.clone()),
                    Style::default().fg(fg).bg(bg),
                ),
            ])
        })
        .collect()
}

fn render_log_lines(
    entries: &[crate::git::log::CommitInfo],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, commit)| {
            let is_cursor = i == cursor_idx;
            let bg = if is_cursor {
                parse_color(&theme.selection_bg)
            } else {
                parse_color(&theme.popup_bg)
            };
            let fg = if is_cursor {
                parse_color(&theme.selection_fg)
            } else {
                parse_color(&theme.popup_fg)
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", commit.hash_short.clone()),
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", commit.date.clone()),
                    Style::default().fg(Color::Cyan).bg(bg),
                ),
                Span::styled(
                    format!("{:<20} ", commit.author.clone()),
                    Style::default().fg(Color::Green).bg(bg),
                ),
                Span::styled(commit.message.clone(), Style::default().fg(fg).bg(bg)),
            ])
        })
        .collect()
}

fn render_branch_lines(
    entries: &[crate::git::branches::BranchInfo],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, branch)| {
            let is_cursor = i == cursor_idx;
            let bg = if is_cursor {
                parse_color(&theme.selection_bg)
            } else {
                parse_color(&theme.popup_bg)
            };
            let name_color = if branch.is_remote {
                Color::DarkGray
            } else if branch.is_current {
                Color::Green
            } else {
                parse_color(&theme.popup_fg)
            };
            let prefix = if branch.is_current {
                "* "
            } else if branch.is_remote {
                "  "
            } else {
                "  "
            };
            let type_label = if branch.is_remote {
                "[remote] "
            } else {
                "         "
            };
            Line::from(vec![
                Span::styled(
                    format!(" {}", prefix),
                    Style::default().fg(Color::Yellow).bg(bg),
                ),
                Span::styled(type_label, Style::default().fg(Color::DarkGray).bg(bg)),
                Span::styled(branch.name.clone(), Style::default().fg(name_color).bg(bg)),
            ])
        })
        .collect()
}

fn render_stash_lines(
    entries: &[crate::git::stash::StashInfo],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, stash)| {
            let is_cursor = i == cursor_idx;
            let bg = if is_cursor {
                parse_color(&theme.selection_bg)
            } else {
                parse_color(&theme.popup_bg)
            };
            let fg = if is_cursor {
                parse_color(&theme.selection_fg)
            } else {
                parse_color(&theme.popup_fg)
            };
            let short_oid = if stash.oid.len() > 7 { &stash.oid[..7] } else { &stash.oid };
            Line::from(vec![
                Span::styled(
                    format!(" stash@{{{}}} ", stash.index),
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", short_oid),
                    Style::default().fg(Color::DarkGray).bg(bg),
                ),
                Span::styled(stash.message.clone(), Style::default().fg(fg).bg(bg)),
            ])
        })
        .collect()
}

