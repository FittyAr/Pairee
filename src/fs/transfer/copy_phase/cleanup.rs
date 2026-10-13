//! After a move: removal of the source folders the move emptied.

use super::super::control::JobControl;
use crate::fs::descriptions::forget_description;
use crate::fs::vfs::Vfs;
use std::path::PathBuf;

/// Removes the emptied source folders on `fs` (deepest first). A folder
/// that still holds files (skipped or failed ones) stays.
pub fn cleanup_source_dirs(dirs_to_delete: &mut [PathBuf], ctl: &JobControl, fs: &dyn Vfs) {
    dirs_to_delete.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    let local = fs.capabilities().local_tools;
    for dir in dirs_to_delete.iter() {
        if ctl.is_cancelled() {
            break;
        }
        if fs.remove_dir(dir).is_ok() && local {
            forget_description(dir);
        }
    }
}
