//! The editor's find / replace dialog: one row per [`FindRow`], painted
//! from the same table the keys use.

use crate::app::state::popup::{EditorSearchPrompt, FindRow};
use crate::config::localization::t;
use crate::ui::popup::kit::{self, FocusStyles, mouse::FormFocus};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, layout::Rect};

const WIDTH: u16 = 58;
/// Rows besides the fields and checkboxes: frame (2), separator, spacer,
/// buttons and the history hint.
const EXTRA_ROWS: u16 = 6;

impl DialogView for EditorSearchPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let theme = cx.theme;
        let rows = self.rows();
        let is_field = |r: &FindRow| matches!(r, FindRow::Query | FindRow::Replacement);
        let fields = rows.iter().filter(|r| is_field(r)).count() as u16;
        let flags = rows
            .iter()
            .filter(|r| matches!(r, FindRow::Flag(_)))
            .count() as u16;
        let border = kit::fg(theme.popup_border);
        let size = (WIDTH, fields * 2 + flags + EXTRA_ROWS);
        let inner = kit::dialog_frame(f, cx.area, size, self.title(), border, theme);
        let styles = FocusStyles {
            active: theme.selection(),
            normal: theme.popup_fg(),
        };
        let focus = FormFocus::new(self.form(), self.cursor_idx);
        let mut y = inner.y;
        let mut take = |h: u16| {
            let area = Rect::new(inner.x, y, inner.width, h).intersection(inner);
            y = y.saturating_add(h);
            area
        };
        for (i, row) in rows.iter().enumerate() {
            match row {
                FindRow::Query => {
                    let label = t("search_query_label");
                    kit::form_labelled_field(f, take(2), (&label, &self.query), i, focus, styles);
                }
                FindRow::Replacement => {
                    if let Some(field) = &self.replacement {
                        let label = t("editor_replace_with_label");
                        kit::form_labelled_field(f, take(2), (&label, field), i, focus, styles);
                    }
                }
                FindRow::Flag(flag) => {
                    if i > 0 && is_field(&rows[i - 1]) {
                        f.render_widget(kit::separator(inner.width, border), take(1));
                    }
                    let text = kit::checkbox_row(flag.get(self.options), &t(flag.label_key()));
                    kit::form_option(f, take(1), &text, i, focus, styles);
                }
                FindRow::Button(_) => {}
            }
        }
        take(1);
        let labels: Vec<String> = rows
            .iter()
            .filter_map(|r| match r {
                FindRow::Button(b) => Some(t(b.label_key())),
                _ => None,
            })
            .collect();
        kit::form_buttons(f, take(1), &labels, focus, styles);
        f.render_widget(kit::hint(t("editor_find_history_hint"), theme), take(1));
    }
}

impl EditorSearchPrompt {
    fn title(&self) -> String {
        t(if self.replacement.is_some() {
            "editor_replace_title"
        } else {
            "editor_search_title"
        })
    }
}
