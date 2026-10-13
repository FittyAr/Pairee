//! Filesystem engines used through the library surface (their screens come
//! later): the duplicate finder and the remote backend factory.

use pairee::dupes::{DupeControl, DupeOptions, find_duplicates};
use pairee::vfs::LocalVfs;
use std::fs;

#[test]
fn duplicate_finder_groups_equal_files() {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in [("a", "same"), ("b", "same"), ("c", "other")] {
        fs::write(dir.path().join(name), body).unwrap();
    }
    let ctl = DupeControl {
        cancelled: &|| false,
        progress: &|_| {},
    };
    let roots = [dir.path().to_path_buf()];
    let report = find_duplicates(&LocalVfs, &roots, &DupeOptions::default(), &ctl);
    assert_eq!(report.groups.len(), 1);
    assert_eq!(report.groups[0].files.len(), 2);
    assert_eq!(report.groups[0].wasted(), 4);
    assert!(!report.partial);
}

#[test]
fn remote_factory_refuses_unknown_schemes() {
    let err = pairee::remote::connect("gopher://example.org/").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    assert!(pairee::remote::Scheme::parse("ftps").is_some());
}
