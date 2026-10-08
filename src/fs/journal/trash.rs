//! Reading items back from the platform trash (Windows Recycle Bin and
//! Freedesktop trash; the `trash` crate cannot restore on macOS).

use crate::config::localization::t;
use std::path::Path;

/// Emits the `restorable` items where the `trash` crate can list and
/// restore the trash (`trash::os_limited`), the `other` items elsewhere.
macro_rules! by_platform {
    (restorable: { $($yes:item)* } other: { $($no:item)* }) => {
        $(
            #[cfg(any(
                target_os = "windows",
                all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
            ))]
            $yes
        )*
        $(
            #[cfg(not(any(
                target_os = "windows",
                all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
            )))]
            $no
        )*
    };
}

fn not_in_trash(path: &Path) -> String {
    t("journal_not_in_trash").replacen("{}", &path.to_string_lossy(), 1)
}

by_platform! {
    restorable: {
        use crate::fs::stamp::entry_exists;

        /// `true` where trashed items can be restored.
        pub const RESTORE_SUPPORTED: bool = true;

        /// Snapshot of the trash contents, listed once and consumed by restores.
        pub struct TrashIndex {
            items: Vec<trash::TrashItem>,
        }

        impl TrashIndex {
            /// Lists the trash.
            pub fn load() -> Result<Self, String> {
                trash::os_limited::list()
                    .map(|items| Self { items })
                    .map_err(|e| e.to_string())
            }

            /// Restores the most recently trashed item that was at `path`.
            pub fn restore(&mut self, path: &Path) -> Result<(), String> {
                let found = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| was_at(item, path))
                    .max_by_key(|(_, item)| item.time_deleted)
                    .map(|(index, _)| index);
                let Some(index) = found else {
                    return Err(not_in_trash(path));
                };
                let item = self.items.swap_remove(index);
                let landed = item.original_path();
                trash::os_limited::restore_all([item]).map_err(|e| e.to_string())?;
                // Restored under the display name (see `was_at`): put the
                // extension back.
                if landed != path && !entry_exists(path) && entry_exists(&landed) {
                    std::fs::rename(&landed, path).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        }

        /// `true` when `item` was trashed from `path`. The Windows Recycle
        /// Bin reports display names, which lack the extension when
        /// Explorer hides extensions; the item id keeps it.
        fn was_at(item: &trash::TrashItem, path: &Path) -> bool {
            let fs = crate::fs::multi_rename::TargetFs::local();
            let same = |a: &std::ffi::OsStr, b: &std::ffi::OsStr| {
                fs.key(&a.to_string_lossy()) == fs.key(&b.to_string_lossy())
            };
            let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
                return false;
            };
            if fs.path_key(&item.original_parent) != fs.path_key(parent) {
                return false;
            }
            if same(&item.name, name) {
                return true;
            }
            let id = Path::new(&item.id);
            match (path.file_stem(), path.extension(), id.extension()) {
                (Some(stem), Some(ext), Some(id_ext)) => same(&item.name, stem) && same(ext, id_ext),
                _ => false,
            }
        }

        /// Test probe: `true` when a file trashed from `dir` can be listed
        /// and restored here. Some CI runners (Windows without a usable
        /// Recycle Bin, tmpfs without `.Trash`) cannot, and tests that need
        /// the trash skip there. The probe file ends up deleted either way.
        #[cfg(test)]
        pub fn trash_round_trips_in(dir: &Path) -> bool {
            let probe = dir.join(format!("pairee-trash-probe-{}.txt", uuid::Uuid::new_v4()));
            if std::fs::write(&probe, b"probe").is_err() || trash::delete(&probe).is_err() {
                let _ = std::fs::remove_file(&probe);
                return false;
            }
            let restored = TrashIndex::load().and_then(|mut index| index.restore(&probe));
            let usable = restored.is_ok() && entry_exists(&probe);
            let _ = std::fs::remove_file(&probe);
            usable
        }

        #[cfg(test)]
        mod tests {
            use super::was_at;
            use std::path::{Path, PathBuf};

            fn item(name: &str, parent: &Path, id: &str) -> trash::TrashItem {
                trash::TrashItem {
                    id: id.into(),
                    name: name.into(),
                    original_parent: parent.to_path_buf(),
                    time_deleted: 0,
                }
            }

            #[test]
            fn items_match_by_folder_and_name_even_without_extension() {
                let dir = PathBuf::from("base").join("dir");
                let file = dir.join("report.txt");
                assert!(was_at(&item("report.txt", &dir, "x"), &file));
                assert!(was_at(&item("report", &dir, "$R1.txt"), &file));
                assert!(!was_at(&item("report", &dir, "$R1.md"), &file));
                assert!(!was_at(&item("report.txt", Path::new("other"), "x"), &file));
            }
        }
    }
    other: {
        /// `true` where trashed items can be restored.
        pub const RESTORE_SUPPORTED: bool = false;

        /// The trash cannot be read back on this platform.
        pub struct TrashIndex;

        impl TrashIndex {
            pub fn load() -> Result<Self, String> {
                Err(t("journal_restore_unsupported"))
            }

            pub fn restore(&mut self, path: &Path) -> Result<(), String> {
                Err(not_in_trash(path))
            }
        }
    }
}
