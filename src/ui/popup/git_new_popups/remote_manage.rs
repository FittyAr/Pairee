use crate::app::state::popup::GitRemoteManageState;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit;
use crate::ui::theme_apply::parse_color;
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
    theme: &Theme,
    size: Rect,
) -> bool {
    let inner = kit::frame_in(
        f,
        centered_rect(70, 60, size),
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
        let lines: Vec<Line> = state
            .remotes
            .iter()
            .enumerate()
            .map(|(i, remote)| {
                let is_selected = i == state.selected_idx;
                let bg = if is_selected {
                    parse_color(&theme.selection_bg)
                } else {
                    parse_color(&theme.popup_bg)
                };
                let fg = if is_selected {
                    parse_color(&theme.selection_fg)
                } else {
                    parse_color(&theme.popup_fg)
                };
                let url = remote.url.as_deref().unwrap_or("-");
                Line::from(vec![
                    Span::styled(
                        format!(" {:<16} ", remote.name),
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
    }

    f.render_widget(kit::hint(t("git_remote_manage_hint")), chunks[1]);

    true
}
