use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

pub fn render_controls(
    f: &mut Frame,
    layout: &[Rect],
    cursor_idx: usize,
    install_progress: Option<&f32>,
    error: Option<&String>,
    theme_popup_fg: Color,
    theme_popup_bg: Color,
) {
    let method = crate::update::detect::detect_install_method();
    let is_managed = method.is_managed();

    let buttons = if is_managed {
        vec![
            ("Copy command", 0),
            ("Remind later", 1),
            ("Ignore version", 2),
        ]
    } else {
        vec![
            ("Update now", 0),
            ("Remind later", 1),
            ("Ignore version", 2),
        ]
    };

    let btn_constraints = vec![Constraint::Ratio(1, buttons.len() as u32); buttons.len()];
    let btn_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(btn_constraints)
        .split(layout[4]);

    for (i, (label, idx)) in buttons.iter().enumerate() {
        let is_selected = cursor_idx == *idx;
        let btn_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme_popup_fg).bg(theme_popup_bg)
        };
        let btn_text = if is_selected {
            format!("[ {} ]", label)
        } else {
            format!("  {}  ", label)
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(btn_text, btn_style)))
                .alignment(ratatui::layout::Alignment::Center),
            btn_cols[i],
        );
    }

    // Progress bar (during download/install)
    if let Some(progress) = install_progress {
        let gauge_area = layout[5];
        if gauge_area.height > 0 {
            let label = format!("Downloading... {:.0}%", progress * 100.0);
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::NONE))
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
                .ratio((*progress as f64).clamp(0.0, 1.0))
                .label(label);
            f.render_widget(gauge, gauge_area);
        }
    }

    // Error message
    if let Some(err) = error {
        let err_area = layout[6];
        if err_area.height > 0 {
            let short_err: String = err
                .chars()
                .take((err_area.width as usize).saturating_sub(2))
                .collect();
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" ⚠ {}", short_err),
                    Style::default().fg(Color::Red),
                ))),
                err_area,
            );
        }
    }
}
