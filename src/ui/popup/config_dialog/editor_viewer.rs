use crate::config::settings::Settings;
use crate::config::localization::t;

pub fn populate_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, bool)>,
) {
    rows.push((
        format!(
            "[{}] {}",
            if settings.editor_use_external {
                "x"
            } else {
                " "
            },
            t("ed_external")
        ),
        true,
    ));
    if editing_value && cursor_idx == 1 {
        rows.push((format!("{} [ {}◄ ]", t("ed_command"), edit_buffer), false));
    } else {
        rows.push((
            format!("{} [ {} ]", t("ed_command"), settings.default_editor),
            false,
        ));
    }
    rows.push((t("ed_internal_title"), true));
    rows.push((
        format!("{} < {} >", t("ed_expand_tabs"), settings.editor_expand_tabs),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_persistent_blocks {
                "x"
            } else {
                " "
            },
            t("ed_persistent_blocks")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_cursor_beyond_eol {
                "x"
            } else {
                " "
            },
            t("ed_cursor_beyond_eol")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_del_removes_blocks {
                "x"
            } else {
                " "
            },
            t("ed_del_removes_blocks")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_select_found {
                "x"
            } else {
                " "
            },
            t("ed_select_found")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_auto_indent {
                "x"
            } else {
                " "
            },
            t("ed_auto_indent")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_cursor_at_end {
                "x"
            } else {
                " "
            },
            t("ed_cursor_at_end")
        ),
        true,
    ));
    rows.push((
        format!("  {} [ {} ]", t("ed_tab_size"), settings.editor_tab_size),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_show_scrollbar {
                "x"
            } else {
                " "
            },
            t("ed_show_scrollbar")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_show_white_space {
                "x"
            } else {
                " "
            },
            t("ed_show_white_space")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_show_line_numbers {
                "x"
            } else {
                " "
            },
            t("ed_show_line_numbers")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_save_file_position {
                "x"
            } else {
                " "
            },
            t("ed_save_pos")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_save_bookmarks {
                "x"
            } else {
                " "
            },
            t("ed_save_bookmarks")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_allow_editing_opened_writing {
                "x"
            } else {
                " "
            },
            t("ed_allow_opened_writing")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_lock_editing_readonly {
                "x"
            } else {
                " "
            },
            t("ed_lock_readonly")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_warn_opening_readonly {
                "x"
            } else {
                " "
            },
            t("ed_warn_readonly")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.editor_autodetect_codepage {
                "x"
            } else {
                " "
            },
            t("ed_detect_codepage")
        ),
        true,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("ed_default_codepage"),
            settings.editor_default_codepage
        ),
        true,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.viewer_use_external {
                "x"
            } else {
                " "
            },
            t("vi_external")
        ),
        true,
    ));
    if editing_value && cursor_idx == 22 {
        rows.push((format!("{} [ {}◄ ]", t("vi_command"), edit_buffer), false));
    } else {
        rows.push((
            format!("{} [ {} ]", t("vi_command"), settings.viewer_command),
            true,
        ));
    }
    rows.push((t("vi_internal_title"), true));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_persistent_selection {
                "x"
            } else {
                " "
            },
            t("vi_persistent_selection")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_show_scrolling_arrows {
                "x"
            } else {
                " "
            },
            t("vi_scrolling_arrows")
        ),
        true,
    ));
    rows.push((
        format!("  {} [ {} ]", t("vi_tab_size"), settings.viewer_tab_size),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_visible_zero {
                "x"
            } else {
                " "
            },
            t("vi_visible_zero")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_show_scrollbar {
                "x"
            } else {
                " "
            },
            t("vi_show_scrollbar")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_save_file_position {
                "x"
            } else {
                " "
            },
            t("vi_save_pos")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_save_view_mode {
                "x"
            } else {
                " "
            },
            t("vi_save_mode")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_save_file_codepage {
                "x"
            } else {
                " "
            },
            t("vi_save_codepage")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_save_wrap_mode {
                "x"
            } else {
                " "
            },
            t("vi_save_wrap")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_save_bookmarks {
                "x"
            } else {
                " "
            },
            t("vi_save_bookmarks")
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_detect_dump_view_mode {
                "x"
            } else {
                " "
            },
            t("vi_detect_dump")
        ),
        true,
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("vi_max_line"),
            settings.viewer_max_line_width
        ),
        true,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_autodetect_codepage {
                "x"
            } else {
                " "
            },
            t("vi_detect_codepage")
        ),
        true,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("vi_default_codepage"),
            settings.viewer_default_codepage
        ),
        true,
    ));
    rows.push((t("vi_codepages_hint"), true));
}
