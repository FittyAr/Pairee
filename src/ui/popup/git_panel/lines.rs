use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn status_color(kind: &crate::git::status::StatusKind) -> Color {
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

pub fn render_status_lines(
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

pub fn render_log_lines(
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

pub fn render_branch_lines(
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
            let prefix = if branch.is_current { "* " } else { "  " };
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

pub fn render_stash_lines(
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
            let short_oid = if stash.oid.len() > 7 {
                &stash.oid[..7]
            } else {
                &stash.oid
            };
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
