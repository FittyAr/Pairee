use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

// ─────────────────────────────────────────────────────────────────────────────
// Menu definitions — fully matches norton_commander_features.md sections 1–5
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the menu item labels for a given top-level menu index.
/// - 0 = Left   (panel config for the left panel)
/// - 1 = Files  (file operations)
/// - 2 = Commands
/// - 3 = Options
/// - 4 = Right  (panel config for the right panel)
pub fn get_menu_items(menu_idx: usize) -> &'static [&'static str] {
    match menu_idx {
        // ── Left (mirrors Right exactly, just different drive shortcut) ───────
        0 => &[
            " Brief          Ctrl+1 ",
            " Medium         Ctrl+2 ",
            " Full           Ctrl+3 ",
            " Wide           Ctrl+4 ",
            " Detailed       Ctrl+5 ",
            " Descriptions   Ctrl+6 ",
            " File owners    Ctrl+7 ",
            " File links     Ctrl+8 ",
            " Alt full       Ctrl+9 ",
            " ─────────────────────── ",
            " Info panel     Ctrl+L ",
            " Quick view     Ctrl+Q ",
            " ─────────────────────── ",
            " Sort modes    Ctrl+F12 ",
            " Show long names Ctrl+N ",
            " Panel On/Off   Ctrl+F1 ",
            " Re-read        Ctrl+R ",
            " Change drive   Alt+F1 ",
        ],
        // ── Files ─────────────────────────────────────────────────────────────
        1 => &[
            " View               F3 ",
            " Edit               F4 ",
            " Copy               F5 ",
            " Rename/Move        F6 ",
            " Link            Alt+F6 ",
            " Make Folder        F7 ",
            " Delete             F8 ",
            " Wipe           Alt+Del ",
            " ─────────────────────── ",
            " Add to archive  Shf+F1 ",
            " Extract files   Shf+F2 ",
            " Archive commands Shf+F3 ",
            " ─────────────────────── ",
            " File attributes Ctrl+A ",
            " Apply command   Ctrl+G ",
            " Describe files  Ctrl+Z ",
            " ─────────────────────── ",
            " Select group     Gray+ ",
            " Unselect group   Gray- ",
            " Invert selection Gray* ",
            " Restore selection Ctrl+M ",
        ],
        // ── Commands ──────────────────────────────────────────────────────────
        2 => &[
            " Find file      Alt+F7 ",
            " History        Alt+F8 ",
            " File view hist Alt+F11 ",
            " Folders hist   Alt+F12 ",
            " ─────────────────────── ",
            " Swap panels    Ctrl+U ",
            " Panels On/Off  Ctrl+O ",
            " Compare folders        ",
            " ─────────────────────── ",
            " Edit user menu         ",
            " File associations      ",
            " Folder shortcuts       ",
            " File panel filter Ctrl+I ",
            " ─────────────────────── ",
            " Plugin commands   F11 ",
            " Screens list      F12 ",
            " Task list      Ctrl+W ",
            " Hotplug devices        ",
        ],
        // ── Options ───────────────────────────────────────────────────────────
        3 => &[
            " System settings        ",
            " Panel settings         ",
            " Interface settings     ",
            " ─────────────────────── ",
            " Confirmations          ",
            " File panel modes       ",
            " File descriptions      ",
            " ─────────────────────── ",
            " Viewer settings        ",
            " Editor settings        ",
            " Code pages             ",
            " ─────────────────────── ",
            " Colors                 ",
            " Files highlighting     ",
            " ─────────────────────── ",
            " Theme: Slate           ",
            " Theme: Classic Blue    ",
            " ─────────────────────── ",
            " Preset: Norton         ",
            " Preset: Vim            ",
            " Preset: Modern         ",
            " ─────────────────────── ",
            " Toggle Hidden   Ctrl+H ",
            " Save setup      Shf+F9 ",
        ],
        // ── Right (mirrors Left) ──────────────────────────────────────────────
        4 => &[
            " Brief          Ctrl+1 ",
            " Medium         Ctrl+2 ",
            " Full           Ctrl+3 ",
            " Wide           Ctrl+4 ",
            " Detailed       Ctrl+5 ",
            " Descriptions   Ctrl+6 ",
            " File owners    Ctrl+7 ",
            " File links     Ctrl+8 ",
            " Alt full       Ctrl+9 ",
            " ─────────────────────── ",
            " Info panel     Ctrl+L ",
            " Quick view     Ctrl+Q ",
            " ─────────────────────── ",
            " Sort modes    Ctrl+F12 ",
            " Show long names Ctrl+N ",
            " Panel On/Off   Ctrl+F2 ",
            " Re-read        Ctrl+R ",
            " Change drive   Alt+F2 ",
        ],
        _ => &[],
    }
}

/// Returns the display labels for the top-level menu bar.
pub fn get_menu_titles() -> &'static [&'static str] {
    &[
        "  Left  ",
        "  Files  ",
        "  Commands  ",
        "  Options  ",
        "  Right  ",
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Renderer
// ─────────────────────────────────────────────────────────────────────────────

pub fn render_menu(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;

    let active_menu_idx = if let Some(PopupType::Menu {
        active_menu_idx, ..
    }) = state.active_popup
    {
        Some(active_menu_idx)
    } else {
        None
    };

    let mut spans = Vec::new();
    for (i, title) in get_menu_titles().iter().enumerate() {
        let is_active = Some(i) == active_menu_idx;
        let style = if is_active {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(parse_color(&theme.panel_fg))
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(*title, style));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(parse_color("DarkGray")));
    f.render_widget(paragraph, area);
}
