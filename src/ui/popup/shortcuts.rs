//! Render the keyboard-shortcuts modal.

use crate::app::shortcuts::model::{ContextTab, ShortcutRow};
use crate::app::shortcuts::state::{Mode, ShortcutsState};
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::keybindings::catalog::Category;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::popup::kit::{ListPopup, Scroll, fg, popup_fg};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Columns of the chords column.
const CHORDS_WIDTH: usize = 30;

pub fn render(f: &mut Frame, popup: &PopupType, theme: &Theme, area: Rect) -> bool {
    let PopupType::Shortcuts(s) = popup else {
        return false;
    };
    let (rows, cursor) = list_rows(s, theme);
    ListPopup {
        area: centered_rect_fixed(area.width * 9 / 10, area.height * 17 / 20, area),
        title: format!(" {} ", t("shortcuts_title")),
        border: Color::Cyan,
        empty: Some(t("shortcuts_empty")),
        header: header(s),
        rows,
        cursor,
        scroll: Scroll::Centered,
        hint: Some(t("shortcuts_hint")),
        scrollbar: None,
    }
    .render(f, theme);
    true
}

/// Context tabs and preset, the mode line and the feedback line.
fn header(s: &ShortcutsState) -> Vec<Line<'static>> {
    let mut tabs: Vec<Span> = ContextTab::ALL
        .iter()
        .map(|tab| {
            let label = t(tab.label_key());
            if *tab == s.tab {
                Span::styled(
                    format!(" [{label}] "),
                    fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(format!("  {label}  "))
            }
        })
        .collect();
    tabs.push(Span::styled(
        format!("   {} ◂ {} ▸", t("shortcuts_preset"), s.preset),
        fg(Color::Yellow),
    ));
    vec![
        Line::from(tabs),
        Line::from(mode_line(s)),
        Line::from(Span::styled(
            s.message.clone().unwrap_or_default(),
            fg(Color::Yellow),
        )),
    ]
}

fn mode_line(s: &ShortcutsState) -> String {
    match &s.mode {
        Mode::Browse => match &s.key_filter {
            Some(chord) => t("shortcuts_key_filter").replace("{}", chord),
            None => format!("> {}", s.query.text()),
        },
        Mode::Capture { add, keys } => {
            let key = if *add {
                "shortcuts_capture_add"
            } else {
                "shortcuts_capture"
            };
            let typed = if keys.is_empty() {
                "…".to_string()
            } else {
                keys.join(" ")
            };
            t(key).replace("{}", &typed)
        }
        Mode::ConfirmReplace { chord, owner, .. } => t("shortcuts_conflict")
            .replacen("{}", chord, 1)
            .replacen("{}", owner, 1),
        Mode::KeySearch => t("shortcuts_key_search"),
        Mode::Export { name } => format!("{} {}", t("shortcuts_export_prompt"), name.text()),
        Mode::ConfirmResetAll => t("shortcuts_reset_all_confirm").replace("{}", &s.preset),
    }
}

/// Visible rows under category headings, and the line of the cursor row.
fn list_rows(s: &ShortcutsState, theme: &Theme) -> (Vec<(String, Style)>, usize) {
    let heading = fg(Color::DarkGray).add_modifier(Modifier::BOLD);
    let mut lines = Vec::new();
    let mut cursor_line = 0;
    let mut category: Option<Category> = None;
    for (i, row) in s.visible().into_iter().enumerate() {
        if category != Some(row.category) {
            category = Some(row.category);
            lines.push((format!(" ── {}", t(row.category.label_key())), heading));
        }
        if i == s.cursor {
            cursor_line = lines.len();
        }
        let style = if row.chords.is_empty() {
            fg(Color::DarkGray)
        } else {
            popup_fg(theme)
        };
        lines.push((row_text(row), style));
    }
    (lines, cursor_line)
}

fn row_text(row: &ShortcutRow) -> String {
    let chords = if row.chords.is_empty() {
        t("shortcuts_unbound")
    } else {
        row.chords.join(", ")
    };
    let chords = clip(&chords, CHORDS_WIDTH);
    let origin = match (&row.plugin, row.user) {
        (_, true) => format!("  ({})", t("shortcuts_origin_user")),
        (Some(plugin), false) => format!("  ({plugin})"),
        (None, false) => String::new(),
    };
    format!(
        " {} {chords:<CHORDS_WIDTH$} {}{origin}",
        row.mark(),
        row.label
    )
}

/// `text` cut to `width` characters, with `…` when cut.
fn clip(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_chord_lists_are_clipped() {
        assert_eq!(clip("F5", 4), "F5");
        assert_eq!(clip("abcdef", 4), "abc…");
    }
}
