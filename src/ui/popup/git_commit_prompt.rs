use crate::app::state::popup::GitCommitPromptState;
use crate::config::localization::t;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::Paragraph,
};

/// Renders the git commit message input prompt.
impl DialogView for GitCommitPromptState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let state = self;
        let title = if state.is_amend {
            format!(" {} [AMEND] ", t("git_commit_prompt_title"))
        } else {
            format!(" {} ", t("git_commit_prompt_title"))
        };
        let inner = kit::dialog_frame(
            f,
            size,
            (60, 7),
            kit::accent_title(title, theme.ui.success),
            kit::fg(theme.ui.success),
            theme,
        );
        let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1); 5]) // label, separator, input, empty, hint
        .split(inner);

        let fg = theme.popup_fg();
        f.render_widget(
            Paragraph::new(t("git_commit_msg_label")).style(fg),
            chunks[0],
        );
        f.render_widget(
            kit::separator(inner.width, kit::fg(theme.ui.muted)),
            chunks[1],
        );
        let cursor = theme.selection();
        f.render_widget(
            Paragraph::new(Line::from(kit::field_spans(&state.input, fg, cursor, true))),
            chunks[2],
        );
        f.render_widget(
            Paragraph::new(t("git_commit_hint")).style(kit::fg(theme.ui.warning)),
            chunks[4],
        );
    }
}
