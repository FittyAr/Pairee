use crate::app::state::popup::GitConfirmCheckoutState;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::popup::kit;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Renders the git checkout confirmation dialog.
impl DialogView for GitConfirmCheckoutState {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let state = self;
        let target = &state.target;
        let is_branch = &state.is_branch;
        let area = centered_rect_fixed(60, 8, size);

        let kind_str = if *is_branch {
            crate::config::localization::t("git_checkout_branch")
        } else {
            crate::config::localization::t("git_checkout_commit")
        };

        let title = format!(" {} ", crate::config::localization::t("git_checkout_title"));

        let inner = kit::frame_in(
            f,
            area,
            kit::accent_title(title, theme.ui.warning),
            kit::fg(theme.ui.warning),
            theme,
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let msg1 = format!(
            "  {} {}:",
            crate::config::localization::t("git_checkout_confirm"),
            kind_str
        );
        f.render_widget(
            Paragraph::new(msg1).style(Style::default().fg(theme.popup_fg)),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(format!("  {}", target)).style(
                Style::default()
                    .fg(theme.ui.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[1],
        );

        f.render_widget(
            crate::ui::popup::kit::separator(
                inner.width,
                crate::ui::popup::kit::fg(theme.ui.muted),
            ),
            chunks[2],
        );

        let buttons = Line::from(vec![
            Span::styled(
                "  [ Enter / Y ] ",
                Style::default()
                    .fg(theme.ui.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                crate::config::localization::t("git_checkout_yes"),
                Style::default().fg(theme.popup_fg),
            ),
            Span::styled("    ", Style::default()),
            Span::styled(
                "[ Esc / N ] ",
                Style::default()
                    .fg(theme.ui.danger)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                crate::config::localization::t("git_checkout_no"),
                Style::default().fg(theme.popup_fg),
            ),
        ]);
        f.render_widget(Paragraph::new(buttons), chunks[3]);
        {
            use crate::ui::popup::kit::mouse::{key, span_buttons};
            use crossterm::event::KeyCode;
            let labels = [
                format!(
                    "  [ Enter / Y ] {}",
                    crate::config::localization::t("git_checkout_yes")
                ),
                format!(
                    "[ Esc / N ] {}",
                    crate::config::localization::t("git_checkout_no")
                ),
            ];
            span_buttons(
                chunks[3],
                &labels,
                (4, false),
                vec![key(KeyCode::Enter), key(KeyCode::Esc)],
            );
        }
    }
}
