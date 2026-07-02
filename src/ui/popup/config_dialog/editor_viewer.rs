use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, RowType)>,
) {
    rows.push(("Editor Settings".to_string(), RowType::Title));
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
        RowType::Setting(0),
    ));

    let is_editing_editor = editing_value && cursor_idx == rows.len();
    if is_editing_editor {
        rows.push((
            format!("{} [ {}◄ ]", t("ed_command"), edit_buffer),
            RowType::Setting(1),
        ));
    } else {
        rows.push((
            format!("{} [ {} ]", t("ed_command"), settings.default_editor),
            RowType::Setting(1),
        ));
    }

    // t("ed_internal_title") was index 2
    rows.push((t("ed_internal_title"), RowType::Subtitle));
    rows.push((
        format!(
            "{} < {} >",
            t("ed_expand_tabs"),
            settings.editor_expand_tabs
        ),
        RowType::Setting(3),
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
        RowType::Setting(4),
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
        RowType::Setting(5),
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
        RowType::Setting(6),
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
        RowType::Setting(7),
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
        RowType::Setting(8),
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
        RowType::Setting(9),
    ));
    rows.push((
        format!("  {} [ {} ]", t("ed_tab_size"), settings.editor_tab_size),
        RowType::Setting(10),
    ));

    rows.push(("Editor Appearance & Saving".to_string(), RowType::Title));
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
        RowType::Setting(11),
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
        RowType::Setting(12),
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
        RowType::Setting(13),
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
        RowType::Setting(14),
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
        RowType::Setting(15),
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
        RowType::Setting(16),
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
        RowType::Setting(17),
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
        RowType::Setting(18),
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
        RowType::Setting(19),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("ed_default_codepage"),
            settings.editor_default_codepage
        ),
        RowType::Setting(20),
    ));

    rows.push(("Viewer Settings".to_string(), RowType::Title));
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
        RowType::Setting(21),
    ));

    rows.push((
        format!(
            "  [{}] {}",
            if settings.enter_use_external {
                "x"
            } else {
                " "
            },
            t("vi_enter_external")
        ),
        RowType::Setting(38),
    ));

    let is_editing_viewer = editing_value && cursor_idx == rows.len();
    if is_editing_viewer {
        rows.push((
            format!("{} [ {}◄ ]", t("vi_command"), edit_buffer),
            RowType::Setting(22),
        ));
    } else {
        rows.push((
            format!("{} [ {} ]", t("vi_command"), settings.viewer_command),
            RowType::Setting(22),
        ));
    }

    // t("vi_internal_title") was index 23
    rows.push((t("vi_internal_title"), RowType::Subtitle));
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
        RowType::Setting(24),
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
        RowType::Setting(25),
    ));
    rows.push((
        format!("  {} [ {} ]", t("vi_tab_size"), settings.viewer_tab_size),
        RowType::Setting(26),
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
        RowType::Setting(27),
    ));

    rows.push(("Viewer Appearance & Saving".to_string(), RowType::Title));
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
        RowType::Setting(28),
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
        RowType::Setting(29),
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
        RowType::Setting(30),
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
        RowType::Setting(31),
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
        RowType::Setting(32),
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
        RowType::Setting(33),
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
        RowType::Setting(34),
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("vi_max_line"),
            settings.viewer_max_line_width
        ),
        RowType::Setting(35),
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
        RowType::Setting(36),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("vi_default_codepage"),
            settings.viewer_default_codepage
        ),
        RowType::Setting(37),
    ));
    rows.push((t("vi_codepages_hint"), RowType::Hint)); // 38
}
