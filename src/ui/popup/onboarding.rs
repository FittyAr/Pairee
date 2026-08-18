//! First-run keymap preset picker.

use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub const PRESET_IDS: [&str; 3] = ["norton", "neovim", "vscode"];

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::OnboardingKeymap { cursor_idx } = popup else {
        return false;
    };

    let area = centered_rect_fixed(62, 16, size);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(t("onboarding_title"))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(t("onboarding_intro"))
            .style(Style::default().fg(parse_color(&theme.popup_fg))),
        chunks[0],
    );

    let rows = [
        (t("onboarding_norton"), t("onboarding_norton_desc")),
        (t("onboarding_neovim"), t("onboarding_neovim_desc")),
        (t("onboarding_vscode"), t("onboarding_vscode_desc")),
    ];
    for (i, (title, desc)) in rows.iter().enumerate() {
        let selected = i == *cursor_idx;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(parse_color(&theme.popup_fg))
        };
        let mark = if selected { ">" } else { " " };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(format!(" {mark} {title}\n")),
                Span::raw(format!("    {desc}")),
            ]))
            .style(style),
            chunks[i + 1],
        );
    }

    f.render_widget(
        Paragraph::new(t("onboarding_hint")).style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_ids_match_builtin_keymaps() {
        assert_eq!(PRESET_IDS, ["norton", "neovim", "vscode"]);
    }
}
