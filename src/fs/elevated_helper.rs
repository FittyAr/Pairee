use super::privileges::FsOperation;
use crate::fs::transfer::job::LinkKind;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

pub fn run_elevated_helper_loop(temp_file_path: &Path) -> Result<()> {
    let res_file = temp_file_path.with_extension("res");

    // The .res file is read by the *unprivileged* caller after
    // we exit. If we create it with the default umask (022 →
    // mode 0644 on Unix), any local user can read the result
    // string and, more importantly, the error messages we
    // surface (which include the paths we operated on, e.g.
    // "Failed to delete /etc/shadow"). Write the result with
    // an explicit 0600 mode so only the file owner can read it.
    // On Windows, default ACLs are per-user and this is a no-op.
    write_res_file(&res_file, "")?;

    let run = || -> Result<()> {
        let content = std::fs::read_to_string(temp_file_path)
            .context("Failed to read operations temp file")?;
        let ops: Vec<FsOperation> =
            serde_json::from_str(&content).context("Failed to deserialize operations JSON")?;

        for op in ops {
            match op {
                FsOperation::Delete { path } => {
                    if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                            .with_context(|| format!("Failed to delete directory: {:?}", path))?;
                    } else {
                        std::fs::remove_file(&path)
                            .with_context(|| format!("Failed to delete file: {:?}", path))?;
                    }
                }
                FsOperation::MkDir { path } => {
                    std::fs::create_dir_all(&path)
                        .with_context(|| format!("Failed to create directory: {:?}", path))?;
                }
                FsOperation::Copy { src, dst } => {
                    copy_recursive(&src, &dst)
                        .with_context(|| format!("Failed to copy {:?} to {:?}", src, dst))?;
                }
                FsOperation::Move { src, dst } => {
                    move_operation(&src, &dst)
                        .with_context(|| format!("Failed to move {:?} to {:?}", src, dst))?;
                }
                FsOperation::Chmod { path, mode } => {
                    set_mode(&path, mode)
                        .with_context(|| format!("Failed to set permissions on {:?}", path))?;
                }
                FsOperation::Rename { src, dst } => {
                    std::fs::rename(&src, &dst)
                        .with_context(|| format!("Failed to rename {:?} to {:?}", src, dst))?;
                }
                FsOperation::CreateLink { src, dst, kind } => {
                    create_link(&src, &dst, kind)
                        .with_context(|| format!("Failed to create link {:?} -> {:?}", src, dst))?;
                }
                FsOperation::Wipe { path, passes } => {
                    let passes = passes.clamp(1, 3);
                    secure_wipe(&path, passes)
                        .with_context(|| format!("Failed to wipe {:?}", path))?;
                    if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                            .with_context(|| format!("Failed to delete directory: {:?}", path))?;
                    } else {
                        std::fs::remove_file(&path)
                            .with_context(|| format!("Failed to delete file: {:?}", path))?;
                    }
                }
            }
        }
        Ok(())
    };

    match run() {
        Ok(_) => {
            let _ = write_res_file(&res_file, "OK");
            Ok(())
        }
        Err(e) => {
            let _ = write_res_file(&res_file, &format!("{:#}", e));
            Err(e)
        }
    }
}

/// Write the helper's result file with explicit 0600 perms so a
/// concurrent local user cannot read the (potentially path-
/// revealing) error message. `std::fs::write` would use the
/// process umask, which is 022 by default on most systems →
/// mode 0644 = world-readable.
fn write_res_file(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            copy_recursive(&src_path, &dst_path)?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

fn move_operation(src: &Path, dst: &Path) -> std::io::Result<()> {
    // Fast path: same-filesystem rename (atomic, no data copy).
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    // Slow path: copy then delete. If the post-copy delete fails, we
    // must roll back the copy so the user is not left with TWO copies
    // of the data and a half-deleted source. The previous code left
    // the duplicated state in place and just propagated the error.
    copy_recursive(src, dst)?;
    let remove_res = if src.is_dir() {
        std::fs::remove_dir_all(src)
    } else {
        std::fs::remove_file(src)
    };
    if let Err(e) = remove_res {
        // Best-effort rollback: delete the destination we just created
        // so the user is back in the original state (data at `src`,
        // nothing at `dst`). We still surface the original error.
        let rollback_res = if dst.is_dir() {
            std::fs::remove_dir_all(dst)
        } else {
            std::fs::remove_file(dst)
        };
        if let Err(rb) = rollback_res {
            // Both the original operation AND the rollback failed.
            // Report both so the user can attempt manual recovery.
            return Err(std::io::Error::new(
                e.kind(),
                format!(
                    "move failed: {}; rollback also failed: {}. \
                     Data is now at both {:?} and {:?}; manual recovery required.",
                    e, rb, src, dst
                ),
            ));
        }
        return Err(e);
    }
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms)?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}

fn create_link(src: &Path, dst: &Path, kind: LinkKind) -> std::io::Result<()> {
    match kind {
        LinkKind::Symbolic => {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(src, dst)?;
            }
            #[cfg(windows)]
            {
                // The Windows API needs the target
                // type (file vs. dir) to pick the
                // right syscall. Probe the source
                // to decide.
                if src.is_dir() {
                    std::os::windows::fs::symlink_dir(src, dst)?;
                } else {
                    std::os::windows::fs::symlink_file(src, dst)?;
                }
            }
        }
        LinkKind::Hard => {
            // `std::fs::hard_link` is cross-platform.
            std::fs::hard_link(src, dst)?;
        }
    }
    Ok(())
}

/// Overwrite the file's bytes with alternating
/// `0x00` and `0xFF` patterns, then unlink it.
/// `passes` is clamped to 1-3 by the caller.
fn secure_wipe(path: &Path, passes: u8) -> std::io::Result<()> {
    if path.is_dir() {
        // Wipe recursively: for each file, secure-wipe
        // the contents; for each directory, recurse.
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            secure_wipe(&entry.path(), passes)?;
        }
        return Ok(());
    }

    // Open the file with O_NOFOLLOW (Unix) so a symlink
    // swap between the `is_dir` check and the actual
    // open cannot redirect our overwrite to an arbitrary
    // target. The `metadata` call below uses `symlink_metadata`
    // (via `std::fs::metadata`'s default) — combined with
    // O_NOFOLLOW this closes the TOCTOU window.
    #[cfg(unix)]
    let f = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?
    };
    #[cfg(not(unix))]
    let f = std::fs::OpenOptions::new().write(true).open(path)?;

    let meta = f.metadata()?;
    let len = meta.len() as usize;
    if len == 0 {
        return Ok(());
    }

    // 64 KiB buffer is large enough to amortise the
    // syscall cost without blowing up memory.
    const BUF: usize = 64 * 1024;
    let zero = vec![0u8; BUF.min(len)];
    let ones = vec![0xFFu8; BUF.min(len)];

    for pass in 0..passes {
        let pattern: &[u8] = if pass % 2 == 0 { &zero } else { &ones };
        let mut written = 0usize;
        let mut f = f.try_clone()?;
        while written < len {
            let chunk = pattern.len().min(len - written);
            f.write_all(&pattern[..chunk])?;
            written += chunk;
        }
        f.sync_all()?;
    }
    Ok(())
}
