//! The settings form of one plugin (G11), pushed over the Plugin Manager:
//! one row per `[settings_schema]` entry (a checkbox for `bool`, a text
//! field otherwise), then Save / Defaults / Cancel.

use crate::app::form::FormLayout;
use crate::app::text_input::TextField;
use crate::plugin::runtime::settings::{Kind, SettingSpec};
use std::collections::HashMap;

/// Value being edited in one row.
#[derive(Debug, Clone)]
pub enum PluginSettingValue {
    Toggle(bool),
    Text(TextField),
}

#[derive(Debug, Clone)]
pub struct PluginSettingRow {
    pub spec: SettingSpec,
    pub value: PluginSettingValue,
}

impl PluginSettingRow {
    fn new(spec: SettingSpec, current: Option<&String>) -> Self {
        let text = current
            .or(spec.default.as_ref())
            .cloned()
            .unwrap_or_default();
        let value = match spec.kind {
            Kind::Bool => PluginSettingValue::Toggle(text.trim() == "true"),
            _ => PluginSettingValue::Text(TextField::new(text)),
        };
        Self { spec, value }
    }

    /// The row's value as the text stored in `config.toml`.
    pub fn raw(&self) -> String {
        match &self.value {
            PluginSettingValue::Toggle(on) => on.to_string(),
            PluginSettingValue::Text(field) => field.text().trim().to_string(),
        }
    }
}

/// Buttons after the rows.
pub const SETTINGS_BUTTONS: usize = 3;
pub const BUTTON_SAVE: usize = 0;
pub const BUTTON_DEFAULTS: usize = 1;

#[derive(Debug, Clone)]
pub struct PluginSettingsDialog {
    pub plugin: String,
    pub rows: Vec<PluginSettingRow>,
    pub focus: usize,
    /// Why the last Save was refused (row index, message).
    pub error: Option<(usize, String)>,
}

impl PluginSettingsDialog {
    /// The form for `plugin` with `specs`, showing the user's `current`
    /// values (or the defaults).
    pub fn new(plugin: &str, specs: Vec<SettingSpec>, current: &HashMap<String, String>) -> Self {
        let rows = specs
            .into_iter()
            .map(|spec| {
                let value = current.get(&spec.key);
                PluginSettingRow::new(spec, value)
            })
            .collect();
        Self {
            plugin: plugin.to_string(),
            rows,
            focus: 0,
            error: None,
        }
    }

    pub fn layout(&self) -> FormLayout {
        FormLayout::new(self.rows.len() + SETTINGS_BUTTONS, self.rows.len())
    }

    /// Text field of the focused row, if it is one.
    pub fn focused_field(&mut self) -> Option<&mut TextField> {
        match &mut self.rows.get_mut(self.focus)?.value {
            PluginSettingValue::Text(field) => Some(field),
            PluginSettingValue::Toggle(_) => None,
        }
    }

    /// Flips the checkbox of row `row` (no-op on text rows).
    pub fn toggle(&mut self, row: usize) {
        if let Some(PluginSettingValue::Toggle(on)) = self.rows.get_mut(row).map(|r| &mut r.value) {
            *on = !*on;
        }
    }

    /// Puts every row back to its schema default.
    pub fn reset(&mut self) {
        for row in &mut self.rows {
            *row = PluginSettingRow::new(row.spec.clone(), None);
        }
        self.error = None;
    }

    /// The values to save: those that differ from their default. The
    /// first row whose text is not valid for its type is reported instead.
    pub fn values(&self) -> Result<HashMap<String, String>, usize> {
        let mut values = HashMap::new();
        for (i, row) in self.rows.iter().enumerate() {
            let raw = row.raw();
            if !row.spec.kind.accepts(&raw) {
                return Err(i);
            }
            if row.spec.default.as_deref().map(str::trim) != Some(raw.as_str()) {
                values.insert(row.spec.key.clone(), raw);
            }
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(key: &str, kind: Kind, default: &str) -> SettingSpec {
        SettingSpec {
            key: key.into(),
            kind,
            default: Some(default.into()),
            description: None,
        }
    }

    fn dialog(current: &[(&str, &str)]) -> PluginSettingsDialog {
        let current: HashMap<String, String> = current
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        PluginSettingsDialog::new(
            "demo",
            vec![
                spec("depth", Kind::Integer, "3"),
                spec("flag", Kind::Bool, "false"),
            ],
            &current,
        )
    }

    #[test]
    fn rows_show_user_values_and_only_changes_are_saved() {
        let mut d = dialog(&[("depth", "5")]);
        assert_eq!(d.rows[0].raw(), "5");
        assert_eq!(d.rows[1].raw(), "false");
        d.toggle(1);
        let values = d.values().unwrap();
        assert_eq!(values.get("depth").map(String::as_str), Some("5"));
        assert_eq!(values.get("flag").map(String::as_str), Some("true"));
        d.reset();
        assert!(d.values().unwrap().is_empty());
        assert_eq!(d.layout().rows, 5);
    }

    #[test]
    fn invalid_values_name_their_row() {
        let mut d = dialog(&[]);
        *d.focused_field().unwrap() = TextField::new("deep");
        assert_eq!(d.values(), Err(0));
        d.focus = 1;
        assert!(d.focused_field().is_none());
    }
}
