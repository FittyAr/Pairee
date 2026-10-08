//! Editing a file of a non-local panel source (an SFTP server, a zip
//! archive): the file is downloaded to a private temporary folder, edited
//! there with the built-in editor and uploaded when it is saved, after
//! checking that the original was not changed in the meantime (size and
//! modification time). Network and archive work runs in the background.

use super::EditorState;
use super::document::EDITOR_MAX_BYTES;
use super::open::{OverwriteReason, open_in_editor};
use crate::app::state::{AppState, PopupType, VfsFollowUp};
use crate::fs::stamp::Stamp;
use crate::fs::vfs::{Vfs, VfsEntry};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where an editor's local copy came from.
#[derive(Debug, Clone)]
pub struct RemoteOrigin {
    pub vfs: Arc<dyn Vfs>,
    /// The file in the source's namespace.
    pub path: PathBuf,
    /// Size and time of the original when it was downloaded or last uploaded.
    pub stamp: Stamp,
    /// Holds the local copy; removed when the last editor or upload drops it.
    _dir: Arc<tempfile::TempDir>,
}

fn stamp_of(entry: &VfsEntry) -> Stamp {
    Stamp {
        size: entry.size,
        modified: entry.modified,
    }
}

/// Downloads `path` into a new temporary folder; returns the origin and
/// the local copy.
pub fn download(vfs: Arc<dyn Vfs>, path: &Path) -> io::Result<(RemoteOrigin, PathBuf)> {
    let entry = vfs.stat(path)?;
    if entry.size > EDITOR_MAX_BYTES {
        let message = crate::app::editor::open::load_error_message(
            &super::document::LoadError::TooLarge(entry.size),
        );
        return Err(io::Error::other(message));
    }
    let dir = tempfile::Builder::new().prefix("pairee-edit-").tempdir()?;
    let local = dir.path().join(crate::fs::file_name_lossy(path));
    io::copy(&mut vfs.open_read(path)?, &mut File::create(&local)?)?;
    let origin = RemoteOrigin {
        vfs,
        path: path.to_path_buf(),
        stamp: stamp_of(&entry),
        _dir: Arc::new(dir),
    };
    Ok((origin, local))
}

/// Uploads the local copy over the original. Unless `force` is set, a
/// changed original is left alone and `None` is returned. A deleted
/// original is recreated. Returns the original's new stamp.
pub fn upload(origin: &RemoteOrigin, local: &Path, force: bool) -> io::Result<Option<Stamp>> {
    if !force {
        match origin.vfs.stat(&origin.path) {
            Ok(entry) if stamp_of(&entry) != origin.stamp => return Ok(None),
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    origin
        .vfs
        .write_file(&origin.path, &mut File::open(local)?)?;
    origin.vfs.stat(&origin.path).map(|e| Some(stamp_of(&e)))
}

/// F4 on a non-local panel: downloads `path` in the background and opens
/// the copy in the editor.
pub fn open_remote(state: &mut AppState, vfs: Arc<dyn Vfs>, path: PathBuf) {
    state.start_vfs_task(move || {
        let (origin, local) = download(vfs, &path)?;
        let follow_up: VfsFollowUp = Box::new(move |state, settings| {
            open_in_editor(state, local.clone(), settings);
            if let Some(ed) = state.active_editor_mut().filter(|ed| ed.path == local) {
                ed.remote = Some(origin);
            }
        });
        Ok(follow_up)
    });
}

/// Uploads the saved copy `local` in the background; asks before
/// overwriting an original that changed since it was opened.
pub fn start_upload(state: &mut AppState, origin: RemoteOrigin, local: PathBuf, force: bool) {
    state.start_vfs_task(move || {
        let uploaded = upload(&origin, &local, force)?;
        let follow_up: VfsFollowUp = Box::new(move |state, _| match uploaded {
            Some(stamp) => state.set_remote_stamp(&local, stamp),
            None => state.dialogs.replace(PopupType::EditorConfirmOverwrite {
                target: local,
                reason: OverwriteReason::ChangedRemotely,
            }),
        });
        Ok(follow_up)
    });
}

impl AppState {
    /// Records the stamp of an upload on the editor of `local`, if open.
    fn set_remote_stamp(&mut self, local: &Path, stamp: Stamp) {
        for screen in self.screens.iter_mut() {
            if let crate::app::state::Screen::Editor(ed) = screen
                && ed.path == local
                && let Some(origin) = ed.remote.as_mut()
            {
                origin.stamp = stamp;
            }
        }
    }
}

impl EditorState {
    /// The path shown to the user: the original of a remote copy.
    pub fn display_path(&self) -> &Path {
        self.remote
            .as_ref()
            .map_or(&self.path, |origin| &origin.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::vfs::LocalVfs;

    fn origin_of(file: &Path) -> (RemoteOrigin, PathBuf) {
        download(Arc::new(LocalVfs), file).unwrap()
    }

    #[test]
    fn edited_copy_is_uploaded_over_an_unchanged_original() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "old").unwrap();
        let (origin, local) = origin_of(&file);
        assert_eq!(std::fs::read_to_string(&local).unwrap(), "old");
        std::fs::write(&local, "new text").unwrap();
        let stamp = upload(&origin, &local, false).unwrap().expect("uploaded");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new text");
        assert_eq!(stamp.size, 8);
    }

    #[test]
    fn a_changed_original_is_not_overwritten_unless_forced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "old").unwrap();
        let (origin, local) = origin_of(&file);
        std::fs::write(&file, "changed elsewhere").unwrap();
        std::fs::write(&local, "mine").unwrap();
        assert_eq!(upload(&origin, &local, false).unwrap(), None);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "changed elsewhere");
        assert!(upload(&origin, &local, true).unwrap().is_some());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "mine");
    }

    #[test]
    fn the_local_copy_lives_as_long_as_its_origin() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        let (origin, local) = origin_of(&file);
        let copy = origin.clone();
        drop(origin);
        assert!(local.exists());
        drop(copy);
        assert!(!local.exists());
    }
}
