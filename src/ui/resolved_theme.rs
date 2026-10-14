//! The theme with every color parsed once (when the theme loads or changes)
//! and the styles dialogs share, so renderers never parse color strings.

use crate::config::theme::Theme;
use crate::git::status::StatusKind;
use crate::syntax::SyntaxKind;
use crate::ui::theme_apply::parse_color;
use ratatui::style::{Color, Modifier, Style};

macro_rules! resolved_theme {
    ([$($field:ident),* $(,)?] [$($syntax:ident),* $(,)?] [$($ui:ident),* $(,)?]) => {
        /// [`Theme`] colors as Ratatui colors.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct ResolvedTheme {
            $(pub $field: Color,)*
            pub syntax: SyntaxPalette,
            pub ui: UiPalette,
        }

        /// [`crate::config::theme::SyntaxColors`] as Ratatui colors.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct SyntaxPalette {
            $(pub $syntax: Color,)*
        }

        /// [`crate::config::theme::UiColors`] as Ratatui colors.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct UiPalette {
            $(pub $ui: Color,)*
        }

        impl ResolvedTheme {
            pub fn new(theme: &Theme) -> Self {
                Self {
                    $($field: parse_color(&theme.$field),)*
                    syntax: SyntaxPalette {
                        $($syntax: parse_color(&theme.syntax.$syntax),)*
                    },
                    ui: UiPalette {
                        $($ui: parse_color(&theme.ui.$ui),)*
                    },
                }
            }
        }
    };
}

crate::with_theme_colors!(resolved_theme);

impl Default for ResolvedTheme {
    fn default() -> Self {
        Self::new(&Theme::default())
    }
}

impl From<&Theme> for ResolvedTheme {
    fn from(theme: &Theme) -> Self {
        Self::new(theme)
    }
}

impl ResolvedTheme {
    /// Selection colors (cursor row, highlighted list rows, focused buttons).
    pub fn selection(&self) -> Style {
        Style::default().bg(self.selection_bg).fg(self.selection_fg)
    }

    /// Background and foreground of a popup list row: the selection colors
    /// on the cursor row, the popup colors elsewhere.
    pub fn row_colors(&self, selected: bool) -> (Color, Color) {
        if selected {
            (self.selection_bg, self.selection_fg)
        } else {
            (self.popup_bg, self.popup_fg)
        }
    }

    /// Popup foreground.
    pub fn popup_fg(&self) -> Style {
        Style::default().fg(self.popup_fg)
    }

    /// Text of syntax class `kind` (editor, viewer).
    pub fn syntax(&self, kind: SyntaxKind) -> Style {
        let p = &self.syntax;
        Style::default().fg(match kind {
            SyntaxKind::Comment => p.comment,
            SyntaxKind::String => p.string,
            SyntaxKind::Keyword => p.keyword,
            SyntaxKind::Constant => p.constant,
            SyntaxKind::Function => p.function,
            SyntaxKind::TypeName => p.type_name,
            SyntaxKind::Operator => p.operator,
            SyntaxKind::Tag => p.tag,
        })
    }

    /// Color of a Git status (panel names, Git panel).
    pub fn git_status(&self, kind: &StatusKind) -> Color {
        let ui = &self.ui;
        match kind {
            StatusKind::Modified => ui.warning,
            StatusKind::Added => ui.success,
            StatusKind::Deleted => ui.danger,
            StatusKind::Untracked => ui.special,
            StatusKind::Renamed => ui.accent,
            StatusKind::Conflicted => ui.error,
        }
    }

    /// A search match inside text (viewer, editor).
    pub fn search_match(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.marked_fg)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_color_once() {
        let resolved = ResolvedTheme::new(&Theme::classic_blue());
        assert_eq!(resolved.panel_bg, Color::Rgb(0, 0, 0xAA));
        assert_eq!(resolved.selection_fg, Color::Rgb(0, 0, 0));
        assert_eq!(resolved.syntax.string, Color::Rgb(0x55, 0xFF, 0x55));
        assert_eq!(
            resolved.syntax(SyntaxKind::Comment),
            Style::default().fg(Color::Rgb(0, 0xAA, 0xAA))
        );
        let slate = ResolvedTheme::default();
        assert_eq!(slate.syntax.comment, Color::DarkGray);
        assert_eq!(slate.selection_bg, Color::Blue);
        assert_eq!(
            slate.selection(),
            Style::default().bg(Color::Blue).fg(Color::White)
        );
    }

    #[test]
    fn built_in_themes_keep_the_former_fixed_colors() {
        for theme in [Theme::default(), Theme::classic_blue()] {
            let ui = ResolvedTheme::new(&theme).ui;
            assert_eq!(
                [
                    ui.accent, ui.warning, ui.danger, ui.error, ui.success, ui.muted
                ],
                [
                    Color::Cyan,
                    Color::Yellow,
                    Color::Red,
                    Color::LightRed,
                    Color::Green,
                    Color::DarkGray
                ]
            );
            assert_eq!((ui.badge_fg, ui.alert_bg), (Color::Black, Color::Black));
        }
    }

    #[test]
    fn git_statuses_share_one_palette() {
        let theme = ResolvedTheme::default();
        for label in ["M", "A", "D", "?", "R", "!"] {
            let kind = StatusKind::from_label(label).unwrap();
            assert_eq!(kind.label(), label);
        }
        assert_eq!(StatusKind::from_label("x"), None);
        assert_eq!(theme.git_status(&StatusKind::Untracked), Color::Magenta);
        assert_eq!(theme.git_status(&StatusKind::Renamed), Color::Cyan);
    }
}
