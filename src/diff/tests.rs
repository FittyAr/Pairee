use super::files::{FileComparison, FileSide, compare_files};
use super::text::diff_texts;
use super::*;
use crate::fs::vfs::LocalVfs;

const PATCH: &str = "diff --git a/f.txt b/f.txt
index 1..2 100644
--- a/f.txt
+++ b/f.txt
@@ -1,3 +1,3 @@
 one
-two
+TWO
 three
";

#[test]
fn unified_patches_keep_headers_apart_from_changes() {
    let doc = DiffDocument::from_unified(PATCH);
    let kinds: Vec<DiffKind> = doc.lines.iter().map(|l| l.kind).collect();
    use DiffKind::*;
    assert_eq!(
        kinds,
        [
            Header, Header, Header, Header, Hunk, Context, Removed, Added, Context
        ]
    );
    assert_eq!(doc.lines[6].text, "two");
    assert_eq!(doc.lines[6].old_no, Some(2));
    assert_eq!(doc.lines[7].new_no, Some(2));
    assert_eq!(doc.lines[8].old_no, Some(3));
    assert_eq!(doc.stats(), (1, 1));
    assert_eq!(doc.lines[7].unified(), "+TWO");
    assert_eq!(doc.lines[2].unified(), "--- a/f.txt");
}

#[test]
fn a_second_file_header_after_a_hunk_is_a_header() {
    let patch = format!("{PATCH}diff --git a/g b/g\n--- a/g\n+++ b/g\n@@ -1 +1 @@\n-x\n+y\n");
    let doc = DiffDocument::from_unified(&patch);
    let minus = doc.lines.iter().find(|l| l.text == "--- a/g").unwrap();
    assert_eq!(minus.kind, DiffKind::Header);
    assert_eq!(doc.stats(), (2, 2));
}

#[test]
fn side_by_side_pairs_removed_with_added_lines() {
    let doc = diff_texts("a\nb\nc\nd\n", "a\nB\nC\nX\nd\n");
    let rows = doc.side_by_side();
    let text = |i: Option<usize>| i.map(|i| doc.lines[i].text.as_str());
    let pairs: Vec<(Option<&str>, Option<&str>)> = rows
        .iter()
        .filter_map(|row| match row {
            SideRow::Pair(l, r) => Some((text(*l), text(*r))),
            SideRow::Full(_) => None,
        })
        .collect();
    assert_eq!(
        pairs,
        [
            (Some("a"), Some("a")),
            (Some("b"), Some("B")),
            (Some("c"), Some("C")),
            (None, Some("X")),
            (Some("d"), Some("d")),
        ]
    );
    assert!(matches!(rows[0], SideRow::Full(0)));
}

#[test]
fn text_diffs_number_lines_and_stay_empty_when_equal() {
    assert!(diff_texts("same\n", "same\n").is_empty());
    let doc = diff_texts("1\n2\n", "1\n3\n");
    assert_eq!(doc.lines[0].text, "@@ -1,2 +1,2 @@");
    let removed = doc
        .lines
        .iter()
        .find(|l| l.kind == DiffKind::Removed)
        .unwrap();
    assert_eq!((removed.text.as_str(), removed.old_no), ("2", Some(2)));
}

#[test]
fn files_are_compared_as_text_or_by_hash() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, data: &[u8]| {
        let path = dir.path().join(name);
        std::fs::write(&path, data).unwrap();
        path
    };
    let a = write("a.txt", b"one\ntwo\n");
    let b = write("b.txt", b"one\n2\n");
    let side = |path| FileSide {
        vfs: &LocalVfs,
        path,
    };
    let never = || false;
    match compare_files(side(&a), side(&b), &never).unwrap() {
        FileComparison::Text(doc) => assert_eq!(doc.stats(), (1, 1)),
        other => panic!("{other:?}"),
    }
    let x = write("x.bin", b"\0\x01\x02");
    let y = write("y.bin", b"\0\x01\x02");
    let z = write("z.bin", b"\0\x01\x03");
    match compare_files(side(&x), side(&y), &never).unwrap() {
        FileComparison::Binary(b) => {
            assert!(b.identical);
            assert_eq!(b.hashes.0, b.hashes.1);
        }
        other => panic!("{other:?}"),
    }
    match compare_files(side(&x), side(&z), &never).unwrap() {
        FileComparison::Binary(b) => assert!(!b.identical),
        other => panic!("{other:?}"),
    }
    assert!(compare_files(side(&a), side(&dir.path().join("missing")), &never).is_err());
}
