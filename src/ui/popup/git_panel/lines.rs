use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::text_width::pad_to_width;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn render_status_lines(
    entries: &[crate::git::status::GitFileStatus],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &ResolvedTheme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, entry)| {
            let is_cursor = i == cursor_idx;
            let label_color = theme.git_status(&entry.kind);
            let (bg, fg) = theme.row_colors(is_cursor);
            let stage_label = if entry.is_staged && entry.is_unstaged {
                "[staged+] "
            } else if entry.is_staged {
                "[staged]  "
            } else {
                "          "
            };
            let stage_color = if entry.is_staged && entry.is_unstaged {
                theme.ui.accent
            } else if entry.is_staged {
                theme.ui.success
            } else {
                theme.ui.muted
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.kind.label()),
                    Style::default()
                        .fg(label_color)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(stage_label, Style::default().fg(stage_color).bg(bg)),
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
    theme: &ResolvedTheme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, commit)| {
            let is_cursor = i == cursor_idx;
            let (bg, fg) = theme.row_colors(is_cursor);
            Line::from(vec![
                Span::styled(
                    format!(" {} ", commit.hash_short.clone()),
                    Style::default()
                        .fg(theme.ui.warning)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", commit.date.clone()),
                    Style::default().fg(theme.ui.accent).bg(bg),
                ),
                Span::styled(
                    format!("{} ", pad_to_width(&commit.author, 20)),
                    Style::default().fg(theme.ui.success).bg(bg),
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
    theme: &ResolvedTheme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, branch)| {
            let is_cursor = i == cursor_idx;
            let bg = if is_cursor {
                theme.selection_bg
            } else {
                theme.popup_bg
            };
            let name_color = if branch.is_remote {
                theme.ui.muted
            } else if branch.is_current {
                theme.ui.success
            } else {
                theme.popup_fg
            };
            let prefix = if branch.is_current { "* " } else { "  " };
            let type_label = if branch.is_remote {
                "[remote] "
            } else {
                "         "
            };
            let sync_label = if !branch.is_remote {
                if branch.ahead > 0 && branch.behind > 0 {
                    format!(" [↑{} ↓{}]", branch.ahead, branch.behind)
                } else if branch.ahead > 0 {
                    format!(" [↑{}]", branch.ahead)
                } else if branch.behind > 0 {
                    format!(" [↓{}]", branch.behind)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            Line::from(vec![
                Span::styled(
                    format!(" {}", prefix),
                    Style::default().fg(theme.ui.warning).bg(bg),
                ),
                Span::styled(type_label, Style::default().fg(theme.ui.muted).bg(bg)),
                Span::styled(branch.name.clone(), Style::default().fg(name_color).bg(bg)),
                Span::styled(sync_label, Style::default().fg(theme.ui.accent).bg(bg)),
            ])
        })
        .collect()
}

pub fn render_stash_lines(
    entries: &[crate::git::stash::StashInfo],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &ResolvedTheme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, stash)| {
            let is_cursor = i == cursor_idx;
            let (bg, fg) = theme.row_colors(is_cursor);
            let short_oid = if stash.oid.len() > 7 {
                &stash.oid[..7]
            } else {
                &stash.oid
            };
            Line::from(vec![
                Span::styled(
                    format!(" stash@{{{}}} ", stash.index),
                    Style::default()
                        .fg(theme.ui.warning)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", short_oid),
                    Style::default().fg(theme.ui.muted).bg(bg),
                ),
                Span::styled(stash.message.clone(), Style::default().fg(fg).bg(bg)),
            ])
        })
        .collect()
}

pub fn render_tag_lines(
    entries: &[crate::git::tags::TagInfo],
    cursor_idx: usize,
    scroll: usize,
    height: usize,
    theme: &ResolvedTheme,
) -> Vec<Line<'static>> {
    entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, tag)| {
            let is_cursor = i == cursor_idx;
            let (bg, fg) = theme.row_colors(is_cursor);
            let short_oid = if tag.target_oid.len() > 7 {
                &tag.target_oid[..7]
            } else {
                &tag.target_oid
            };
            let msg = tag.message.as_deref().unwrap_or("");
            Line::from(vec![
                Span::styled(
                    format!("  {} ", tag.name),
                    Style::default()
                        .fg(theme.ui.warning)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{}] ", short_oid),
                    Style::default().fg(theme.ui.muted).bg(bg),
                ),
                Span::styled(msg.to_string(), Style::default().fg(fg).bg(bg)),
            ])
        })
        .collect()
}
