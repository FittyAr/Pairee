use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_fkeys(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;

    let is_editor = matches!(
        state.screens.get(state.active_screen_idx),
        Some(crate::app::state::Screen::Editor(_))
    );
    let is_viewer = matches!(
        state.screens.get(state.active_screen_idx),
        Some(crate::app::state::Screen::Viewer(_))
    );

    let modifiers = state
        .fkeys_modifier_override
        .unwrap_or(state.current_modifiers);

    let fkeys = if is_editor {
        vec![
            ("1", t("fkey_help")),
            ("2", t("fkey_ed_save")),
            ("3", String::new()),
            ("4", t("fkey_ed_hex")),
            ("5", String::new()),
            ("6", String::new()),
            ("7", t("fkey_ed_search")),
            ("8", t("fkey_ed_discard")),
            ("9", String::new()),
            ("10", t("fkey_ed_quit")),
            ("11", String::new()),
            ("12", String::new()),
        ]
    } else if is_viewer {
        vec![
            ("1", t("fkey_help")),
            ("2", String::new()),
            ("3", String::new()),
            ("4", t("fkey_vw_hex")),
            ("5", String::new()),
            ("6", String::new()),
            ("7", t("fkey_vw_search")),
            ("8", String::new()),
            ("9", String::new()),
            ("10", t("fkey_vw_quit")),
            ("11", String::new()),
            ("12", String::new()),
        ]
    } else if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        vec![
            ("1", t("fkey_ctrl_left")),
            ("2", t("fkey_ctrl_right")),
            ("3", t("fkey_ctrl_name")),
            ("4", t("fkey_ctrl_extens")),
            ("5", t("fkey_ctrl_time")),
            ("6", t("fkey_ctrl_size")),
            ("7", t("fkey_ctrl_unsort")),
            ("8", t("fkey_ctrl_creatn")),
            ("9", t("fkey_ctrl_access")),
            ("10", t("fkey_ctrl_descr")),
            ("11", t("fkey_ctrl_owner")),
            ("12", t("fkey_ctrl_sort")),
        ]
    } else if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        vec![
            ("1", t("fkey_alt_left")),
            ("2", t("fkey_alt_right")),
            ("3", t("fkey_alt_view")),
            ("4", t("fkey_alt_edit")),
            ("5", t("fkey_alt_print")),
            ("6", t("fkey_alt_mklink")),
            ("7", t("fkey_alt_find")),
            ("8", t("fkey_alt_history")),
            ("9", t("fkey_alt_video")),
            ("10", t("fkey_alt_tree")),
            ("11", t("fkey_alt_viewhs")),
            ("12", t("fkey_alt_foldhs")),
        ]
    } else {
        vec![
            ("1", t("fkey_help")),
            ("2", t("fkey_menu")),
            ("3", t("fkey_view")),
            ("4", t("fkey_edit")),
            ("5", t("fkey_copy")),
            ("6", t("fkey_renmov")),
            ("7", t("fkey_mkdir")),
            ("8", t("fkey_delete")),
            ("9", t("fkey_pulldn")),
            ("10", t("fkey_quit")),
            ("11", t("fkey_plugin")),
            ("12", t("fkey_screen")),
        ]
    };

    // Divide the row into 12 equal columns
    let constraints = vec![Constraint::Ratio(1, 12); 12];
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let num_style = Style::default()
        .bg(parse_color(&theme.fkey_bg))
        .fg(parse_color(&theme.fkey_num_fg));

    let text_style = Style::default()
        .bg(parse_color("DarkGray"))
        .fg(parse_color(&theme.fkey_text_fg));

    for (i, (num, text)) in fkeys.iter().enumerate() {
        let block_area = columns[i];

        // Compose block as " 1 Help   "
        let line = Line::from(vec![
            Span::styled(format!(" {:>2}", num), num_style),
            Span::styled(format!(" {:<6}", text), text_style),
        ]);

        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, block_area);
    }
}
