//! In-place editing of every writable format (FEAT-10): one generic check
//! per behaviour, run for zip, tar, tar.gz, tar.bz2, tar.xz and 7z.

use super::edit::{ArchiveEdit, EntrySource, PrivateTempFile};
use super::test_fixtures::{
    SAMPLE_TREE, write_7z, write_tar, write_tar_bz2, write_tar_gz, write_tar_xz, write_zip,
};
use super::{ArchiveVfs, browsable_reader};
use crate::fs::vfs::Vfs;
use std::io::{self, Read};
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

/// A local file holding `bytes` (removed when dropped).
fn staged(bytes: &[u8]) -> (PrivateTempFile, EntrySource) {
    let file = PrivateTempFile::from_reader("data", &mut &bytes[..]).unwrap();
    let source = EntrySource::File(file.path().to_path_buf());
    (file, source)
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
        let (_new, new) = staged(b"new");
        let edit = ArchiveEdit {
            remove: vec![PathBuf::from("sub").join("deeper")],
            add: vec![
                (PathBuf::from("a.txt"), new),
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
        let (_x, x) = staged(b"x");
        let edit = ArchiveEdit {
            add: vec![(PathBuf::from("x.txt"), x)],
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

/// An archive opened through a link is edited where it really is: the
/// link stays a link and the archive it points to gets the change.
#[test]
fn editing_through_a_link_changes_the_real_archive() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.zip");
    write_zip(&real, SAMPLE_TREE);
    let link = dir.path().join("link.zip");
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&real, &link);
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(&real, &link);
    if made.is_err() {
        // Links need Developer Mode or an administrator on Windows.
        return;
    }
    let (_n, n) = staged(b"n");
    let edit = ArchiveEdit {
        add: vec![(PathBuf::from("new.txt"), n)],
        ..ArchiveEdit::default()
    };
    rewrite(&link, &edit);
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(files(&real).contains(&"new.txt".to_string()));
}

/// Size of the streams written in the tests below.
const STREAM_BYTES: u64 = 256 * 1024;

/// Length of the staged copy named `name`, if one exists.
fn staged_len(name: &str) -> Option<u64> {
    std::fs::read_dir(std::env::temp_dir())
        .ok()?
        .flatten()
        .filter(|d| {
            d.file_name()
                .to_string_lossy()
                .starts_with("pairee-archive-")
        })
        .find_map(|d| std::fs::metadata(d.path().join(name)).ok())
        .map(|m| m.len())
}

/// A stream of `STREAM_BYTES` bytes, read in small pieces. Before each
/// read it checks that everything yielded so far is already on disk in the
/// staged copy named `name`: the stream is never gathered in memory.
/// With `fail_at`, the read reaching that offset fails (a cancellation).
struct Stream {
    name: String,
    sent: u64,
    fail_at: Option<u64>,
}

impl Stream {
    fn new(fail_at: Option<u64>) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        Self {
            name: format!("stream-{}-{nanos}.bin", std::process::id()),
            sent: 0,
            fail_at,
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.sent > 0 {
            assert_eq!(staged_len(&self.name), Some(self.sent), "not streamed");
        }
        if self.fail_at.is_some_and(|at| self.sent >= at) {
            return Err(io::Error::other("cancelled"));
        }
        let n = (STREAM_BYTES - self.sent).min(buf.len() as u64) as usize;
        buf[..n].fill(b'z');
        self.sent += n as u64;
        Ok(n)
    }
}

/// FS2-38: a stream written into an archive goes through a temporary file
/// (never whole into memory), which is gone once the archive is rewritten.
#[test]
fn every_format_streams_written_files_through_a_temporary_file() {
    for (name, write) in FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join(name);
        write(&archive, SAMPLE_TREE);
        let vfs = ArchiveVfs::open(archive.clone()).unwrap();
        let mut stream = Stream::new(None);
        let entry = archive.join(&stream.name);
        vfs.write_file(&entry, &mut stream).unwrap();
        assert_eq!(stream.sent, STREAM_BYTES, "{name}");
        assert_eq!(vfs.stat(&entry).unwrap().size, STREAM_BYTES, "{name}");
        assert_eq!(vfs.read_prefix(&entry, 4).unwrap(), b"zzzz", "{name}");
        assert_eq!(staged_len(&stream.name), None, "{name}: staged copy left");
    }
}

/// A stream that fails half-way (a cancelled copy) leaves the archive as
/// it was and no temporary file behind.
#[test]
fn a_failed_stream_keeps_the_archive_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, SAMPLE_TREE);
    let before = std::fs::read(&archive).unwrap();
    let vfs = ArchiveVfs::open(archive.clone()).unwrap();
    let mut stream = Stream::new(Some(STREAM_BYTES / 2));
    let entry = archive.join(&stream.name);
    assert!(vfs.write_file(&entry, &mut stream).is_err());
    assert_eq!(std::fs::read(&archive).unwrap(), before);
    assert_eq!(staged_len(&stream.name), None);
}
