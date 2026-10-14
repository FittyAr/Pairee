//! Directory hotlist and folder shortcut dialog rendering.

use super::super::centered_rect_in;
use crate::app::input_popup::folder_shortcuts::{SLOT_COUNT, slot_for_row};
use crate::config::bookmarks::HotlistEntry;
use crate::config::localization::t;
use crate::keybindings::{Action, KeybindingResolver};
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::resolved_theme::ResolvedTheme;
use crate::ui::text_width::pad_to_width;
use ratatui::{Frame, layout::Rect, text::Line};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn render_hotlist(
    f: &mut Frame,
    theme: &ResolvedTheme,
    size: Rect,
    entries: &[HotlistEntry],
    cursor_idx: usize,
) {
    let style = theme.popup_fg();
    let rows = entries
        .iter()
        .map(|e| {
            (
                format!(
                    " {} ->  {} ",
                    pad_to_width(&e.name, 20),
                    e.path.to_string_lossy()
                ),
                style,
            )
        })
        .collect::<Vec<_>>();
    let header = if rows.is_empty() {
        vec![Line::from(t("hotlist_empty"))]
    } else {
        Vec::new()
    };
    ListPopup {
        header,
        rows,
        cursor: cursor_idx,
        ..bookmark_popup(theme, size, t("popup_hotlist"), t("hotlist_hint"))
    }
    .render(f, theme);
}

pub fn render_folder_shortcuts(
    f: &mut Frame,
    theme: &ResolvedTheme,
    size: Rect,
    shortcuts: &HashMap<u8, PathBuf>,
    resolver: &KeybindingResolver,
    cursor_idx: usize,
) {
    let unassigned = t("folder_shortcut_unassigned");
    let style = theme.popup_fg();
    let rows = (0..SLOT_COUNT)
        .map(|i| {
            let slot = slot_for_row(i);
            let chord = resolver
                .key_for_action(Action::GoFolderShortcut(slot))
                .map(|s| s.to_string())
                .unwrap_or_else(|| slot.to_string());
            let target = shortcuts
                .get(&slot)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| unassigned.clone());
            (format!(" {chord:<12} ->  {target} "), style)
        })
        .collect();
    ListPopup {
        rows,
        cursor: cursor_idx,
        ..bookmark_popup(
            theme,
            size,
            t("popup_folder_shortcuts"),
            t("folder_shortcuts_hint"),
        )
    }
    .render(f, theme);
}

/// Frame shared by the hotlist and the folder shortcuts (rows set by the caller).
fn bookmark_popup(
    theme: &ResolvedTheme,
    size: Rect,
    title: String,
    hint: String,
) -> ListPopup<'static> {
    ListPopup {
        area: centered_rect_in(60, 40, size),
        title,
        border: theme.popup_border,
        empty: None,
        header: Vec::new(),
        rows: Vec::new(),
        cursor: 0,
        scroll: Scroll::HalfPage,
        hint: Some(hint),
        scrollbar: None,
    }
}
