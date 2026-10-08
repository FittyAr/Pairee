//! SFTP listing for the folder size walker (`fs::du`).

use super::SharedSshClient;
use super::sftp_ops::{entry_name, is_real_child};
use crate::fs::du::{DuEntry, DuKind, DuSource};
use std::io;
use std::path::Path;

impl DuSource for SharedSshClient {
    /// Reads one remote directory. The session is locked per directory so
    /// other remote operations can interleave with a long scan.
    fn read_dir(&self, dir: &Path) -> io::Result<Vec<DuEntry>> {
        let items = self.lock().sftp.readdir(dir).map_err(io::Error::other)?;
        Ok(items
            .into_iter()
            .filter_map(|(path, stat)| {
                let name = entry_name(&path);
                if !is_real_child(&name) {
                    return None;
                }
                // `readdir` reports link attributes, so links are not followed.
                let is_dir = stat.is_dir();
                Some(DuEntry {
                    name,
                    path,
                    kind: if is_dir { DuKind::Dir } else { DuKind::File },
                    size: if is_dir { 0 } else { stat.size.unwrap_or(0) },
                    id: None,
                })
            })
            .collect())
    }
}
