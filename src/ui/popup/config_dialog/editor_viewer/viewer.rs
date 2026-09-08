use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_viewer_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, RowType)>,
) {
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
    rows.push((t("vi_codepages_hint"), RowType::Hint));
}
