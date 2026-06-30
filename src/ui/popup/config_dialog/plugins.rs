use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("Language".to_string(), RowType::Title));
    rows.push((
        format!("{}: < {} >", t("lang_label"), settings.language),
        RowType::Setting(0),
    ));

    rows.push(("Plugins Configuration".to_string(), RowType::Title));
    rows.push((
        format!(
            "{}: [ArcLite | EMenu | HlfViewer | NetBox]",
            t("plugins_config")
        ),
        RowType::Hint,
    )); // 1

    rows.push((t("plugins_manager_settings"), RowType::Title)); // 2
    rows.push((
        format!(
            "  [{}] {}",
            if settings.plugins_manager_oem_support {
                "x"
            } else {
                " "
            },
            t("oem_support")
        ),
        RowType::Setting(3),
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.plugins_manager_scan_symlinks {
                "x"
            } else {
                " "
            },
            t("scan_symlinks")
        ),
        RowType::Setting(4),
    ));

    rows.push((format!("  {}", t("plugin_selection")), RowType::Subtitle)); // 5
    rows.push((
        format!(
            "    [{}] {}",
            if settings.plugins_manager_file_processing {
                "x"
            } else {
                " "
            },
            t("file_processing")
        ),
        RowType::Setting(6),
    ));
    rows.push((
        format!(
            "      [{}] {}",
            if settings.plugins_manager_show_standard_association {
                "x"
            } else {
                " "
            },
            t("show_std_association")
        ),
        RowType::Setting(7),
    ));
    rows.push((
        format!(
            "        [{}] {}",
            if settings.plugins_manager_even_if_one_found {
                "x"
            } else {
                " "
            },
            t("even_if_one")
        ),
        RowType::Setting(8),
    ));
    rows.push((
        format!(
            "    [{}] {}",
            if settings.plugins_manager_search_results {
                "x"
            } else {
                " "
            },
            t("search_results")
        ),
        RowType::Setting(9),
    ));
    rows.push((
        format!(
            "    [{}] {}",
            if settings.plugins_manager_prefix_processing {
                "x"
            } else {
                " "
            },
            t("prefix_processing")
        ),
        RowType::Setting(10),
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.plugins_developer_mode {
                "x"
            } else {
                " "
            },
            t("developer_mode")
        ),
        RowType::Setting(11),
    ));
}
