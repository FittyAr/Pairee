//! Which folders cannot rely on native change notifications.

use std::path::Path;

/// True for folders on network shares (SMB/NFS mapped drives, UNC paths,
/// WSL `\\wsl$` paths) and, on Linux, on file systems whose notifications
/// miss changes made elsewhere (9p/drvfs Windows drives under WSL, FUSE
/// mounts such as sshfs). Such folders are polled.
///
/// May block on an unresponsive network mount: call it off the UI thread.
pub fn needs_polling(dir: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        let text = dir.to_string_lossy();
        if text.starts_with(r"\\?\UNC\") {
            return true;
        }
        // Verbatim local paths (`\\?\C:\…`) are not UNC shares.
        match text.strip_prefix(r"\\?\") {
            Some(local) => crate::fs::transfer::network::is_lan_path(Path::new(local)),
            None => crate::fs::transfer::network::is_lan_path(dir),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        const UNWATCHABLE_FS: [&str; 3] = ["9p", "drvfs", "fuse"];
        crate::fs::transfer::network::is_lan_path(dir)
            || crate::fs::transfer::network::mount_fs_type(dir)
                .is_some_and(|fs| UNWATCHABLE_FS.contains(&fs.as_str()) || fs.starts_with("fuse."))
    }
}
