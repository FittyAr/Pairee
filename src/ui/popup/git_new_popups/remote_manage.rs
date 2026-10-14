use crate::app::state::popup::GitRemoteManageState;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit;
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::text_width::pad_to_width;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_remote_manage(
    f: &mut Frame,
    state: &GitRemoteManageState,
    theme: &ResolvedTheme,
    size: Rect,
) {
    let inner = kit::frame_in(
        f,
        centered_rect_in(70, 60, size),
        kit::accent_title(t("git_remote_manage_title"), Color::Cyan),
        kit::fg(Color::Cyan),
        theme,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Remote list
            Constraint::Length(1), // Hint bar
        ])
        .split(inner);

    if state.remotes.is_empty() {
        f.render_widget(kit::hint(t("git_remote_no_remotes")), chunks[0]);
    } else {
        let start = crate::ui::scrollbar::centered_scroll(
            state.selected_idx,
            state.remotes.len(),
            chunks[0].height as usize,
        );
        let lines: Vec<Line> = state
            .remotes
            .iter()
            .enumerate()
            .skip(start)
            .map(|(i, remote)| {
                let is_selected = i == state.selected_idx;
                let (bg, fg) = theme.row_colors(is_selected);
                let url = remote.url.as_deref().unwrap_or("-");
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", pad_to_width(&remote.name, 16)),
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" {} ", url), Style::default().fg(fg).bg(bg)),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), chunks[0]);
        crate::ui::popup::kit::mouse::list_rows(
            chunks[0],
            start,
            state.remotes.len(),
            state.selected_idx,
        );
    }

    f.render_widget(kit::hint(t("git_remote_manage_hint")), chunks[1]);
}
