use crate::fs::error::LocalizedResult;
use anyhow::Result;
use std::fs;
use std::path::Path;

fn run_as_admin_mkdir(path: &Path) -> Result<()> {
    crate::fs::run_in_elevated_helper(vec![crate::fs::FsOperation::MkDir {
        path: path.to_path_buf(),
    }])
}

/// Creates a new directory at the specified path.
/// Creates an empty file at `path`; fails if anything exists there.
pub fn create_empty_file(path: &Path) -> std::io::Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
}

pub fn create_directory(path: &Path, req_admin: bool) -> Result<()> {
    let res = fs::create_dir(path).localized("fs_error_create_dir", path);
    if res.is_err() && req_admin {
        run_as_admin_mkdir(path)
    } else {
        res
    }
}
