use crate::config::settings::Settings;
use crate::config::localization::t;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, bool)>) {
    rows.push((format!("{}: < {} >", t("col_theme"), settings.theme), false));
    rows.push((
        t("col_groups"),
        true,
    ));
    rows.push((
        t("col_highlighting"),
        true,
    ));
}
