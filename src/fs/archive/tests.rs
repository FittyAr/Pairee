//! Round-trip tests (create → list → extract) for every native format and
//! for the format dispatcher.

use super::test_fixtures::{SAMPLE_TREE, expected, run, snapshot, write_tar_gz, write_tree};
use super::*;

/// Extracts `archive` with the format dispatcher into a fresh `out/` folder
/// and returns that folder's file snapshot.
fn extract_all(archive: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let dest = archive.parent().unwrap().join("out");
    run(|tx, cancel| extract_archive(archive, &dest, tx, cancel)).unwrap();
    snapshot(&dest)
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.iter_mut().for_each(|n| *n = n.replace('\\', "/"));
    names.sort();
    names
}

#[test]
fn detect_format_uses_case_insensitive_extension() {
    let cases = [
        ("a.zip", "zip"),
        ("a.ZIP", "zip"),
        ("a.tar.gz", "targz"),
        ("a.tgz", "targz"),
        ("a.tar", "tar"),
        ("a.7z", "7z"),
        ("a.rar", "rar"),
        ("a.iso", "iso"),
        ("a.txt", "none"),
        ("noext", "none"),
    ];
    for (name, want) in cases {
        let got = match detect_format(Path::new(name)) {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGz => "targz",
            ArchiveFormat::SevenZ => "7z",
            ArchiveFormat::Rar => "rar",
            ArchiveFormat::Iso => "iso",
            ArchiveFormat::Unsupported => "none",
        };
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn unsupported_formats_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, b"plain").unwrap();
    assert!(list_archive_files(&path).is_err());
    assert!(run(|tx, cancel| extract_archive(&path, dir.path(), tx, cancel)).is_err());
}

#[test]
fn zip_round_trip_preserves_folder_and_loose_file() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("project");
    write_tree(&src, SAMPLE_TREE);
    let loose = dir.path().join("loose.txt");
    std::fs::write(&loose, b"loose").unwrap();

    let archive = dir.path().join("bundle.zip");
    run(|tx, cancel| compress_zip(vec![src.clone(), loose.clone()], &archive, tx, cancel)).unwrap();

    let listed = sorted(list_archive_files(&archive).unwrap());
    for (rel, _) in SAMPLE_TREE {
        assert!(listed.contains(&format!("project/{rel}")), "{listed:?}");
    }
    assert!(listed.contains(&"loose.txt".to_string()), "{listed:?}");

    let mut want = expected(SAMPLE_TREE, Some("project"));
    want.insert("loose.txt".into(), b"loose".to_vec());
    assert_eq!(extract_all(&archive), want);
}

#[test]
fn tar_gz_round_trip_restores_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("bundle.tar.gz");
    write_tar_gz(&archive, SAMPLE_TREE);

    let listed = sorted(list_archive_files(&archive).unwrap());
    let want_names: Vec<String> = SAMPLE_TREE.iter().map(|(r, _)| r.to_string()).collect();
    assert_eq!(listed, sorted(want_names));

    assert_eq!(extract_all(&archive), expected(SAMPLE_TREE, None));
}

#[test]
fn seven_z_round_trip_restores_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("project");
    write_tree(&src, SAMPLE_TREE);

    let archive = dir.path().join("bundle.7z");
    sevenz_rust2::compress_to_path(&src, &archive).unwrap();

    let listed = sorted(list_archive_files(&archive).unwrap());
    for (rel, _) in SAMPLE_TREE {
        assert!(listed.contains(&rel.to_string()), "{listed:?}");
    }

    assert_eq!(extract_all(&archive), expected(SAMPLE_TREE, None));
}

#[test]
fn extraction_stops_when_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("bundle.tar.gz");
    write_tar_gz(&archive, SAMPLE_TREE);
    let dest = dir.path().join("out");

    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    let cancel = AtomicBool::new(true);
    assert!(extract_archive(&archive, &dest, &tx, &cancel).is_err());
    assert!(!dest.join("a.txt").exists());
}
