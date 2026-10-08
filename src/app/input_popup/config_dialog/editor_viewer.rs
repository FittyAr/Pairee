use crate::config::settings::Settings;

/// Cycles the tab sizes offered by the dialog: 2 → 4 → 8 → 2.
fn next_tab_size(size: u32) -> u32 {
    match size {
        2 => 4,
        4 => 8,
        _ => 2,
    }
}

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        10 => settings.editor_tab_size = next_tab_size(settings.editor_tab_size),
        11 => settings.editor_expand_tabs = settings.editor_expand_tabs.next(),
        12 => settings.editor_auto_indent = !settings.editor_auto_indent,
        13 => settings.editor_show_line_numbers = !settings.editor_show_line_numbers,
        14 => settings.editor_cursor_at_end = !settings.editor_cursor_at_end,
        15 => settings.editor_lock_editing_readonly = !settings.editor_lock_editing_readonly,
        16 => settings.editor_warn_opening_readonly = !settings.editor_warn_opening_readonly,
        21 => settings.viewer_use_external = !settings.viewer_use_external,
        26 => settings.viewer_tab_size = next_tab_size(settings.viewer_tab_size),
        28 => settings.viewer_show_scrollbar = !settings.viewer_show_scrollbar,
        38 => settings.enter_use_external = !settings.enter_use_external,
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::TabExpansion;

    #[test]
    fn editor_rows_toggle_their_settings() {
        let mut s = Settings::default();
        handle_row(11, &mut s);
        assert_eq!(s.editor_expand_tabs, TabExpansion::NewTabs);
        let before = s.editor_auto_indent;
        handle_row(12, &mut s);
        assert_ne!(s.editor_auto_indent, before);
        handle_row(10, &mut s);
        assert_eq!(s.editor_tab_size, 2);
    }
}
