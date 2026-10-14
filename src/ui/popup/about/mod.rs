mod libs;

pub use libs::DEPENDENCY_LIBRARIES;

use crate::app::state::popup::About;
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::wrap::wrap_lines;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Styles of the About text.
struct AboutStyles {
    text: Style,
    bold: Style,
    link: Style,
    title: Style,
}

impl AboutStyles {
    fn new(theme: &crate::ui::resolved_theme::ResolvedTheme) -> Self {
        let text = Style::default().fg(theme.popup_fg);
        Self {
            text,
            bold: text.add_modifier(Modifier::BOLD),
            link: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED),
            title: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl DialogView for About {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render_about(f, self.scroll_y, cx.theme, cx.area, cx.scrollbar());
    }
}

fn render_about(
    f: &mut Frame,
    scroll_y: usize,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) {
    // Center the rectangle (width: 70 columns, height: 20 lines)
    let area = super::centered_rect_fixed(70, 20, size);
    f.render_widget(Clear, area);

    let styles = AboutStyles::new(theme);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border))
        .title(t("about_title"))
        .style(Style::default().bg(theme.popup_bg));

    let inner_width = area.width.saturating_sub(4) as usize;
    let wrapped_lines = wrap_lines(about_lines(&styles), inner_width);

    // Sub-layout: Content (top) and Hint (bottom)
    let popup_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(area.height.saturating_sub(3)),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(block.inner(area));

    scrollbar::render_text_pane(
        f,
        popup_chunks[0],
        (wrapped_lines, styles.text),
        scroll_y,
        theme,
        scrollbar::ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::About,
        },
    );

    // Render bottom hint
    let hint_para = Paragraph::new(t("about_hint"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint_para, popup_chunks[1]);

    // Render block border and title
    f.render_widget(block, area);
}

/// Title, build information, links, license and credited libraries.
fn about_lines(styles: &AboutStyles) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Pairee", styles.title.fg(Color::LightCyan)),
            Span::styled(t("about_tagline"), styles.title),
        ]),
        Line::from(Span::styled(
            "==============================",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let git_suffix = env!("PAIREE_GIT_HASH");
    let version_str = if git_suffix == "no-git" {
        env!("CARGO_PKG_VERSION").to_string()
    } else {
        format!("{} ({})", env!("CARGO_PKG_VERSION"), git_suffix)
    };
    let info_rows = [
        ("about_version", version_str, styles.text),
        (
            "about_target",
            env!("PAIREE_TARGET").to_string(),
            styles.text,
        ),
        (
            "about_profile",
            env!("PAIREE_BUILD_PROFILE").to_string(),
            styles.text,
        ),
        ("about_website", "pairee.fitty.ar".to_string(), styles.link),
        (
            "about_github",
            "https://github.com/FittyAr/Pairee".to_string(),
            styles.link,
        ),
        (
            "about_license",
            "GNU General Public License v3.0".to_string(),
            styles.text,
        ),
    ];
    for (key, value, value_style) in info_rows {
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", t(key)), styles.bold),
            Span::styled(value, value_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(t("about_libraries"), styles.bold)));
    lines.push(Line::from(Span::styled(
        "---------------------------",
        Style::default().fg(Color::DarkGray),
    )));
    lines.extend(library_lines(styles));
    lines
}

/// Two lines per credited library: name and license, then the repository.
fn library_lines(styles: &AboutStyles) -> impl Iterator<Item = Line<'static>> + '_ {
    DEPENDENCY_LIBRARIES
        .iter()
        .flat_map(move |(name, license, url)| {
            [
                Line::from(vec![
                    Span::styled("• ", Style::default().fg(Color::Cyan)),
                    Span::styled(*name, styles.bold.fg(Color::LightGreen)),
                    Span::styled(format!(" ({})", license), styles.text),
                ]),
                Line::from(vec![
                    Span::styled("  ", styles.text),
                    Span::styled(
                        *url,
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ]),
            ]
        })
}
