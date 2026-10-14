//! The viewer's find dialog (F7) and encoding selector (F8).

use crate::app::state::popup::TextSearchState as Search;
use crate::app::state::popup::{ViewerEncoding, ViewerSearchPrompt};
use crate::config::localization::t;
use crate::fs::text::ENCODINGS;
use crate::ui::popup::kit::{self, FocusStyles, ListPopup, Scroll, list_area, marked};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

impl DialogView for ViewerSearchPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render_search(f, "viewer_search_title", self, cx.theme, cx.area);
    }
}

impl DialogView for ViewerEncoding {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render_encodings(f, self.cursor_idx, cx.theme, cx.area);
    }
}

/// The list of encodings the viewer can switch to.
fn render_encodings(
    f: &mut Frame,
    cursor: usize,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) {
    let rows: Vec<(String, _)> = ENCODINGS
        .iter()
        .enumerate()
        .map(|(i, e)| (marked(e.name(), i == cursor), theme.popup_fg()))
        .collect();
    let width = rows.iter().map(|(text, _)| text.len()).max().unwrap_or(0);
    let hint = t("viewer_encoding_hint");
    ListPopup {
        area: list_area(
            size,
            width.max(crate::ui::text_width::display_width(&hint)),
            rows.len() + 2,
            20,
        ),
        title: t("viewer_encoding_title"),
        border: theme.popup_border,
        empty: None,
        header: Vec::new(),
        rows,
        cursor,
        scroll: Scroll::Centered,
        hint: Some(hint),
        scrollbar: None,
    }
    .render(f, theme);
}

/// Query field, case checkbox and [Search] [Cancel] buttons.
fn render_search(
    f: &mut Frame,
    title_key: &str,
    search: &Search,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    size: Rect,
) {
    let border = kit::fg(theme.popup_border);
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
        active: theme.selection(),
        normal: theme.popup_fg(),
    };
    let focus = kit::mouse::FormFocus::new(Search::FORM, search.cursor_idx);
    kit::form_labelled_field(
        f,
        chunks[0],
        (&t("search_query_label"), &search.query),
        0,
        focus,
        styles,
    );
    f.render_widget(kit::separator(inner.width, border), chunks[1]);
    kit::form_option(
        f,
        chunks[2],
        &kit::checkbox_row(search.case_sensitive, &t("sys_case_sensitive")),
        Search::ROW_CASE,
        focus,
        styles,
    );
    kit::form_buttons(
        f,
        chunks[4],
        &[t("btn_search_bracket"), t("btn_cancel_bracket")],
        focus,
        styles,
    );
}
