//! The connection manager dialog: saved connections of every kind (SFTP,
//! FTP, FTPS, WebDAV, S3) on the left, the fields of the selected one on
//! the right, and Connect / Save / Delete / Cancel below.

use crate::app::state::types::ActivePanel;
use crate::app::text_input::TextField;
use crate::config::settings::{Connection, RemoteKind};

/// Rows of the form, in display order. `Kind` is a selector, the others
/// are text fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionField {
    Name,
    Kind,
    Host,
    Port,
    User,
    Password,
    KeyPath,
    Folder,
    Options,
}

/// Number of text fields (every row but `Kind`).
const TEXT_FIELDS: usize = 8;

impl ConnectionField {
    pub const ALL: [Self; 9] = [
        Self::Name,
        Self::Kind,
        Self::Host,
        Self::Port,
        Self::User,
        Self::Password,
        Self::KeyPath,
        Self::Folder,
        Self::Options,
    ];

    /// Label translation key; S3 names its host, user and password fields
    /// after what they hold.
    pub fn label_key(self, kind: RemoteKind) -> &'static str {
        let s3 = kind == RemoteKind::S3;
        match self {
            Self::Name => "prompt_conn_name",
            Self::Kind => "prompt_conn_kind",
            Self::Host if s3 => "prompt_conn_bucket",
            Self::Host => "prompt_ssh_host",
            Self::Port => "prompt_ssh_port",
            Self::User if s3 => "prompt_conn_access_key",
            Self::User => "prompt_ssh_user",
            Self::Password if s3 => "prompt_conn_secret_key",
            Self::Password => "prompt_ssh_pass",
            Self::KeyPath => "prompt_ssh_key_path",
            Self::Folder => "prompt_conn_folder",
            Self::Options => "prompt_conn_options",
        }
    }

    /// Dialog row of the field (row 0 is the connection list).
    pub fn row(self) -> usize {
        self as usize + 1
    }

    /// Index of the field in [`ConnectionPromptState::fields`] (`None`:
    /// the kind selector).
    fn text_index(self) -> Option<usize> {
        match self {
            Self::Name => Some(0),
            Self::Kind => None,
            other => Some(other as usize - 1),
        }
    }
}

/// Connection dialog: list, kind selector, eight fields and four buttons.
#[derive(Debug, Clone)]
pub struct ConnectionPromptState {
    pub panel: ActivePanel,
    pub fields: [TextField; TEXT_FIELDS],
    pub kind: RemoteKind,
    pub cursor_idx: usize,
    pub selected_idx: Option<usize>,
    /// The shown connection's password could not be read from the keyring:
    /// an empty password field means "keep it", not "none".
    secret_unread: bool,
}

impl ConnectionPromptState {
    pub const ROW_LIST: usize = 0;
    pub const LAST_FIELD: usize = ConnectionField::ALL.len();
    pub const BUTTON_CONNECT: usize = Self::LAST_FIELD + 1;
    pub const BUTTON_SAVE: usize = Self::LAST_FIELD + 2;
    pub const BUTTON_DELETE: usize = Self::LAST_FIELD + 3;
    pub const BUTTON_CANCEL: usize = Self::LAST_FIELD + 4;
    pub const ROW_COUNT: usize = Self::LAST_FIELD + 5;

    /// Dialog for `panel`, showing the first connection when there is one.
    pub fn new(panel: ActivePanel, saved: &[Connection]) -> Self {
        let mut prompt = Self {
            panel,
            fields: Default::default(),
            kind: RemoteKind::default(),
            cursor_idx: ConnectionField::Name.row(),
            selected_idx: None,
            secret_unread: false,
        };
        prompt.clear_fields();
        if let Some(first) = saved.first() {
            prompt.load(0, first);
            prompt.cursor_idx = Self::ROW_LIST;
        }
        prompt
    }

    /// The text field of `field` (`None`: the kind selector).
    pub fn text_field(&self, field: ConnectionField) -> Option<&TextField> {
        field.text_index().map(|idx| &self.fields[idx])
    }

    /// The text of `field` (empty for the kind selector).
    pub fn field(&self, field: ConnectionField) -> &str {
        self.text_field(field).map_or("", TextField::text)
    }

    /// The field on dialog row `row`: `Some((field, None))` for the kind
    /// selector, `Some((field, Some(text)))` for a text field.
    pub fn field_at(&mut self, row: usize) -> Option<(ConnectionField, Option<&mut TextField>)> {
        let field = *ConnectionField::ALL.get(row.checked_sub(1)?)?;
        Some((field, field.text_index().map(|idx| &mut self.fields[idx])))
    }

    /// The text field on dialog row `row`, if that row is one.
    pub fn text_field_at(&mut self, row: usize) -> Option<&mut TextField> {
        self.field_at(row).and_then(|(_, text)| text)
    }

    fn set(&mut self, field: ConnectionField, value: impl Into<String>) {
        if let Some(idx) = field.text_index() {
            self.fields[idx].set_text(value);
        }
    }

    /// Shows connection `idx` in the fields.
    pub fn load(&mut self, idx: usize, c: &Connection) {
        use ConnectionField as F;
        self.selected_idx = Some(idx);
        self.kind = c.kind;
        self.secret_unread = c.secret_unread;
        self.set(F::Name, c.name.clone());
        self.set(F::Host, c.host.clone());
        self.set(F::Port, c.port.clone());
        self.set(F::User, c.username.clone());
        self.set(F::Password, c.password.clone().unwrap_or_default());
        self.set(F::KeyPath, c.key_path.clone().unwrap_or_default());
        self.set(F::Folder, c.path.clone());
        self.set(F::Options, c.options.clone());
    }

    /// Empty fields for a new SFTP connection on the default port.
    pub fn clear_fields(&mut self) {
        self.selected_idx = None;
        self.secret_unread = false;
        self.kind = RemoteKind::default();
        for field in &mut self.fields {
            field.clear();
        }
        self.set_default_port();
    }

    fn set_default_port(&mut self) {
        let port = self.kind.default_port().map(|p| p.to_string());
        self.set(ConnectionField::Port, port.unwrap_or_default());
    }

    /// Switches to the next (`forward`) or previous kind. A port that was
    /// the old kind's default follows to the new kind's.
    pub fn cycle_kind(&mut self, forward: bool) {
        let typed = self.field(ConnectionField::Port).trim().to_string();
        let was_default =
            typed.is_empty() || self.kind.default_port().map(|p| p.to_string()) == Some(typed);
        self.kind = self.kind.cycle(forward);
        if was_default {
            self.set_default_port();
        }
    }

    /// The fields as a connection (trimmed; empty password / key mean none).
    pub fn to_connection(&self) -> Connection {
        use ConnectionField as F;
        let optional = |f: F| Some(self.field(f).to_string()).filter(|s| !s.is_empty());
        let trimmed = |f: F| self.field(f).trim().to_string();
        let password = optional(F::Password);
        Connection {
            name: trimmed(F::Name),
            kind: self.kind,
            host: trimmed(F::Host),
            port: trimmed(F::Port),
            username: trimmed(F::User),
            secret_unread: self.secret_unread && password.is_none(),
            password,
            key_path: optional(F::KeyPath),
            path: trimmed(F::Folder),
            options: trimmed(F::Options),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(name: &str) -> Connection {
        Connection {
            name: name.into(),
            host: "h".into(),
            port: "2222".into(),
            username: "u".into(),
            key_path: Some("k".into()),
            ..Connection::default()
        }
    }

    #[test]
    fn opens_on_first_connection_or_empty_form() {
        let empty = ConnectionPromptState::new(ActivePanel::Left, &[]);
        assert_eq!(empty.cursor_idx, ConnectionField::Name.row());
        assert_eq!(empty.field(ConnectionField::Port), "22");
        let p = ConnectionPromptState::new(ActivePanel::Left, &[connection("a")]);
        assert_eq!(p.cursor_idx, ConnectionPromptState::ROW_LIST);
        assert_eq!(p.field(ConnectionField::Port), "2222");
        assert_eq!(p.to_connection(), connection("a"));
    }

    #[test]
    fn field_rows_map_to_fields() {
        let mut p = ConnectionPromptState::new(ActivePanel::Right, &[]);
        assert!(p.field_at(0).is_none());
        assert!(p.field_at(ConnectionPromptState::BUTTON_CONNECT).is_none());
        let (field, text) = p.field_at(ConnectionField::Kind.row()).unwrap();
        assert_eq!((field, text.is_none()), (ConnectionField::Kind, true));
        let text = p.text_field_at(ConnectionField::Options.row()).unwrap();
        text.insert_char('x');
        assert_eq!(p.to_connection().options, "x");
        p.text_field_at(ConnectionField::User.row())
            .unwrap()
            .insert_char('y');
        assert_eq!(p.to_connection().username, "y");
    }

    #[test]
    fn changing_the_kind_moves_a_default_port_only() {
        let mut p = ConnectionPromptState::new(ActivePanel::Left, &[]);
        p.cycle_kind(true);
        assert_eq!(p.kind, RemoteKind::Ftp);
        assert_eq!(p.field(ConnectionField::Port), "21");
        p.cycle_kind(false);
        p.cycle_kind(false);
        assert_eq!(p.kind, RemoteKind::S3);
        assert_eq!(p.field(ConnectionField::Port), "");
        let mut typed = ConnectionPromptState::new(ActivePanel::Left, &[connection("a")]);
        typed.cycle_kind(true);
        assert_eq!(
            typed.field(ConnectionField::Port),
            "2222",
            "typed port kept"
        );
    }

    #[test]
    fn an_unread_password_stays_unknown_until_one_is_typed() {
        let mut saved = connection("a");
        saved.secret_unread = true;
        let mut p = ConnectionPromptState::new(ActivePanel::Left, &[saved]);
        assert!(p.to_connection().secret_unread);
        p.text_field_at(ConnectionField::Password.row())
            .unwrap()
            .insert_char('p');
        let typed = p.to_connection();
        assert!(!typed.secret_unread);
        assert_eq!(typed.password.as_deref(), Some("p"));
    }
}
