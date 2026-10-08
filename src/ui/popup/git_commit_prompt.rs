use crate::app::state::PopupType;
use crate::app::state::popup::GitPromptPopup;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::kit;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::Line,
    widgets::Paragraph,
};

/// Renders the git commit message input prompt.
pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, size: Rect) -> bool {
    let PopupType::GitPrompt(GitPromptPopup::CommitPrompt(state)) = popup else {
        return false;
    };
    let title = if state.is_amend {
        format!(" {} [AMEND] ", t("git_commit_prompt_title"))
    } else {
        format!(" {} ", t("git_commit_prompt_title"))
    };
    let inner = kit::dialog_frame(
        f,
        size,
        (60, 7),
        kit::accent_title(title, Color::Green),
        kit::fg(Color::Green),
        theme,
    );
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1); 5]) // label, separator, input, empty, hint
        .split(inner);

    let fg = kit::popup_fg(theme);
    f.render_widget(
        Paragraph::new(t("git_commit_msg_label")).style(fg),
        chunks[0],
    );
    f.render_widget(
        kit::separator(inner.width, kit::fg(Color::DarkGray)),
        chunks[1],
    );
    let cursor = kit::selection(theme);
    f.render_widget(
        Paragraph::new(Line::from(kit::field_spans(&state.input, fg, cursor, true))),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(t("git_commit_hint")).style(kit::fg(Color::Yellow)),
        chunks[4],
    );
    true
}
