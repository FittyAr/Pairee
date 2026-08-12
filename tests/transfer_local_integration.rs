//! Integration-style filesystem contracts used by the transfer engine
//! (without starting the TUI). Complements in-crate worker unit tests.

use std::fs;
use std::path::PathBuf;

#[test]
fn multi_file_tree_copy_preserves_content() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("a.txt"), b"alpha").unwrap();
    fs::write(src.join("b.txt"), b"beta").unwrap();
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub").join("c.txt"), b"gamma").unwrap();

    copy_dir_recursive(&src, &dst.join("src")).unwrap();

    assert_eq!(
        fs::read_to_string(dst.join("src").join("a.txt")).unwrap(),
        "alpha"
    );
    assert_eq!(
        fs::read_to_string(dst.join("src").join("b.txt")).unwrap(),
        "beta"
    );
    assert_eq!(
        fs::read_to_string(dst.join("src").join("sub").join("c.txt")).unwrap(),
        "gamma"
    );
}

#[test]
fn conflict_rename_candidate_keeps_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("report.md");
    fs::write(&path, b"one").unwrap();
    let alt = unique_sibling(&path);
    assert_ne!(path, alt);
    assert!(
        alt.file_name()
            .unwrap()
            .to_string_lossy()
            .contains("report")
    );
    assert_eq!(alt.extension().and_then(|e| e.to_str()), Some("md"));
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

fn unique_sibling(path: &std::path::Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for i in 1..1000 {
        let candidate = parent.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}-copy{ext}"))
}
