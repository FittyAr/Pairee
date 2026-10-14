use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

pub fn render_controls(
    f: &mut Frame,
    layout: &[Rect],
    cursor_idx: usize,
    install_progress: Option<&f32>,
    error: Option<&String>,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    let method = crate::update::detect::detect_install_method();
    let is_managed = method.is_managed();

    let first = if is_managed {
        "update_btn_copy"
    } else {
        "update_btn_now"
    };
    let buttons = [first, "update_btn_later", "update_btn_ignore"]
        .into_iter()
        .enumerate()
        .map(|(idx, key)| (t(key), idx))
        .collect::<Vec<_>>();

    let btn_constraints = vec![Constraint::Ratio(1, buttons.len() as u32); buttons.len()];
    let btn_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(btn_constraints)
        .split(layout[4]);

    for (i, (label, idx)) in buttons.iter().enumerate() {
        let is_selected = cursor_idx == *idx;
        let btn_style = if is_selected {
            Style::default()
                .fg(theme.ui.badge_fg)
                .bg(theme.ui.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.popup_fg).bg(theme.popup_bg)
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
        // Right / Left walk the buttons, Enter presses the focused one.
        let mut keys = crate::ui::popup::kit::mouse::presses(
            *idx as i64 - cursor_idx as i64,
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyCode::Left,
        );
        keys.push(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        crate::ui::popup::kit::mouse::keys(btn_cols[i], keys);
    }

    // Progress bar (during download/install)
    if let Some(progress) = install_progress {
        let gauge_area = layout[5];
        if gauge_area.height > 0 {
            let label =
                t("update_downloading").replacen("{}", &format!("{:.0}", progress * 100.0), 1);
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::NONE))
                .gauge_style(Style::default().fg(theme.ui.accent).bg(theme.ui.muted))
                .ratio((*progress as f64).clamp(0.0, 1.0))
                .label(label);
            f.render_widget(gauge, gauge_area);
        }
    }

    // Error message
    if let Some(err) = error {
        let err_area = layout[6];
        if err_area.height > 0 {
            let short_err = crate::ui::text_width::truncate_to_width(
                err,
                (err_area.width as usize).saturating_sub(3),
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" ⚠ {}", short_err),
                    Style::default().fg(theme.ui.danger),
                ))),
                err_area,
            );
        }
    }
}
