use super::key::name_key;
use super::*;
use std::path::Path;
use std::time::{Duration, SystemTime};

fn write_file(dir: &Path, name: &str, content: &[u8], time: SystemTime) {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(time)).unwrap();
}

fn pairs(left: &Path, right: &Path, case_insensitive: bool) -> Vec<EntryPair> {
    let scan = |dir| {
        scan_directory(&crate::fs::vfs::LocalVfs, dir, case_insensitive, |_, _| {
            true
        })
        .unwrap()
    };
    pair_entries(scan(left), scan(right))
}

fn summary(size: u64, secs: u64, is_dir: bool) -> FileSummary {
    FileSummary {
        size,
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
        is_dir,
    }
}

#[test]
fn pairs_are_sorted_and_classified() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let t = SystemTime::now() - Duration::from_secs(3600);
    write_file(left.path(), "b_common.txt", b"same", t);
    write_file(right.path(), "b_common.txt", b"same", t);
    write_file(left.path(), "a_left.rs", b"l", t);
    write_file(right.path(), "c_right.rs", b"r", t);

    let pairs = pairs(left.path(), right.path(), false);
    let names: Vec<_> = pairs.iter().map(EntryPair::name).collect();
    assert_eq!(names, ["a_left.rs", "b_common.txt", "c_right.rs"]);
    assert!(matches!(pairs[0], EntryPair::Left(_)));
    assert!(matches!(&pairs[1], EntryPair::Both(l, r)
        if metadata_equal(&l.summary, &r.summary, Duration::from_secs(2))));
    assert!(matches!(pairs[2], EntryPair::Right(_)));
}

#[test]
fn fat_two_second_granularity_is_equal_by_default() {
    let default = CompareOptions::default().mtime_tolerance;
    assert_eq!(default, Duration::from_secs(2));
    let a = FileSummary {
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(1990)),
        ..summary(1, 0, false)
    };
    assert!(metadata_equal(&a, &summary(1, 0, false), default));
    assert!(!metadata_equal(
        &a,
        &summary(1, 0, false),
        Duration::from_secs(1)
    ));
    assert!(!metadata_equal(
        &summary(2, 0, false),
        &summary(1, 0, false),
        default
    ));
    assert!(!metadata_equal(
        &summary(0, 0, true),
        &summary(0, 0, false),
        default
    ));
    assert!(metadata_equal(
        &summary(0, 0, true),
        &summary(0, 9, true),
        default
    ));
}

#[test]
fn tolerance_is_configurable() {
    assert!(mtime_within(
        Some(SystemTime::UNIX_EPOCH),
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(5)),
        Duration::from_secs(5)
    ));
    assert!(!mtime_within(
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(6)),
        Some(SystemTime::UNIX_EPOCH),
        Duration::from_secs(5)
    ));
    assert!(mtime_within(None, None, Duration::ZERO));
    assert!(!mtime_within(
        Some(SystemTime::UNIX_EPOCH),
        None,
        Duration::MAX
    ));

    let mut settings = crate::config::settings::Settings::default();
    assert_eq!(
        CompareOptions::from_settings(&settings).mtime_tolerance,
        Duration::from_secs(DEFAULT_MTIME_TOLERANCE_SECS)
    );
    settings.compare_mtime_tolerance_secs = 7;
    assert_eq!(
        CompareOptions::from_settings(&settings).mtime_tolerance,
        Duration::from_secs(7)
    );
}

#[test]
fn case_insensitive_matching_pairs_names() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let t = SystemTime::now() - Duration::from_secs(3600);
    write_file(left.path(), "Readme.TXT", b"x", t);
    write_file(right.path(), "readme.txt", b"x", t);

    let ci = pairs(left.path(), right.path(), true);
    assert_eq!(ci.len(), 1);
    assert_eq!(ci[0].name(), "Readme.TXT", "left spelling is shown");
    assert!(
        matches!(&ci[0], EntryPair::Both(_, r) if r.name == "readme.txt"),
        "right entry keeps its own spelling"
    );

    let cs = pairs(left.path(), right.path(), false);
    assert_eq!(cs.len(), 2);
}

#[test]
fn platform_default_case_handling() {
    assert_eq!(
        platform_case_insensitive(),
        cfg!(any(windows, target_os = "macos"))
    );
    assert_eq!(
        CompareOptions::default().case_insensitive,
        platform_case_insensitive()
    );
    assert_eq!(name_key("AbC", true), "abc");
    assert_eq!(name_key("AbC", false), "AbC");
}
