//! Rows of the configuration dialog, described as data (ratatui-free).
//!
//! Each tab is a list of [`Row`]s; a [`Setting`] row carries how its value
//! is shown and changed ([`Kind`]), so the renderer and the key handler both
//! work from the same table instead of keeping matching `id`s in two places.

mod tabs;
mod tabs_extra;

use crate::app::context::AppContext;
use crate::app::state::ConfigDraft;
use crate::app::state::PopupType;
use crate::app::text_input::TextField;
use crate::config::localization::t;

/// Translation keys of the tab titles (with their `&` hotkeys), in order.
pub const TAB_KEYS: [&str; 8] = [
    "tab_system",
    "tab_panel",
    "tab_interface",
    "tab_confirmations",
    "tab_plugins",
    "tab_editor",
    "tab_colors",
    "tab_git",
];

/// Row text: a translation key, fixed text, or text built at run time.
#[derive(Debug, Clone)]
pub enum Label {
    Key(&'static str),
    Raw(&'static str),
    Owned(String),
}

impl Label {
    pub fn text(&self) -> String {
        match self {
            Self::Key(key) => t(key),
            Self::Raw(text) => (*text).to_string(),
            Self::Owned(text) => text.clone(),
        }
    }
}

/// How a cycled value is shown next to its label.
#[derive(Debug, Clone, Copy)]
pub enum CycleFormat {
    /// `label < value >`
    Angle,
    /// `label: < value >`
    ColonAngle,
    /// `label [ value ]`
    Bracket,
}

/// What a setting row shows and does on Enter / Space.
#[derive(Clone, Copy)]
pub enum Kind {
    /// `[x] label`; flips the flag.
    Toggle {
        get: fn(&ConfigDraft) -> bool,
        set: fn(&mut ConfigDraft, bool),
    },
    /// Label and current value; advances to the next value.
    Cycle {
        show: fn(&ConfigDraft) -> String,
        next: fn(&mut ConfigDraft),
        format: CycleFormat,
    },
    /// `label: value`; Enter edits the text in place.
    Edit {
        show: fn(&ConfigDraft) -> String,
        get: fn(&ConfigDraft) -> String,
        set: fn(&mut ConfigDraft, &str),
    },
    /// Plain label; opens another dialog or panel.
    Action(fn(&ConfigDraft, &AppContext) -> Option<PopupType>),
}

#[derive(Clone)]
pub struct Setting {
    pub label: Label,
    /// Indentation level (two spaces each).
    pub indent: usize,
    pub kind: Kind,
}

impl Setting {
    /// Display text; `edit` is the in-progress text of an edited row.
    pub fn text(&self, settings: &ConfigDraft, edit: Option<&TextField>) -> String {
        let pad = "  ".repeat(self.indent);
        let label = self.label.text();
        let body = match (self.kind, edit) {
            (Kind::Edit { .. }, Some(field)) => format!("{}: {}█", label, field.text()),
            (Kind::Edit { show, .. }, None) => format!("{}: {}", label, show(settings)),
            (Kind::Toggle { get, .. }, _) => {
                format!("[{}] {}", if get(settings) { "x" } else { " " }, label)
            }
            (Kind::Cycle { show, format, .. }, _) => {
                let value = show(settings);
                match format {
                    CycleFormat::Angle => format!("{} < {} >", label, value),
                    CycleFormat::ColonAngle => format!("{}: < {} >", label, value),
                    CycleFormat::Bracket => format!("{} [ {} ]", label, value),
                }
            }
            (Kind::Action(_), _) => label,
        };
        format!("{pad}{body}")
    }
}

#[derive(Clone)]
pub enum Row {
    Title(Label),
    Subtitle(Label),
    Hint(Label),
    Setting(Setting),
}

impl Row {
    pub fn is_selectable(&self) -> bool {
        matches!(self, Self::Setting(_))
    }

    pub fn setting(&self) -> Option<&Setting> {
        match self {
            Self::Setting(s) => Some(s),
            _ => None,
        }
    }
}

/// Inputs some tabs need besides the settings being edited.
pub struct RowCtx<'a> {
    pub settings: &'a ConfigDraft,
    pub keybindings: &'a crate::config::keybindings::KeybindingsConfig,
}

/// The rows of tab `tab` (without the OK / Cancel buttons).
pub fn tab_rows(tab: usize, ctx: &RowCtx) -> Vec<Row> {
    match tab {
        0 => tabs::system(),
        1 => tabs::panel(),
        2 => tabs_extra::interface(ctx),
        3 => tabs::confirmations(),
        4 => tabs_extra::plugins(ctx.settings),
        5 => tabs_extra::editor_viewer(),
        6 => tabs_extra::colors(),
        7 => tabs_extra::git(),
        _ => Vec::new(),
    }
}

/// What activating a setting row asks the dialog to do.
pub enum Activation {
    /// The value changed in place.
    Changed,
    /// Start editing with this text.
    StartEdit(TextField),
    /// Show this popup.
    Open(Box<PopupType>),
}

impl Setting {
    /// Enter / Space on the row.
    pub fn activate(&self, settings: &mut ConfigDraft, context: &AppContext) -> Activation {
        match self.kind {
            Kind::Toggle { get, set } => {
                let value = get(settings);
                set(settings, !value);
            }
            Kind::Cycle { next, .. } => next(settings),
            Kind::Edit { get, .. } => return Activation::StartEdit(TextField::new(get(settings))),
            Kind::Action(open) => {
                if let Some(popup) = open(settings, context) {
                    return Activation::Open(Box::new(popup));
                }
            }
        }
        Activation::Changed
    }

    /// Enter while editing: store `text`.
    pub fn commit_edit(&self, settings: &mut ConfigDraft, text: &str) {
        if let Kind::Edit { set, .. } = self.kind {
            set(settings, text);
        }
    }
}

/// `[x] label` row for a `bool` field path of [`crate::config::settings::Settings`]; `indent` levels
/// of two spaces.
macro_rules! toggle {
    ($key:literal, $($field:ident).+) => {
        toggle!(0, $key, $($field).+)
    };
    ($indent:literal, $key:literal, $($field:ident).+) => {
        $crate::app::config_rows::Row::Setting($crate::app::config_rows::Setting {
            label: $crate::app::config_rows::Label::Key($key),
            indent: $indent,
            kind: $crate::app::config_rows::Kind::Toggle {
                get: |s| s.$($field).+,
                set: |s, v| s.$($field).+ = v,
            },
        })
    };
}
pub(crate) use toggle;

/// Builds a title row from fixed text.
pub fn title(text: &'static str) -> Row {
    Row::Title(Label::Raw(text))
}

/// Builds a title row from a translation key.
pub fn title_key(key: &'static str) -> Row {
    Row::Title(Label::Key(key))
}

/// A cycled setting row.
pub fn cycle(
    key: &'static str,
    indent: usize,
    format: CycleFormat,
    show: fn(&ConfigDraft) -> String,
    next: fn(&mut ConfigDraft),
) -> Row {
    Row::Setting(Setting {
        label: Label::Key(key),
        indent,
        kind: Kind::Cycle { show, next, format },
    })
}

#[cfg(test)]
mod tests;
