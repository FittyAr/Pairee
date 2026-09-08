mod libs;

pub use libs::get_dependency_libraries;

use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::update::wrap_lines;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_about_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    if let PopupType::About { scroll_y } = popup {
        // Center the rectangle (width: 70 columns, height: 20 lines)
        let area = super::centered_rect_fixed(70, 20, size);
        f.render_widget(Clear, area);

        let border_style = Style::default().fg(parse_color(&theme.popup_border));
        let bg_style = Style::default().bg(parse_color(&theme.popup_bg));
        let text_style = Style::default().fg(parse_color(&theme.popup_fg));
        let bold_style = text_style.add_modifier(Modifier::BOLD);
        let link_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::UNDERLINED);
        let title_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(t("about_title"))
            .style(bg_style);

        // Build list of lines
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Pairee", title_style.fg(Color::LightCyan)),
            Span::styled(" - Terminal File Manager", title_style),
        ]));
        lines.push(Line::from(Span::styled(
            "==============================",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Version
        let git_suffix = env!("PAIREE_GIT_HASH");
        let version_str = if git_suffix == "no-git" {
            env!("CARGO_PKG_VERSION").to_string()
        } else {
            format!("{} ({})", env!("CARGO_PKG_VERSION"), git_suffix)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_version")), bold_style),
            Span::styled(version_str, text_style),
        ]));

        // Target
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_target")), bold_style),
            Span::styled(env!("PAIREE_TARGET"), text_style),
        ]));

        // Profile
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_profile")), bold_style),
            Span::styled(env!("PAIREE_BUILD_PROFILE"), text_style),
        ]));

        // Website
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_website")), bold_style),
            Span::styled("pairee.fitty.ar", link_style),
        ]));

        // GitHub
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_github")), bold_style),
            Span::styled("https://github.com/FittyAr/Pairee", link_style),
        ]));

        // License
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t("about_license")), bold_style),
            Span::styled("GNU General Public License v3.0", text_style),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(t("about_libraries"), bold_style)));
        lines.push(Line::from(Span::styled(
            "---------------------------",
            Style::default().fg(Color::DarkGray),
        )));

        // List of dependencies
        for (name, license, url) in get_dependency_libraries() {
            lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Cyan)),
                Span::styled(name, bold_style.fg(Color::LightGreen)),
                Span::styled(format!(" ({})", license), text_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", text_style),
                Span::styled(
                    url,
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]));
        }

        let inner_width = area.width.saturating_sub(4) as usize;
        let wrapped_lines = wrap_lines(lines, inner_width);
        let total_lines = wrapped_lines.len();
        let inner_height = area.height.saturating_sub(4) as usize; // reserve space for borders and bottom hint

        // Clamp scroll_y to valid range
        let max_scroll = total_lines.saturating_sub(inner_height);
        let clamped_scroll = (*scroll_y).min(max_scroll);

        // Sub-layout: Content (top) and Hint (bottom)
        let popup_chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(area.height.saturating_sub(3)),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(block.inner(area));

        let paragraph = Paragraph::new(wrapped_lines)
            .scroll((clamped_scroll as u16, 0))
            .style(text_style);
        f.render_widget(paragraph, popup_chunks[0]);

        // Fractional scrollbar on the content pane
        scrollbar::render_vertical(
            f,
            Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(3),
            },
            total_lines,
            inner_height,
            clamped_scroll,
            theme,
            ScrollbarSurface::Popup,
            scrollbar,
            ScrollTargetId::About,
        );

        // Render bottom hint
        let hint_text = t("about_hint");
        let hint_para = Paragraph::new(hint_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(hint_para, popup_chunks[1]);

        // Render block border and title
        f.render_widget(block, area);
        true
    } else {
        false
    }
}
