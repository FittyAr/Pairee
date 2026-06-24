use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

/// Populates the rows for the Git configuration tab.
/// Setting IDs are in the range 800–819 to avoid clashing with other tabs.
pub fn populate_rows(
    settings: &Settings,
    editing: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, RowType)>,
) {
    rows.push((t("git_section_general"), RowType::Title));

    // 800: git_enabled
    rows.push((
        format!(
            "[{}] {}",
            if settings.git_enabled { "x" } else { " " },
            t("git_enabled")
        ),
        RowType::Setting(800),
    ));

    // 801: git_auto_detect
    rows.push((
        format!(
            "[{}] {}",
            if settings.git_auto_detect { "x" } else { " " },
            t("git_auto_detect")
        ),
        RowType::Setting(801),
    ));

    rows.push((t("git_section_author"), RowType::Title));

    // 802: git_author_name (text input)
    let name_display = if editing && cursor_idx == rows.len() {
        format!("{}: {}█", t("git_author_name"), edit_buffer)
    } else {
        format!(
            "{}: {}",
            t("git_author_name"),
            if settings.git_author_name.is_empty() {
                format!("({})", t("git_from_git_config"))
            } else {
                settings.git_author_name.clone()
            }
        )
    };
    rows.push((name_display, RowType::Setting(802)));

    // 803: git_author_email (text input)
    let email_display = if editing && cursor_idx == rows.len() {
        format!("{}: {}█", t("git_author_email"), edit_buffer)
    } else {
        format!(
            "{}: {}",
            t("git_author_email"),
            if settings.git_author_email.is_empty() {
                format!("({})", t("git_from_git_config"))
            } else {
                settings.git_author_email.clone()
            }
        )
    };
    rows.push((email_display, RowType::Setting(803)));

    rows.push((t("git_section_log"), RowType::Title));

    // 804: git_log_limit (numeric input)
    let limit_display = if editing && cursor_idx == rows.len() {
        format!("{}: {}█", t("git_log_limit"), edit_buffer)
    } else {
        format!("{}: {}", t("git_log_limit"), settings.git_log_limit)
    };
    rows.push((limit_display, RowType::Setting(804)));

    rows.push((t("git_hint_config"), RowType::Hint));
}
