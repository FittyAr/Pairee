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

/// The presets offered, in the order of the built-in list.
pub fn preset_ids() -> Vec<&'static str> {
    crate::keybindings::embedded::PRESETS
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::OnboardingKeymap { cursor_idx } = popup else {
        return false;
    };

    let area = centered_rect_fixed(66, 4 + 3 * preset_ids().len() as u16 + 2, size);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(t("onboarding_title"))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let ids = preset_ids();
    let constraints: Vec<Constraint> = std::iter::once(Constraint::Length(2))
        .chain(ids.iter().map(|_| Constraint::Length(3)))
        .chain(std::iter::once(Constraint::Min(1)))
        .collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    f.render_widget(
        Paragraph::new(t("onboarding_intro"))
            .style(Style::default().fg(parse_color(&theme.popup_fg))),
        chunks[0],
    );

    let rows: Vec<(String, String)> = ids
        .iter()
        .map(|id| {
            (
                t(&format!("onboarding_{id}")),
                t(&format!("onboarding_{id}_desc")),
            )
        })
        .collect();
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
        chunks[ids.len() + 1],
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_title_and_description() {
        use crate::config::localization::translator::get_default_english_translation as en;
        assert_eq!(preset_ids(), ["norton", "standard", "neovim", "yazi"]);
        for id in preset_ids() {
            for key in [format!("onboarding_{id}"), format!("onboarding_{id}_desc")] {
                assert_ne!(en(&key), key);
            }
        }
    }
}
