//! First-run keymap preset picker.

use crate::app::state::popup::OnboardingKeymap;
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};

/// The presets offered, in the order of the built-in list.
pub fn preset_ids() -> Vec<&'static str> {
    crate::keybindings::embedded::PRESETS
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

impl DialogView for OnboardingKeymap {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let OnboardingKeymap { cursor_idx } = self;

        let inner = crate::ui::popup::kit::dialog_frame(
            f,
            size,
            (66, 4 + 3 * preset_ids().len() as u16 + 2),
            t("onboarding_title"),
            crate::ui::popup::kit::fg(Color::Cyan),
            theme,
        );

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
            Paragraph::new(t("onboarding_intro")).style(Style::default().fg(theme.popup_fg)),
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
            crate::ui::popup::kit::mouse::list_item(chunks[i + 1], (i, *cursor_idx), &[]);
            let selected = i == *cursor_idx;
            let style = if selected {
                theme.selection().add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.popup_fg)
            };
            let mark = if selected { ">" } else { " " };
            // Title and description on rows of their own (a newline
            // inside a span is not drawn).
            f.render_widget(
                Paragraph::new(Text::from(vec![
                    Line::from(format!(" {mark} {title}")),
                    Line::from(format!("    {desc}")),
                ]))
                .style(style)
                .wrap(Wrap { trim: false }),
                chunks[i + 1],
            );
        }

        f.render_widget(
            Paragraph::new(t("onboarding_hint")).style(Style::default().fg(Color::DarkGray)),
            chunks[ids.len() + 1],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_preset_description_gets_its_own_row() {
        let h = crate::test_harness::Harness::builder()
            .size(100, 40)
            .settings(|s| s.onboarding_completed = false)
            .build();
        let (_, title_row) = h.locate(&t("onboarding_norton"));
        let desc: String = t("onboarding_norton_desc").chars().take(20).collect();
        let (_, desc_row) = h.locate(&desc);
        assert_eq!(desc_row, title_row + 1);
    }

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
