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

/// Renders a generic confirm action popup.
pub fn render_confirm_action(
    f: &mut Frame,
    state: &crate::app::state::popup::GitConfirmActionState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let message = &state.message;
    // Centered small confirmation box
    let area = centered_rect(50, 20, size);
    let title = crate::config::localization::t("git_confirm_action_title");
    let inner = kit::frame_in(
        f,
        area,
        kit::accent_title(title, Color::Cyan),
        kit::fg(Color::Cyan),
        theme,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),    // Message
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons: Yes / No
        ])
        .split(inner);

    // Render message
    let msg_para = Paragraph::new(message.as_str())
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(parse_color(&theme.popup_fg)));
    f.render_widget(msg_para, chunks[0]);

    // Yes/No Buttons. By default YES is focused (we handle Enter / Esc).
    let yes_style = Style::default()
        .bg(parse_color(&theme.selection_bg))
        .fg(parse_color(&theme.selection_fg))
        .add_modifier(Modifier::BOLD);
    let no_style = Style::default().fg(parse_color(&theme.popup_fg));

    let yes_text = format!(
        " [ {} (Enter) ] ",
        crate::config::localization::t("git_btn_yes")
    );
    let no_text = format!(
        " [ {} (Esc) ] ",
        crate::config::localization::t("git_btn_no")
    );
    let buttons_line = Line::from(vec![
        Span::styled(yes_text, yes_style),
        Span::raw("    "),
        Span::styled(no_text, no_style),
    ]);
    let buttons_para = Paragraph::new(buttons_line).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(buttons_para, chunks[2]);

    true
}
