use crate::app::state::popup::PluginDialog;
use crate::config::localization::t;
use crate::plugin::manager::DialogPosition;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

impl DialogView for PluginDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        match self {
            PluginDialog::Confirm {
                title,
                msg,
                cursor_idx,
                position,
            } => render_confirm(f, theme, size, title, msg, *cursor_idx, position.as_ref()),
            PluginDialog::Input {
                title,
                input,
                obscure,
                position,
            } => render_input(f, theme, size, title, input, *obscure, position.as_ref()),
            PluginDialog::Which {
                candidates,
                silent,
                position,
            } => render_which(f, theme, size, candidates, *silent, position.as_ref()),
        }
    }
}

fn dialog_area(
    size: Rect,
    position: Option<&DialogPosition>,
    default_w: u16,
    default_h: u16,
) -> Rect {
    let (w, h) = match position {
        Some(p) => (
            if p.w > 0 { p.w } else { default_w },
            if p.h > 0 { p.h } else { default_h },
        ),
        None => (default_w, default_h),
    };
    let mut area = crate::ui::popup::centered_area(w.min(size.width), h.min(size.height), size);
    if let Some(p) = position {
        let origin = p.origin.to_ascii_lowercase();
        if origin.contains("top") {
            area.y = 1;
        } else if origin.contains("bottom") {
            area.y = size.height.saturating_sub(area.height).saturating_sub(1);
        }
        area.x = offset_u16(area.x, p.x, size.width.saturating_sub(area.width));
        area.y = offset_u16(area.y, p.y, size.height.saturating_sub(area.height));
    }
    crate::ui::popup::place_popup(area, size)
}

fn offset_u16(base: u16, delta: i32, max: u16) -> u16 {
    let next = i32::from(base).saturating_add(delta);
    next.clamp(0, i32::from(max)) as u16
}

fn render_confirm(
    f: &mut Frame,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    title: &str,
    msg: &str,
    cursor_idx: usize,
    position: Option<&DialogPosition>,
) {
    let area = dialog_area(size, position, 50, 8);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" {title} "))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(msg.to_string())
            .style(Style::default().fg(theme.popup_fg))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );

    let yes_style = if cursor_idx == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let no_style = if cursor_idx == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };
    let labels = [
        format!(" [{}] ", t("plugin_dialog_yes")),
        format!(" [{}] ", t("plugin_dialog_no")),
    ];
    let buttons = Line::from(vec![
        Span::styled(labels[0].clone(), yes_style),
        Span::raw("  "),
        Span::styled(labels[1].clone(), no_style),
    ]);
    f.render_widget(Paragraph::new(buttons), chunks[1]);
    {
        use crate::ui::popup::kit::mouse::{key, span_buttons};
        use crossterm::event::KeyCode;
        let keys = vec![key(KeyCode::Char('y')), key(KeyCode::Char('n'))];
        span_buttons(chunks[1], &labels, (2, false), keys);
    }
    f.render_widget(
        Paragraph::new(t("plugin_dialog_confirm_hint")).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

fn render_input(
    f: &mut Frame,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    title: &str,
    input: &crate::app::text_input::TextField,
    obscure: bool,
    position: Option<&DialogPosition>,
) {
    let area = dialog_area(size, position, 56, 7);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border))
        .title(format!(" {title} "))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(inner);

    let style = Style::default().fg(theme.popup_fg);
    let mut spans = vec![ratatui::text::Span::styled(" > ", style)];
    if obscure {
        let masked = "*".repeat(input.text().chars().count());
        spans.push(ratatui::text::Span::styled(format!("{masked}_"), style));
    } else {
        spans.extend(crate::ui::popup::kit::field_spans(
            input,
            style,
            theme.selection(),
            true,
        ));
    }
    f.render_widget(Paragraph::new(ratatui::text::Line::from(spans)), chunks[0]);
    f.render_widget(
        Paragraph::new(t("plugin_dialog_input_hint")).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

fn render_which(
    f: &mut Frame,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    candidates: &[crate::plugin::manager::WhichCandidate],
    silent: bool,
    position: Option<&DialogPosition>,
) {
    if silent {
        let area = dialog_area(size, position, 40, 3);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(theme.popup_bg));
        f.render_widget(
            Paragraph::new(t("plugin_dialog_which_silent"))
                .block(block)
                .style(Style::default().fg(theme.popup_fg)),
            area,
        );
        return;
    }

    let height = (candidates.len() as u16).saturating_add(4).min(16);
    let area = dialog_area(size, position, 50, height);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(t("plugin_dialog_which_title"))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = candidates
        .iter()
        .map(|c| {
            let keys = c.on.join(", ");
            let desc = c.desc.as_deref().unwrap_or("");
            Line::from(vec![
                Span::styled(
                    format!("  {keys:<12} "),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.to_string(), Style::default().fg(theme.popup_fg)),
            ])
        })
        .collect();
    // A click on a one-character candidate presses its key.
    for (c, y) in candidates.iter().zip(inner.y..inner.bottom()) {
        let mut chars = c.on.first().map(|k| k.chars()).into_iter().flatten();
        if let (Some(ch), None) = (chars.next(), chars.next()) {
            crate::ui::popup::kit::mouse::keys(
                Rect::new(inner.x, y, inner.width, 1),
                crate::ui::popup::kit::mouse::key(crossterm::event::KeyCode::Char(ch)),
            );
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t("plugin_dialog_which_hint"),
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_area_honours_explicit_size() {
        let screen = Rect::new(0, 0, 100, 40);
        let pos = DialogPosition {
            origin: "center".into(),
            x: 0,
            y: 0,
            w: 30,
            h: 6,
        };
        let area = dialog_area(screen, Some(&pos), 50, 8);
        assert_eq!(area.width, 30);
        assert_eq!(area.height, 6);
    }

    #[test]
    fn dialog_area_top_origin_pins_near_top() {
        let screen = Rect::new(0, 0, 80, 24);
        let pos = DialogPosition {
            origin: "top-center".into(),
            x: 0,
            y: 0,
            w: 20,
            h: 4,
        };
        let area = dialog_area(screen, Some(&pos), 50, 8);
        assert_eq!(area.y, 1);
        assert_eq!(area.width, 20);
    }
}
