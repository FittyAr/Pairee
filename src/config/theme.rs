use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub panel_bg: String,
    pub panel_fg: String,
    pub panel_border: String,
    pub selection_bg: String,
    pub selection_fg: String,
    pub marked_fg: String,
    pub header_bg: String,
    pub header_fg: String,
    pub cli_bg: String,
    pub cli_fg: String,
    pub fkey_num_fg: String,
    pub fkey_text_fg: String,
    pub fkey_bg: String,
    pub popup_bg: String,
    pub popup_fg: String,
    pub popup_border: String,
    /// Syntax highlighting colors of the editor and viewer (`[syntax]`;
    /// theme files without it get the default ones).
    #[serde(default)]
    pub syntax: SyntaxColors,
    /// Semantic colors of dialogs, progress bars and status labels (`[ui]`;
    /// theme files without it get the default ones).
    #[serde(default)]
    pub ui: UiColors,
}

/// Semantic colors shared by dialogs, status labels and progress bars.
/// The same defaults serve every built-in theme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiColors {
    /// Borders and titles of informational dialogs, progress bars, bullets.
    pub accent: String,
    /// Second accent: the application name, copies to the left panel.
    pub accent_alt: String,
    /// Borders of confirmations, key hints, hotkeys, labels to notice.
    pub warning: String,
    /// Second warning tone: entries found only in the right folder.
    pub warning_alt: String,
    /// Destructive dialogs, deleted entries, cancelled jobs.
    pub danger: String,
    /// Error messages, conflicts, failed jobs.
    pub error: String,
    /// Confirmed actions, added entries, running jobs.
    pub success: String,
    /// Completed jobs, entries found only in the left folder, names.
    pub success_alt: String,
    /// Dim text (hints, separators, inactive items); background of the menu
    /// bar and of the key labels.
    pub muted: String,
    /// Secondary text: details, logs, queued jobs.
    pub secondary: String,
    /// Folders in lists, verifying jobs.
    pub info: String,
    /// Links, the menu row whose submenu is open.
    pub link: String,
    /// Untracked files, code in help pages.
    pub special: String,
    /// Strong text and text on danger labels.
    pub strong: String,
    /// Text on accent, warning and success labels.
    pub badge_fg: String,
    /// Background of alerts over the transfer panel (file conflicts).
    pub alert_bg: String,
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            accent: "Cyan".into(),
            accent_alt: "LightCyan".into(),
            warning: "Yellow".into(),
            warning_alt: "LightYellow".into(),
            danger: "Red".into(),
            error: "LightRed".into(),
            success: "Green".into(),
            success_alt: "LightGreen".into(),
            muted: "DarkGray".into(),
            secondary: "Gray".into(),
            info: "LightBlue".into(),
            link: "Blue".into(),
            special: "Magenta".into(),
            strong: "White".into(),
            badge_fg: "Black".into(),
            alert_bg: "Black".into(),
        }
    }
}

/// Colors of the token classes of syntax highlighting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyntaxColors {
    pub comment: String,
    pub string: String,
    pub keyword: String,
    /// Numbers, `true` / `false`, escapes.
    pub constant: String,
    pub function: String,
    pub type_name: String,
    pub operator: String,
    /// Markup tags, headings and links.
    pub tag: String,
}

impl Default for SyntaxColors {
    fn default() -> Self {
        Self::from_list([
            "DarkGray",
            "Green",
            "LightMagenta",
            "LightRed",
            "LightYellow",
            "LightCyan",
            "Gray",
            "LightBlue",
        ])
    }
}

impl SyntaxColors {
    /// The colors in field order.
    fn from_list(c: [&str; 8]) -> Self {
        Self {
            comment: c[0].into(),
            string: c[1].into(),
            keyword: c[2].into(),
            constant: c[3].into(),
            function: c[4].into(),
            type_name: c[5].into(),
            operator: c[6].into(),
            tag: c[7].into(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        // Modern slate/dark-mode theme as default
        Self {
            name: "slate".to_string(),
            panel_bg: "Reset".to_string(),
            panel_fg: "White".to_string(),
            panel_border: "DarkGray".to_string(),
            selection_bg: "Blue".to_string(),
            selection_fg: "White".to_string(),
            marked_fg: "Yellow".to_string(),
            header_bg: "Reset".to_string(),
            header_fg: "Cyan".to_string(),
            cli_bg: "Reset".to_string(),
            cli_fg: "White".to_string(),
            fkey_num_fg: "White".to_string(),
            fkey_text_fg: "Black".to_string(),
            fkey_bg: "Cyan".to_string(),
            popup_bg: "Black".to_string(),
            popup_fg: "White".to_string(),
            popup_border: "DarkGray".to_string(),
            syntax: SyntaxColors::default(),
            ui: UiColors::default(),
        }
    }
}

impl Theme {
    /// Generates the classic Norton Commander blue/cyan interface colors.
    pub fn classic_blue() -> Self {
        Self {
            name: "classic_blue".to_string(),
            panel_bg: "#0000AA".to_string(),
            panel_fg: "#AAAAAA".to_string(),
            panel_border: "#55FFFF".to_string(),
            selection_bg: "#00AAAA".to_string(),
            selection_fg: "#000000".to_string(),
            marked_fg: "#FFFF55".to_string(),
            header_bg: "#0000AA".to_string(),
            header_fg: "#55FFFF".to_string(),
            cli_bg: "#000000".to_string(),
            cli_fg: "#AAAAAA".to_string(),
            fkey_num_fg: "#FFFFFF".to_string(),
            fkey_text_fg: "#000000".to_string(),
            fkey_bg: "#00AAAA".to_string(),
            popup_bg: "#AAAAAA".to_string(),
            popup_fg: "#000000".to_string(),
            popup_border: "#000000".to_string(),
            syntax: SyntaxColors::from_list([
                "#00AAAA", "#55FF55", "#FFFFFF", "#FF55FF", "#FFFF55", "#55FFFF", "#FFFFFF",
                "#FF5555",
            ]),
            ui: UiColors::default(),
        }
    }
}

/// One editable theme color: its field name and accessors.
pub struct ColorProp {
    pub name: &'static str,
    pub get: fn(&Theme) -> &String,
    pub get_mut: fn(&mut Theme) -> &mut String,
}

/// Builds [`COLOR_PROPS`] from the field list, so the order and the
/// accessors cannot drift apart.
macro_rules! color_props {
    ([$($field:ident),* $(,)?] [$($syntax:ident),* $(,)?] [$($ui:ident),* $(,)?]) => {
        /// Theme colors in the order the "Color groups" dialog lists them.
        pub const COLOR_PROPS: &[ColorProp] = &[$(ColorProp {
            name: stringify!($field),
            get: |t| &t.$field,
            get_mut: |t| &mut t.$field,
        },)* $(ColorProp {
            name: concat!("syntax_", stringify!($syntax)),
            get: |t| &t.syntax.$syntax,
            get_mut: |t| &mut t.syntax.$syntax,
        },)* $(ColorProp {
            name: concat!("ui_", stringify!($ui)),
            get: |t| &t.ui.$ui,
            get_mut: |t| &mut t.ui.$ui,
        },)*];
    };
}

/// Calls `$callback!` with every theme color field in dialog order, then
/// every [`SyntaxColors`] field, then every [`UiColors`] field: the one list [`COLOR_PROPS`] and the UI's
/// resolved theme are both built from.
#[macro_export]
macro_rules! with_theme_colors {
    ($callback:ident) => {
        $callback!(
            [panel_bg,
            panel_fg,
            panel_border,
            selection_bg,
            selection_fg,
            marked_fg,
            header_bg,
            header_fg,
            cli_bg,
            cli_fg,
            fkey_num_fg,
            fkey_text_fg,
            fkey_bg,
            popup_bg,
            popup_fg,
            popup_border,]
            [comment, string, keyword, constant, function, type_name, operator, tag]
            [accent, accent_alt, warning, warning_alt, danger, error, success, success_alt,
            muted, secondary, info, link, special, strong, badge_fg, alert_bg]
        );
    };
}

with_theme_colors!(color_props);

/// Named colors cycled with Left / Right in the color dialogs.
pub const NAMED_COLORS: [&str; 17] = [
    "Reset",
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "Gray",
    "DarkGray",
    "LightRed",
    "LightGreen",
    "LightYellow",
    "LightBlue",
    "LightMagenta",
    "LightCyan",
    "White",
];

/// The named color `step` places after `current` (wrapping; unknown names
/// count as "Reset").
pub fn cycle_named_color(current: &str, forward: bool) -> String {
    let len = NAMED_COLORS.len();
    let idx = NAMED_COLORS.iter().position(|&c| c == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % len
    } else {
        (idx + len - 1) % len
    };
    NAMED_COLORS[next].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_props_cover_fields_in_order() {
        let mut theme = Theme::default();
        assert_eq!(COLOR_PROPS.len(), 40);
        assert_eq!(COLOR_PROPS[24].name, "ui_accent");
        *(COLOR_PROPS[39].get_mut)(&mut theme) = "Red".into();
        assert_eq!(theme.ui.alert_bg, "Red");
        assert_eq!(COLOR_PROPS[16].name, "syntax_comment");
        *(COLOR_PROPS[23].get_mut)(&mut theme) = "Blue".into();
        assert_eq!(theme.syntax.tag, "Blue");
        assert_eq!(COLOR_PROPS[3].name, "selection_bg");
        *(COLOR_PROPS[15].get_mut)(&mut theme) = "Red".into();
        assert_eq!(theme.popup_border, "Red");
        assert_eq!((COLOR_PROPS[0].get)(&theme), &theme.panel_bg);
    }

    #[test]
    fn theme_files_without_syntax_colors_still_load() {
        let mut table = toml::Table::try_from(Theme::classic_blue()).unwrap();
        table.remove("syntax");
        table.remove("ui");
        let theme: Theme = toml::from_str(&table.to_string()).unwrap();
        assert_eq!(theme.panel_bg, "#0000AA");
        assert_eq!(theme.syntax, SyntaxColors::default());
        assert_eq!(theme.ui, UiColors::default());
        let partial: UiColors = toml::from_str("accent = \"Red\"").unwrap();
        assert_eq!(
            (partial.accent.as_str(), partial.muted.as_str()),
            ("Red", "DarkGray")
        );
        let partial: SyntaxColors = toml::from_str("comment = \"Red\"").unwrap();
        assert_eq!(
            (partial.comment.as_str(), partial.string.as_str()),
            ("Red", "Green")
        );
    }

    #[test]
    fn named_colors_wrap() {
        assert_eq!(cycle_named_color("Reset", false), "White");
        assert_eq!(cycle_named_color("White", true), "Reset");
        assert_eq!(cycle_named_color("#123456", true), "Black");
    }
}
