use super::*;

fn search() -> (tempfile::TempDir, DupesState) {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.txt", "b.txt", "c.txt"] {
        std::fs::write(dir.path().join(name), b"same contents").unwrap();
    }
    std::fs::write(dir.path().join("other.txt"), b"different").unwrap();
    let mut dupes = DupesState::default();
    // No runtime in unit tests: the search runs inline.
    dupes.start(
        vec![dir.path().to_path_buf()],
        PanelSource::default(),
        DupeOptions::default(),
    );
    (dir, dupes)
}

#[test]
fn finds_groups_and_marks_all_but_one_copy() {
    let (dir, mut dupes) = search();
    assert!(!dupes.is_scanning());
    assert_eq!(dupes.groups().len(), 1);
    assert_eq!(dupes.file_count(), 3);
    dupes.mark_all_but(Keep::First);
    assert_eq!(
        dupes.marked(),
        [dir.path().join("b.txt"), dir.path().join("c.txt")]
    );
    assert_eq!(dupes.marked_bytes(), 2 * 13);
    dupes.unmark_all();
    assert!(dupes.marked().is_empty());
}

#[test]
fn toggling_marks_the_highlighted_file() {
    let (dir, mut dupes) = search();
    dupes.cursor = 2;
    dupes.toggle_selected();
    assert!(dupes.is_marked(&dir.path().join("c.txt")));
    dupes.toggle_selected();
    assert!(!dupes.is_marked(&dir.path().join("c.txt")));
}

#[test]
fn deleted_files_leave_the_report() {
    let (dir, mut dupes) = search();
    dupes.mark_all_but(Keep::First);
    dupes.cursor = 2;
    let b = dir.path().join("b.txt");
    let c = dir.path().join("c.txt");
    assert!(dupes.forget(&[b.as_path(), c.as_path()]));
    assert!(dupes.groups().is_empty());
    assert!(dupes.marked().is_empty());
    assert_eq!(dupes.cursor, 0);
}
