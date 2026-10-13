use super::privileges::FsOperation;
use crate::fs::vfs::{LocalVfs, Vfs};
use crate::fs::walk::{WalkOptions, Walker};
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
                    LocalVfs
                        .remove_all(&path)
                        .with_context(|| format!("Failed to delete: {:?}", path))?;
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

/// Copies `src` (a file, or a folder with everything below it; links are
/// followed, loops are reported) to `dst`.
fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.is_dir() {
        return copy_file(src, dst);
    }
    std::fs::create_dir_all(dst)?;
    for item in Walker::new(&LocalVfs, src, WalkOptions::following(true)) {
        let item = item?;
        let target = dst.join(item.path().strip_prefix(src).unwrap_or(item.path()));
        if item.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            copy_file(item.path(), &target)?;
        }
    }
    Ok(())
}

/// One file, through the atomic write every local copy uses, with its
/// times and attributes.
fn copy_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    LocalVfs.write_file(dst, &mut std::fs::File::open(src)?)?;
    let options = crate::fs::transfer::options::TransferOptions::default();
    crate::fs::transfer::metadata::preserve_metadata(src, dst, &options)
}

fn move_operation(src: &Path, dst: &Path) -> std::io::Result<()> {
    if std::fs::rename(src, dst).is_err() {
        copy_recursive(src, dst)?;
        LocalVfs.remove_all(src)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_and_moves_whole_trees() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub").join("f.txt"), b"data").unwrap();
        let copy = dir.path().join("copy");
        copy_recursive(&src, &copy).unwrap();
        assert_eq!(
            std::fs::read(copy.join("sub").join("f.txt")).unwrap(),
            b"data"
        );
        let moved = dir.path().join("moved");
        move_operation(&copy, &moved).unwrap();
        assert!(!copy.exists());
        assert_eq!(
            std::fs::read(moved.join("sub").join("f.txt")).unwrap(),
            b"data"
        );
    }
}
