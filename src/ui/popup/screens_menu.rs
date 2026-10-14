use super::kit::{ListPopup, Scroll, list_area};
use crate::app::state::Screen;
use crate::app::state::popup::ScreensMenu;
use crate::config::localization::t;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for ScreensMenu {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (state, theme, size) = (cx.state, cx.theme, cx.area);
        let ScreensMenu { cursor_idx, .. } = self;
        let rows: Vec<(String, _)> = state
            .screens
            .iter()
            .enumerate()
            .map(|(i, screen)| {
                let active_marker = if i == state.screens.active_index() {
                    "*"
                } else {
                    " "
                };
                let name = screen_name(screen);
                (
                    format!("{} {} {}", active_marker, i + 1, name),
                    theme.popup_fg(),
                )
            })
            .collect();
        let width = rows.iter().map(|(text, _)| text.len()).max().unwrap_or(0);
        ListPopup {
            area: list_area(size, width.max(20), rows.len(), 20),
            title: t("screens_menu_title"),
            border: theme.ui.warning,
            empty: None,
            header: Vec::new(),
            rows,
            cursor: *cursor_idx,
            scroll: Scroll::Centered,
            hint: Some(t("screens_menu_hint")),
            scrollbar: None,
        }
        .render(f, theme);
    }
}

/// How the list names `screen`.
fn screen_name(screen: &Screen) -> String {
    let path =
        |key: &str, path: &std::path::Path| t(key).replacen("{}", &path.display().to_string(), 1);
    match screen {
        Screen::Panels => t("screens_menu_panels"),
        Screen::Editor(ed) => path("screens_menu_editor", &ed.path),
        Screen::Viewer(vw) => path("screens_menu_viewer", &vw.path),
        Screen::Terminal(ts) => {
            let status = if ts.is_running() {
                t("terminal_status_running")
            } else {
                t("terminal_status_done")
            };
            t("screens_menu_terminal")
                .replacen("{}", &ts.command, 1)
                .replacen("{}", &status, 1)
        }
    }
}
