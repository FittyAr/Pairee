//! Saved remote connections (`[[connections]]` in `config.toml`): SFTP,
//! FTP, FTPS, WebDAV and S3. Older releases wrote SSH-only
//! `[[ssh_presets]]`; they are read as SFTP connections and written back as
//! `[[connections]]`. Passwords and secret keys live in the system keyring
//! (see `config::credentials`), never here unless the user opted in.

use serde::{Deserialize, Serialize};

/// The protocol of a saved connection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteKind {
    /// SSH File Transfer Protocol (the only kind older releases had).
    #[default]
    Sftp,
    Ftp,
    /// FTP with explicit TLS (`AUTH TLS`).
    Ftps,
    /// WebDAV over plain HTTP.
    WebDav,
    /// WebDAV over HTTPS.
    WebDavs,
    /// Amazon S3 and compatible object stores.
    S3,
}

impl RemoteKind {
    /// Every kind, in the order the connection dialog cycles through them.
    pub const ALL: [Self; 6] = [
        Self::Sftp,
        Self::Ftp,
        Self::Ftps,
        Self::WebDav,
        Self::WebDavs,
        Self::S3,
    ];

    /// The port a server of this kind listens on by default (`None`: the
    /// endpoint decides, S3).
    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::Sftp => Some(22),
            Self::Ftp | Self::Ftps => Some(21),
            Self::WebDav => Some(80),
            Self::WebDavs => Some(443),
            Self::S3 => None,
        }
    }

    /// Translation key of the protocol name.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Sftp => "remote_kind_sftp",
            Self::Ftp => "remote_kind_ftp",
            Self::Ftps => "remote_kind_ftps",
            Self::WebDav => "remote_kind_webdav",
            Self::WebDavs => "remote_kind_webdavs",
            Self::S3 => "remote_kind_s3",
        }
    }

    /// URL scheme of the backend (see `fs::remote::connect`).
    pub fn scheme(self) -> &'static str {
        match self {
            Self::Sftp => "sftp",
            Self::Ftp => "ftp",
            Self::Ftps => "ftps",
            Self::WebDav => "webdav",
            Self::WebDavs => "webdavs",
            Self::S3 => "s3",
        }
    }

    /// The next (`forward`) or previous kind, wrapping around.
    pub fn cycle(self, forward: bool) -> Self {
        let len = Self::ALL.len();
        let idx = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        let next = if forward {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        };
        Self::ALL[next]
    }
}

/// One saved connection.
///
/// For S3, `host` is the bucket, `username` / `password` the access key and
/// secret key, `path` the key prefix and `options` the endpoint settings
/// (`endpoint=https://…&region=…&path_style=true`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Connection {
    pub name: String,
    /// Older `[[ssh_presets]]` entries have no kind: SFTP.
    #[serde(default)]
    pub kind: RemoteKind,
    pub host: String,
    /// Empty: the kind's default port.
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub username: String,
    /// The password (or S3 secret key). Kept in the keyring: only present
    /// in `config.toml` with `[security] passwords_in_config`.
    #[serde(default)]
    pub password: Option<String>,
    /// SSH private key file (SFTP only).
    #[serde(default)]
    pub key_path: Option<String>,
    /// Folder opened after connecting (empty: the server's root or home).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Extra settings of the kind (S3 endpoint, region and URL style).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub options: String,
    /// The keyring could not be read when the settings were loaded, so an
    /// empty `password` means "unknown", not "none": saving must keep the
    /// keyring entry instead of deleting it.
    #[serde(skip)]
    pub secret_unread: bool,
}

impl Connection {
    /// The port to connect to: the typed one, else the kind's default.
    pub fn port_number(&self) -> Option<u16> {
        self.port
            .trim()
            .parse()
            .ok()
            .or_else(|| self.kind.default_port())
    }

    /// `true` when the options turn certificate checks off (FTPS and
    /// WebDAV over HTTPS: `insecure=true`).
    pub fn insecure_tls(&self) -> bool {
        matches!(self.kind, RemoteKind::Ftps | RemoteKind::WebDavs)
            && self
                .options
                .trim_start_matches('?')
                .split('&')
                .any(|pair| matches!(pair, "insecure=true" | "insecure=1" | "insecure=yes"))
    }

    /// `true` when this connection reaches the server `kind://user@host:port`.
    pub fn matches(&self, kind: RemoteKind, host: &str, port: Option<u16>, user: &str) -> bool {
        self.kind == kind
            && self.host == host
            && self.username == user
            && self.port_number() == port
    }
}
