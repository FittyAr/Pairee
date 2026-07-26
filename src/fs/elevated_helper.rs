use super::privileges::FsOperation;
use anyhow::{Context, Result};
use std::path::Path;

pub fn run_elevated_helper_loop(temp_file_path: &Path) -> Result<()> {
    let res_file = temp_file_path.with_extension("res");

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
            }
        }
        Ok(())
    };

    match run() {
        Ok(_) => {
            let _ = std::fs::write(&res_file, "OK");
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::write(&res_file, format!("{:#}", e));
            Err(e)
        }
    }
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
