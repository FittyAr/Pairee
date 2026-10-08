use crate::app::state::types::ActivePanel;
use crate::app::text_input::TextField;
use crate::config::settings::SshPreset;

/// Text fields of the SSH dialog, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshField {
    Name,
    Host,
    Port,
    User,
    Password,
    KeyPath,
}

impl SshField {
    pub const ALL: [Self; 6] = [
        Self::Name,
        Self::Host,
        Self::Port,
        Self::User,
        Self::Password,
        Self::KeyPath,
    ];

    /// Label translation key.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Name => "prompt_ssh_name",
            Self::Host => "prompt_ssh_host",
            Self::Port => "prompt_ssh_port",
            Self::User => "prompt_ssh_user",
            Self::Password => "prompt_ssh_pass",
            Self::KeyPath => "prompt_ssh_key_path",
        }
    }

    /// Dialog row of the field (row 0 is the preset list).
    pub fn row(self) -> usize {
        self as usize + 1
    }
}

/// SSH connection dialog: preset list, six fields and four buttons.
#[derive(Debug, Clone)]
pub struct SshConnectPromptState {
    pub panel: ActivePanel,
    pub fields: [TextField; 6],
    pub cursor_idx: usize,
    pub selected_preset_idx: Option<usize>,
}

impl SshConnectPromptState {
    pub const ROW_PRESETS: usize = 0;
    pub const BUTTON_CONNECT: usize = 7;
    pub const BUTTON_SAVE: usize = 8;
    pub const BUTTON_DELETE: usize = 9;
    pub const BUTTON_CANCEL: usize = 10;
    pub const ROW_COUNT: usize = 11;

    /// Dialog for `panel`, showing the first preset when there is one.
    pub fn new(panel: ActivePanel, presets: &[SshPreset]) -> Self {
        let mut prompt = Self {
            panel,
            fields: Default::default(),
            cursor_idx: SshField::Name.row(),
            selected_preset_idx: None,
        };
        prompt.clear_fields();
        if let Some(first) = presets.first() {
            prompt.load_preset(0, first);
            prompt.cursor_idx = Self::ROW_PRESETS;
        }
        prompt
    }

    pub fn field(&self, field: SshField) -> &str {
        self.fields[field as usize].text()
    }

    /// The field on dialog row `row`, if that row is a field.
    pub fn field_at(&mut self, row: usize) -> Option<(SshField, &mut TextField)> {
        let field = *SshField::ALL.get(row.checked_sub(1)?)?;
        Some((field, &mut self.fields[field as usize]))
    }

    /// Shows preset `idx` in the fields.
    pub fn load_preset(&mut self, idx: usize, p: &SshPreset) {
        self.selected_preset_idx = Some(idx);
        let values = [
            p.name.clone(),
            p.host.clone(),
            p.port.clone(),
            p.username.clone(),
            p.password.clone().unwrap_or_default(),
            p.key_path.clone().unwrap_or_default(),
        ];
        for (field, value) in self.fields.iter_mut().zip(values) {
            field.set_text(value);
        }
    }

    /// Empty fields with the default port.
    pub fn clear_fields(&mut self) {
        self.selected_preset_idx = None;
        for field in &mut self.fields {
            field.clear();
        }
        self.fields[SshField::Port as usize].set_text("22");
    }

    /// The fields as a preset (trimmed; empty password / key mean none).
    pub fn to_preset(&self) -> SshPreset {
        let optional = |f: SshField| Some(self.field(f).to_string()).filter(|s| !s.is_empty());
        SshPreset {
            name: self.field(SshField::Name).trim().to_string(),
            host: self.field(SshField::Host).trim().to_string(),
            port: self.field(SshField::Port).trim().to_string(),
            username: self.field(SshField::User).trim().to_string(),
            password: optional(SshField::Password),
            key_path: optional(SshField::KeyPath),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str) -> SshPreset {
        SshPreset {
            name: name.into(),
            host: "h".into(),
            port: "2222".into(),
            username: "u".into(),
            password: None,
            key_path: Some("k".into()),
        }
    }

    #[test]
    fn opens_on_first_preset_or_empty_form() {
        let empty = SshConnectPromptState::new(ActivePanel::Left, &[]);
        assert_eq!(empty.cursor_idx, SshField::Name.row());
        assert_eq!(empty.field(SshField::Port), "22");
        let p = SshConnectPromptState::new(ActivePanel::Left, &[preset("a")]);
        assert_eq!(p.cursor_idx, SshConnectPromptState::ROW_PRESETS);
        assert_eq!(p.field(SshField::Port), "2222");
        assert_eq!(p.to_preset(), preset("a"));
    }

    #[test]
    fn field_rows_map_to_fields() {
        let mut p = SshConnectPromptState::new(ActivePanel::Right, &[]);
        assert!(p.field_at(0).is_none());
        assert!(p.field_at(SshConnectPromptState::BUTTON_CONNECT).is_none());
        let (field, text) = p.field_at(SshField::User.row()).unwrap();
        assert_eq!(field, SshField::User);
        text.insert_char('x');
        assert_eq!(p.to_preset().username, "x");
    }
}
