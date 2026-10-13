//! The settings panel listings read on every refresh (sorting, `..` row,
//! folder selection, auto-refresh limit), copied from [`Settings`] by
//! [`AppState::apply_settings`] at start-up and whenever the settings change.

use super::AppState;
use crate::config::settings::Settings;

/// How listings are sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortPrefs {
    pub case_sensitive: bool,
    pub digits_as_numbers: bool,
    /// `linguistic`, `natural` or `ordinal`.
    pub collation: String,
    pub folder_names_by_extension: bool,
}

impl Default for SortPrefs {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            digits_as_numbers: false,
            collation: "linguistic".to_string(),
            folder_names_by_extension: false,
        }
    }
}

impl SortPrefs {
    /// Names compare digit runs as numbers.
    pub fn natural(&self) -> bool {
        self.digits_as_numbers || self.collation == "natural"
    }
}

/// Panel options mirrored from the settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelPrefs {
    pub sort: SortPrefs,
    /// Selecting by mask / Select all includes folders.
    pub select_folders: bool,
    pub show_dotdot_in_root_folders: bool,
    /// Folders with more entries than this are not reread automatically (0 = no limit).
    pub disable_panel_update_object_count: u32,
    /// Read folders through the elevated helper when access is denied.
    pub req_admin_reading: bool,
}

impl PanelPrefs {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            sort: SortPrefs {
                case_sensitive: settings.case_sensitive_sort,
                digits_as_numbers: settings.treat_digits_as_numbers,
                collation: settings.sorting_collation.clone(),
                folder_names_by_extension: settings.sort_folder_names_by_extension,
            },
            select_folders: settings.select_folders,
            show_dotdot_in_root_folders: settings.show_dotdot_in_root_folders,
            disable_panel_update_object_count: settings.disable_panel_update_object_count,
            req_admin_reading: settings.req_admin_reading,
        }
    }
}

impl AppState {
    /// Takes over the parts of `settings` the state keeps (panel options,
    /// auto-refresh monitors).
    pub fn apply_settings(&mut self, settings: &Settings) {
        self.prefs = PanelPrefs::from_settings(settings);
        self.screens.terminal_lines = settings.terminal_scrollback_lines as usize;
        self.auto_refresh.configure(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_settings_copies_the_panel_options() {
        let settings = Settings {
            case_sensitive_sort: true,
            sorting_collation: "natural".into(),
            select_folders: true,
            disable_panel_update_object_count: 7,
            ..Settings::default()
        };
        let mut state = AppState::new(".".into(), ".".into());
        state.apply_settings(&settings);
        assert!(state.prefs.sort.case_sensitive && state.prefs.sort.natural());
        assert!(state.prefs.select_folders);
        assert_eq!(state.prefs.disable_panel_update_object_count, 7);
    }
}
