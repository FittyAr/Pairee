//! The theme with every color parsed once (when the theme loads or changes)
//! and the styles dialogs share, so renderers never parse color strings.

use crate::config::theme::Theme;
use crate::ui::theme_apply::parse_color;
use ratatui::style::{Color, Modifier, Style};

macro_rules! resolved_theme {
    ($($field:ident),* $(,)?) => {
        /// [`Theme`] colors as Ratatui colors.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct ResolvedTheme {
            $(pub $field: Color,)*
        }

        impl ResolvedTheme {
            pub fn new(theme: &Theme) -> Self {
                Self {
                    $($field: parse_color(&theme.$field),)*
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
        let slate = ResolvedTheme::default();
        assert_eq!(slate.selection_bg, Color::Blue);
        assert_eq!(
            slate.selection(),
            Style::default().bg(Color::Blue).fg(Color::White)
        );
    }
}
