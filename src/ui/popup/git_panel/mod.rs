//! Main Git panel popup rendering.
//!
//! Only reads the [`GitPanelState`] filled by the background loader; no
//! repository access happens while painting.

mod lines;

use crate::app::git_panel_load::GitPanelLoader;
use crate::app::state::{GitPanelState, PopupType};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Localization keys per tab: name, empty-list message, key hints.
const TABS: [(&str, &str, &str); 5] = [
    ("git_tab_status", "git_no_changes", "git_hint_status"),
    ("git_tab_log", "git_log_empty", "git_hint_log"),
    (
        "git_tab_branches",
        "git_branches_empty",
        "git_hint_branches",
    ),
    ("git_tab_stash", "git_stash_empty", "git_hint_stash"),
    ("git_tab_tags", "git_tags_empty", "git_hint_tags"),
];

fn tab_keys(tab: usize) -> (&'static str, &'static str, &'static str) {
    TABS[tab.min(TABS.len() - 1)]
}

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
    loader: &GitPanelLoader,
) -> bool {
    let PopupType::GitPanel(panel) = popup else {
        return false;
    };
    let loading = loader.is_loading(&panel.repo_path);
    let area = centered_rect(85, 88, size);
    f.render_widget(Clear, area);
    let block = frame_block(panel, loading, theme);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab header
            Constraint::Length(1), // separator
            Constraint::Min(3),    // content
            Constraint::Length(1), // hint bar
        ])
        .split(inner);

    render_tab_header(f, chunks[0], panel.active_tab, theme);
    f.render_widget(
        Paragraph::new("─".repeat(inner.width as usize))
            .style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
    render_content(f, chunks[2], panel, loading, theme, scrollbar);
    f.render_widget(
        Paragraph::new(Span::styled(
            t(tab_keys(panel.active_tab).2),
            Style::default().fg(Color::Yellow),
        )),
        chunks[3],
    );
    true
}

/// Border with "Git: <repo> [<branch>]" and a loading marker.
fn frame_block(panel: &GitPanelState, loading: bool, theme: &Theme) -> Block<'static> {
    let repo_name = panel
        .repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?");
    let mut title = format!(" Git: {} [{}] ", repo_name, panel.current_branch);
    if loading {
        title.push_str(&t("panel_loading"));
        title.push(' ');
    }
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.popup_bg)))
}

fn render_tab_header(f: &mut Frame, area: Rect, active_tab: usize, theme: &Theme) {
    let spans: Vec<Span> = TABS
        .iter()
        .enumerate()
        .map(|(i, (name, _, _))| {
            let style = if i == active_tab {
                Style::default()
                    .bg(parse_color(&theme.selection_bg))
                    .fg(parse_color(&theme.selection_fg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(parse_color(&theme.popup_fg))
            };
            Span::styled(format!("  [ {} ]  ", t(name)), style)
        })
        .collect();
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Rows of the active tab, its scrollbar, or the empty / loading notice.
fn render_content(
    f: &mut Frame,
    area: Rect,
    panel: &GitPanelState,
    loading: bool,
    theme: &Theme,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let height = area.height as usize;
    let (cursor, scroll) = (panel.cursor_idx, panel.scroll);
    let offset = if cursor < scroll {
        cursor
    } else if cursor >= scroll + height {
        cursor.saturating_sub(height - 1)
    } else {
        scroll
    };
    let rows: Vec<Line> = match panel.active_tab {
        0 => lines::render_status_lines(&panel.status_entries, cursor, offset, height, theme),
        1 => lines::render_log_lines(&panel.log_entries, cursor, offset, height, theme),
        2 => lines::render_branch_lines(&panel.branch_entries, cursor, offset, height, theme),
        3 => lines::render_stash_lines(&panel.stash_entries, cursor, offset, height, theme),
        4 => lines::render_tag_lines(&panel.tag_entries, cursor, offset, height, theme),
        _ => Vec::new(),
    };
    if rows.is_empty() {
        let notice = if loading {
            t("panel_loading")
        } else {
            t(tab_keys(panel.active_tab).1)
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  {}", notice),
                Style::default().fg(Color::DarkGray),
            )),
            area,
        );
        return;
    }
    f.render_widget(Paragraph::new(rows), area);
    scrollbar::render_vertical_right(
        f,
        area,
        ScrollView {
            content_len: panel.tab_len(panel.active_tab),
            viewport_len: height,
            offset,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::GitList,
        },
    );
}
