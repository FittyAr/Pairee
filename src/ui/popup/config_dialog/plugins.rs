use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("Language".to_string(), RowType::Title));
    rows.push((
        format!("{}: < {} >", t("lang_label"), settings.language),
        RowType::Setting(0),
    ));

    rows.push((t("plugins_manager_settings"), RowType::Title)); // 2
    rows.push((format!("  {}", t("plugin_selection")), RowType::Subtitle)); // 5
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
    if settings.plugins_developer_mode {
        rows.push((
            format!("    Path: {}", settings.plugins_dev_dir),
            RowType::Setting(12),
        ));
    }
}
