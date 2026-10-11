use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshPreset {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PluginConfig {
    pub name: String,
    #[serde(default)]
    pub trusted: bool,
}

/// How the built-in editor treats tab characters (`editor_expand_tabs`).
///
/// Stored with the labels older releases wrote to `config.toml`; any other
/// value falls back to [`TabExpansion::Keep`] instead of failing the load.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum TabExpansion {
    /// Tab inserts spaces up to the next tab stop; existing tabs are kept.
    #[serde(rename = "Expand newly entered tabs to spaces")]
    NewTabs,
    /// Like `NewTabs`, and tabs in the file are converted when it is opened.
    #[serde(rename = "Convert all tabs to spaces")]
    ConvertAll,
    /// Tab inserts a tab character; existing tabs are kept. Must stay the
    /// last variant: `other` makes it the fallback for unknown values.
    #[default]
    #[serde(rename = "Do not expand tabs", other)]
    Keep,
}

impl TabExpansion {
    /// Next option in the configuration dialog cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Keep => Self::NewTabs,
            Self::NewTabs => Self::ConvertAll,
            Self::ConvertAll => Self::Keep,
        }
    }

    /// Translation key of the option label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Keep => "ed_expand_tabs_keep",
            Self::NewTabs => "ed_expand_tabs_new",
            Self::ConvertAll => "ed_expand_tabs_all",
        }
    }

    /// `true` when the Tab key inserts spaces.
    pub fn inserts_spaces(self) -> bool {
        self != Self::Keep
    }
}

/// When Pairee captures the mouse (`mouse`).
///
/// Also reads the boolean `mouse_support` written by older releases:
/// `true` → [`MouseMode::Auto`], `false` → [`MouseMode::Off`].
#[derive(Debug, Clone, Copy, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MouseMode {
    /// Capture when the terminal reports mouse events.
    #[default]
    Auto,
    /// Always capture, even where detection says the terminal cannot.
    On,
    /// Never capture: the terminal keeps its native text selection.
    Off,
}

impl<'de> Deserialize<'de> for MouseMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Flag(bool),
            Name(String),
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Flag(true) => Self::Auto,
            Raw::Flag(false) => Self::Off,
            Raw::Name(name) => match name.to_ascii_lowercase().as_str() {
                "on" => Self::On,
                "off" => Self::Off,
                _ => Self::Auto,
            },
        })
    }
}

impl MouseMode {
    /// Next option in the configuration dialog cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::On,
            Self::On => Self::Off,
            Self::Off => Self::Auto,
        }
    }

    /// Translation key of the option label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Auto => "int_mouse_auto",
            Self::On => "int_mouse_on",
            Self::Off => "int_mouse_off",
        }
    }

    /// Whether to capture, given what detection found about the terminal.
    pub fn wants_capture(self, terminal_reports_mouse: bool) -> bool {
        match self {
            Self::Auto => terminal_reports_mouse,
            Self::On => true,
            Self::Off => false,
        }
    }
}

/// What a right click on a panel row does (`mouse_right_click`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RightClick {
    /// Tags the entry, and dragging tags a range (Norton / Far).
    #[default]
    Mark,
    /// Opens the context menu.
    #[serde(other)]
    Menu,
}

impl RightClick {
    pub fn next(self) -> Self {
        match self {
            Self::Mark => Self::Menu,
            Self::Menu => Self::Mark,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Mark => "int_right_click_mark",
            Self::Menu => "int_right_click_menu",
        }
    }
}

/// Longest pause between the clicks of a double click (`mouse_double_click_ms`).
pub fn default_double_click_ms() -> u32 {
    400
}

pub fn default_true() -> bool {
    true
}

/// Encoding used by the viewer when automatic detection is off.
pub fn default_viewer_codepage() -> String {
    encoding_rs::UTF_8.name().to_string()
}

/// Seconds between folder checks when a panel folder is polled.
pub fn default_auto_refresh_poll_secs() -> u32 {
    3
}

/// Seconds between checks of an SFTP panel folder (network round trips).
pub fn default_auto_refresh_ssh_poll_secs() -> u32 {
    30
}

pub fn default_ssh_timeout_secs() -> u64 {
    crate::fs::ssh::DEFAULT_SSH_TIMEOUT_SECS
}

pub fn default_compare_mtime_tolerance_secs() -> u64 {
    crate::fs::compare::DEFAULT_MTIME_TOLERANCE_SECS
}

pub fn default_git_log_limit() -> u32 {
    100
}

pub fn default_plugins_dev_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            std::path::PathBuf::from(appdata)
                .join("pairee")
                .join("config")
                .join("plugins")
                .to_string_lossy()
                .into_owned()
        } else {
            "./config/plugins".to_string()
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        crate::config::paths::get_config_dir()
            .join("plugins")
            .to_string_lossy()
            .into_owned()
    }
}

pub fn default_transfer_hash() -> String {
    "blake3".to_string()
}
pub fn default_transfer_buffer() -> u32 {
    1024 * 1024
}
pub fn default_transfer_max_retries() -> u32 {
    3
}
pub fn default_transfer_conflict() -> String {
    "ask".to_string()
}
pub fn default_transfer_report_format() -> String {
    "html".to_string()
}
