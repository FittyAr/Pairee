//! Find dialog shared by the viewer (F7) and the editor (Ctrl+F / F7).

use crate::app::state::PopupType;
use crate::app::state::popup::TextSearchState as Search;
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub fn render_viewer_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::ViewerSearchPrompt(search) => {
            render_search(f, "viewer_search_title", search, theme, size)
        }
        _ => false,
    }
}

/// Query field, case checkbox and [Search] [Cancel] buttons.
pub fn render_search(
    f: &mut Frame,
    title_key: &str,
    search: &Search,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let border = kit::fg(parse_color(&theme.popup_border));
    let inner = kit::dialog_frame(f, size, (50, 9), t(title_key), border, theme);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // 0: Search query prompt + input line
            Constraint::Length(1), // 1: Separator line
            Constraint::Length(1), // 2: Case sensitive checkbox
            Constraint::Min(1),    // 3: Spacer
            Constraint::Length(1), // 4: Buttons
        ])
        .split(inner);

    let styles = FocusStyles {
        active: kit::selection(theme),
        normal: kit::popup_fg(theme),
    };
    let focus = search.cursor_idx;
    f.render_widget(
        kit::labelled_field(&t("search_query_label"), &search.query, focus == 0, styles),
        chunks[0],
    );
    f.render_widget(kit::separator(inner.width, border), chunks[1]);
    f.render_widget(
        kit::marked_row(
            &kit::checkbox_row(search.case_sensitive, &t("sys_case_sensitive")),
            focus == Search::ROW_CASE,
            styles,
        ),
        chunks[2],
    );
    f.render_widget(
        kit::button_bar(
            &[t("btn_search_bracket"), t("btn_cancel_bracket")],
            Search::FORM.focused_button(focus),
            styles,
        ),
        chunks[4],
    );
    true
}
