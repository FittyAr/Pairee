//! In-place editing of every writable format (FEAT-10): one generic check
//! per behaviour, run for zip, tar, tar.gz, tar.bz2, tar.xz and 7z.

use super::edit::{ArchiveEdit, EntrySource};
use super::test_fixtures::{
    SAMPLE_TREE, write_7z, write_tar, write_tar_bz2, write_tar_gz, write_tar_xz, write_zip,
};
use super::{ArchiveVfs, browsable_reader};
use crate::fs::vfs::Vfs;
use std::path::{Path, PathBuf};

type Writer = fn(&Path, &[(&str, &[u8])]);

const FORMATS: &[(&str, Writer)] = &[
    ("a.zip", write_zip),
    ("a.tar", write_tar),
    ("a.tar.gz", write_tar_gz),
    ("a.tar.bz2", write_tar_bz2),
    ("a.tar.xz", write_tar_xz),
    ("a.7z", write_7z),
];

fn rewrite(archive: &Path, edit: &ArchiveEdit) {
    let reader = browsable_reader(archive).unwrap();
    assert!(reader.writable(), "{archive:?}");
    reader.rewrite(archive, edit, &mut |_| Ok(())).unwrap();
}

/// Files of the archive (folders left out), sorted.
fn files(archive: &Path) -> Vec<String> {
    let reader = browsable_reader(archive).unwrap();
    let mut names: Vec<String> = reader
        .entries(archive)
        .unwrap()
        .into_iter()
        .filter(|e| e.kind == super::format::EntryKind::File)
        .map(|e| e.name.trim_start_matches("./").to_string())
        .collect();
    names.sort();
    names
}

fn read(archive: &Path, rel: &str) -> Vec<u8> {
    let vfs = ArchiveVfs::open(archive.to_path_buf()).unwrap();
    vfs.read_prefix(&archive.join(rel), 1024).unwrap()
}

#[test]
fn every_format_adds_replaces_and_removes_entries() {
    for (name, write) in FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join(name);
        write(&archive, SAMPLE_TREE);
        let local = dir.path().join("local.txt");
        std::fs::write(&local, b"from disk").unwrap();
        let edit = ArchiveEdit {
            remove: vec![PathBuf::from("sub").join("deeper")],
            add: vec![
                (PathBuf::from("a.txt"), EntrySource::Bytes(b"new".to_vec())),
                (PathBuf::from("made"), EntrySource::Dir),
                (PathBuf::from("made/local.txt"), EntrySource::File(local)),
            ],
        };
        rewrite(&archive, &edit);
        assert_eq!(
            files(&archive),
            ["a.txt", "made/local.txt", "sub/b.txt"],
            "{name}"
        );
        assert_eq!(read(&archive, "a.txt"), b"new", "{name}");
        assert_eq!(read(&archive, "sub/b.txt"), b"bravo", "{name}");
        assert_eq!(read(&archive, "made/local.txt"), b"from disk", "{name}");
    }
}

#[test]
fn a_failed_rewrite_keeps_the_original() {
    for (name, write) in FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join(name);
        write(&archive, SAMPLE_TREE);
        let before = std::fs::read(&archive).unwrap();
        let edit = ArchiveEdit {
            add: vec![(PathBuf::from("x.txt"), EntrySource::Bytes(b"x".to_vec()))],
            ..ArchiveEdit::default()
        };
        let reader = browsable_reader(&archive).unwrap();
        let cancelled = reader.rewrite(&archive, &edit, &mut |_| {
            Err(std::io::Error::other("cancelled"))
        });
        assert!(cancelled.is_err(), "{name}");
        assert_eq!(std::fs::read(&archive).unwrap(), before, "{name}");
        let leftovers = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(leftovers, 1, "{name}: no temporary file left behind");
    }
}

/// The archive keeps its mode, and added files keep theirs (FS-14).
#[cfg(unix)]
#[test]
fn rewriting_keeps_archive_and_entry_permissions() {
    use std::os::unix::fs::PermissionsExt;
    for (name, write) in FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join(name);
        write(&archive, SAMPLE_TREE);
        std::fs::set_permissions(&archive, std::fs::Permissions::from_mode(0o640)).unwrap();
        let script = dir.path().join("run.sh");
        std::fs::write(&script, b"#!/bin/sh").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o750)).unwrap();
        let edit = ArchiveEdit {
            add: vec![(PathBuf::from("run.sh"), EntrySource::File(script))],
            ..ArchiveEdit::default()
        };
        rewrite(&archive, &edit);
        let mode = std::fs::metadata(&archive).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "{name}");
        let entry = browsable_reader(&archive)
            .unwrap()
            .entries(&archive)
            .unwrap()
            .into_iter()
            .find(|e| e.name == "run.sh")
            .unwrap();
        assert_eq!(entry.mode.map(|m| m & 0o777), Some(0o750), "{name}");
    }
}
