use super::RowType;
use crate::config::settings::Settings;
use std::collections::HashMap;

mod advanced;
mod general;

pub fn populate_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, RowType)>,
    custom_bindings: &HashMap<String, String>,
) {
    general::populate_general_rows(settings, editing_value, cursor_idx, edit_buffer, rows);
    advanced::populate_advanced_rows(settings, rows, custom_bindings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Settings;
    use std::collections::HashMap;

    #[test]
    fn keymap_section_always_has_status_and_view_rows() {
        let settings = Settings::default();
        let mut rows = Vec::new();
        populate_rows(&settings, false, 0, "", &mut rows, &HashMap::new());
        assert!(
            rows.iter()
                .any(|(_, kind)| matches!(kind, RowType::Setting(36))),
            "preset row"
        );
        assert!(
            rows.iter()
                .any(|(_, kind)| matches!(kind, RowType::Setting(38))),
            "view issues row"
        );
        assert!(
            rows.iter().any(
                |(label, kind)| matches!(kind, RowType::Hint | RowType::Subtitle)
                    && (label.contains("OK")
                        || label.contains("error")
                        || label.contains("errores")
                        || label.contains("Keymap")
                        || label.contains("Mapa"))
            ),
            "status line present: {:?}",
            rows
        );
    }
}
