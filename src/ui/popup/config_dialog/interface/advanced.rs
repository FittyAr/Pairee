use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;
use crate::keybindings::loader::load_keybinds;
use std::collections::HashMap;

pub fn populate_advanced_rows(
    settings: &Settings,
    rows: &mut Vec<(String, RowType)>,
    custom_bindings: &HashMap<String, String>,
) {
    // Keybindings
    rows.push(("Keybindings".to_string(), RowType::Title));
    rows.push((
        format!(
            "{} < {} >",
            t("int_keybindings"),
            settings.keybinding_preset
        ),
        RowType::Setting(36),
    ));
    let (_, keymap_report) = load_keybinds(&settings.keybinding_preset, custom_bindings);
    let status_row = if keymap_report.ok() && keymap_report.warnings.is_empty() {
        RowType::Hint
    } else {
        RowType::Subtitle
    };
    rows.push((keymap_report.summary_line(), status_row));
    rows.push((t("int_keymap_gray"), RowType::Hint));
    rows.push((t("int_keymap_view"), RowType::Setting(38)));
    rows.push((
        format!(
            "[{}] {}",
            if settings.enable_yazi_workflow {
                "x"
            } else {
                " "
            },
            t("int_yazi_workflow")
        ),
        RowType::Setting(37),
    ));
}
