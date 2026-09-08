use crate::config::localization::t;
use ratatui::{Frame, layout::Rect, style::Style, widgets::Paragraph};

pub fn render_buttons(
    f: &mut Frame,
    area: Rect,
    cursor_idx: usize,
    btn_action_text: &str,
    act_style: Style,
    norm_style: Style,
) {
    let b1 = if cursor_idx == 10 {
        act_style
    } else {
        norm_style
    };
    let b2 = if cursor_idx == 11 {
        act_style
    } else {
        norm_style
    };
    let b3 = if cursor_idx == 12 {
        act_style
    } else {
        norm_style
    };
    let b4 = if cursor_idx == 13 {
        act_style
    } else {
        norm_style
    };

    let btns = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(btn_action_text, b1),
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled(t("btn_f10_tree"), b2),
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled(t("btn_filter"), b3),
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled(t("btn_cancel_bracket"), b4),
    ]);
    f.render_widget(
        Paragraph::new(btns).alignment(ratatui::layout::Alignment::Center),
        area,
    );
}
