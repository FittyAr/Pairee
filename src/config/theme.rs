use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ($($field:ident),* $(,)?) => {
        /// Theme colors in the order the "Color groups" dialog lists them.
        pub const COLOR_PROPS: &[ColorProp] = &[$(ColorProp {
            name: stringify!($field),
            get: |t| &t.$field,
            get_mut: |t| &mut t.$field,
        }),*];
    };
}

color_props!(
    panel_bg,
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
    popup_border,
);

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
        assert_eq!(COLOR_PROPS.len(), 16);
        assert_eq!(COLOR_PROPS[3].name, "selection_bg");
        *(COLOR_PROPS[15].get_mut)(&mut theme) = "Red".into();
        assert_eq!(theme.popup_border, "Red");
        assert_eq!((COLOR_PROPS[0].get)(&theme), &theme.panel_bg);
    }

    #[test]
    fn named_colors_wrap() {
        assert_eq!(cycle_named_color("Reset", false), "White");
        assert_eq!(cycle_named_color("White", true), "Reset");
        assert_eq!(cycle_named_color("#123456", true), "Black");
    }
}
