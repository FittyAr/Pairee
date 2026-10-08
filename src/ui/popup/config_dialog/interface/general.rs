use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_general_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("General".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_clock { "x" } else { " " },
            t("int_clock")
        ),
        RowType::Setting(0),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.mouse_support { "x" } else { " " },
            t("int_mouse")
        ),
        RowType::Setting(1),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_show_key_bar {
                "x"
            } else {
                " "
            },
            t("int_key_bar")
        ),
        RowType::Setting(2),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_always_show_menu_bar {
                "x"
            } else {
                " "
            },
            t("int_menu_bar")
        ),
        RowType::Setting(3),
    ));
}
