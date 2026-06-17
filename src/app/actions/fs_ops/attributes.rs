use crate::app::state::{AppState, FileAttrsSnapshot, PopupType};
use crate::config::localization::t;

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        if entry.name != ".." {
            match crate::fs::read_attrs(&entry.path) {
                Ok(attrs) => {
                    let mode_octal = format!("{:o}", attrs.mode & 0o7777);
                    state.active_popup = Some(PopupType::FileAttributesDialog {
                        attrs: FileAttrsSnapshot {
                            path: attrs.path,
                            readonly: attrs.readonly,
                            size: attrs.size,
                            modified: attrs.modified,
                            created: attrs.created,
                            owner: attrs.owner,
                            nlinks: attrs.nlinks,
                        },
                        mode_input: mode_octal,
                    });
                }
                Err(e) => {
                    state.active_popup = Some(PopupType::Error(format!(
                        "{} {}",
                        t("error_read_attrs_failed"),
                        e
                    )));
                }
            }
        }
    }
    true
}
