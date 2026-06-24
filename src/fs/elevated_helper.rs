use std::path::Path;
use anyhow::{Result, Context};
use super::privileges::FsOperation;

pub fn run_elevated_helper_loop(temp_file_path: &Path) -> Result<()> {
    let res_file = temp_file_path.with_extension("res");

    let run = || -> Result<()> {
        let content = std::fs::read_to_string(temp_file_path)
            .context("Failed to read operations temp file")?;
        let ops: Vec<FsOperation> = serde_json::from_str(&content)
            .context("Failed to deserialize operations JSON")?;

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
    if std::fs::rename(src, dst).is_err() {
        copy_recursive(src, dst)?;
        if src.is_dir() {
            std::fs::remove_dir_all(src)?;
        } else {
            std::fs::remove_file(src)?;
        }
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
