use super::key::name_key;
use super::summary::mtime_within;
use super::*;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn set_mtime(path: &Path, time: SystemTime) {
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(time)).unwrap();
}

fn status_of<'a>(results: &'a [CompareEntry], name: &str) -> Option<&'a CompareStatus> {
    results.iter().find(|e| e.name == name).map(|e| &e.status)
}

fn options(tolerance_secs: u64, case_insensitive: bool) -> CompareOptions {
    CompareOptions {
        mtime_tolerance: Duration::from_secs(tolerance_secs),
        case_insensitive,
    }
}

#[test]
fn compare_basic_statuses() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let t = SystemTime::now() - Duration::from_secs(3600);
    set_mtime(&write_file(left.path(), "common.txt", b"same"), t);
    set_mtime(&write_file(right.path(), "common.txt", b"same"), t);
    write_file(left.path(), "left_only.rs", b"l");
    write_file(right.path(), "right_only.rs", b"r");
    write_file(left.path(), "differ.txt", b"version A");
    write_file(right.path(), "differ.txt", b"version B longer");

    let results =
        compare_directories(left.path(), right.path(), &CompareOptions::default()).unwrap();

    assert_eq!(
        status_of(&results, "left_only.rs"),
        Some(&CompareStatus::OnlyLeft)
    );
    assert_eq!(
        status_of(&results, "right_only.rs"),
        Some(&CompareStatus::OnlyRight)
    );
    assert_eq!(
        status_of(&results, "differ.txt"),
        Some(&CompareStatus::Different)
    );
    assert_eq!(
        status_of(&results, "common.txt"),
        Some(&CompareStatus::Equal)
    );
    let names: Vec<_> = results.iter().map(|e| e.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "results are sorted by name");
}

#[test]
fn fat_two_second_granularity_is_equal_by_default() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let t = SystemTime::now() - Duration::from_secs(3600);
    set_mtime(&write_file(left.path(), "a.txt", b"x"), t);
    set_mtime(
        &write_file(right.path(), "a.txt", b"x"),
        t + Duration::from_millis(1990),
    );

    let default = compare_directories(left.path(), right.path(), &CompareOptions::default());
    assert_eq!(
        status_of(&default.unwrap(), "a.txt"),
        Some(&CompareStatus::Equal)
    );
    let strict = compare_directories(left.path(), right.path(), &options(1, false)).unwrap();
    assert_eq!(status_of(&strict, "a.txt"), Some(&CompareStatus::Different));
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
    set_mtime(&write_file(left.path(), "Readme.TXT", b"x"), t);
    set_mtime(&write_file(right.path(), "readme.txt", b"x"), t);

    let ci = compare_directories(left.path(), right.path(), &options(2, true)).unwrap();
    assert_eq!(ci.len(), 1);
    assert_eq!(ci[0].name, "Readme.TXT", "left spelling is shown");
    assert_eq!(ci[0].status, CompareStatus::Equal);

    let cs = compare_directories(left.path(), right.path(), &options(2, false)).unwrap();
    assert_eq!(status_of(&cs, "Readme.TXT"), Some(&CompareStatus::OnlyLeft));
    assert_eq!(
        status_of(&cs, "readme.txt"),
        Some(&CompareStatus::OnlyRight)
    );
}

#[test]
fn platform_default_case_handling() {
    assert_eq!(
        platform_case_insensitive(),
        cfg!(any(windows, target_os = "macos"))
    );
    assert_eq!(name_key("AbC", true), "abc");
    assert_eq!(name_key("AbC", false), "AbC");
}

#[test]
fn file_and_folder_with_same_name_differ() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    write_file(left.path(), "item", b"x");
    std::fs::create_dir(right.path().join("item")).unwrap();
    let results =
        compare_directories(left.path(), right.path(), &CompareOptions::default()).unwrap();
    assert_eq!(status_of(&results, "item"), Some(&CompareStatus::Different));
}
