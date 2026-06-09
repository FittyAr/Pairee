use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, bool)>) {
    rows.push((
        format!(
            "{}: < {} >",
            t("lang_label"),
            settings.language
        ),
        false,
    ));
    rows.push((
        format!(
            "{}: [ArcLite | EMenu | HlfViewer | NetBox]",
            t("plugins_config")
        ),
        false,
    ));
    rows.push((
        t("plugins_manager_settings"),
        false,
    ));
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
        false,
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
        false,
    ));
    rows.push((
        format!("  {}", t("plugin_selection")),
        false,
    ));
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
    ));
}
