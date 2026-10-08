//! Attributes dialog (Ctrl+A): reads the entry through the panel source,
//! on the UI thread for local files and in the background otherwise.

use crate::app::state::{AppState, FileAttrsSnapshot, PopupType, VfsFollowUp};
use crate::config::localization::t;
use crate::fs::attrs::FileAttrs;
use crate::fs::vfs::PanelSource;
use std::io;

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| e.name != "..")
    else {
        return true;
    };
    let path = entry.path.clone();
    let source = active.source.clone();
    let vfs = source.vfs();
    if source.is_local() {
        let read = vfs.attributes(&path);
        show(state, source, read);
    } else {
        state.start_vfs_task(move || {
            let read = vfs.attributes(&path);
            let follow_up: VfsFollowUp = Box::new(move |state, _| show(state, source, read));
            Ok(follow_up)
        });
    }
    true
}

/// Opens the dialog for `read`, or shows why the attributes are unknown.
fn show(state: &mut AppState, source: PanelSource, read: io::Result<FileAttrs>) {
    let popup = match read {
        Ok(attrs) => PopupType::FileAttributesDialog {
            mode_input: format!("{:o}", attrs.mode & 0o7777),
            attrs: FileAttrsSnapshot {
                source,
                path: attrs.path,
                readonly: attrs.readonly,
                size: attrs.size,
                modified: attrs.modified,
                created: attrs.created,
                owner: attrs.owner,
                nlinks: attrs.nlinks,
            },
        },
        Err(e) => PopupType::Error(format!("{} {}", t("error_read_attrs_failed"), e)),
    };
    state.dialogs.replace(popup);
}
