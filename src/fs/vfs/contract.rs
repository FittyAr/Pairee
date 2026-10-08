//! Contract tests for [`Vfs`] adapters: one generic check per capability,
//! instantiated for every adapter by [`vfs_contract!`] (no per-adapter
//! copies). Write checks assert the operation works when the adapter
//! advertises it and fails as unsupported otherwise.

use super::{Vfs, VfsEntry};
use crate::fs::du::{ScanControl, scan};
use crate::fs::list::ListOptions;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Files every fixture starts with, below its root.
pub const TREE: &[(&str, &[u8])] = &[
    ("a.txt", b"alpha"),
    (".hidden", b"h"),
    ("sub/b.txt", b"bravo"),
];

/// An adapter loaded with [`TREE`] below `root`.
pub struct Fixture {
    pub vfs: Arc<dyn Vfs>,
    pub root: PathBuf,
    /// Keeps the backing files alive for the test.
    pub _dir: tempfile::TempDir,
}

/// The local adapter on a temporary folder.
pub fn local_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    crate::fs::archive::test_fixtures::write_tree(dir.path(), TREE);
    Fixture {
        vfs: Arc::new(super::LocalVfs),
        root: dir.path().to_path_buf(),
        _dir: dir,
    }
}

/// An archive adapter on a file written by `write` (zip, tar, 7z...).
/// Writes an archive holding the given `(path, contents)` files.
pub type ArchiveWriter = fn(&Path, &[(&str, &[u8])]);

pub fn archive_fixture(name: &str, write: ArchiveWriter) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(name);
    write(&root, TREE);
    Fixture {
        vfs: Arc::new(crate::fs::archive::ArchiveVfs::open(root.clone()).unwrap()),
        root,
        _dir: dir,
    }
}

fn names(entries: &[VfsEntry]) -> Vec<String> {
    let mut names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
    names.sort();
    names
}

fn read_all(vfs: &dyn Vfs, path: &Path) -> Vec<u8> {
    let mut buf = Vec::new();
    vfs.open_read(path).unwrap().read_to_end(&mut buf).unwrap();
    buf
}

fn options(show_hidden: bool) -> ListOptions {
    ListOptions {
        show_hidden,
        case_sensitive: false,
        natural: false,
        req_admin: false,
        sort_field: crate::app::state::SortField::Name,
        sort_reverse: false,
        folder_by_ext: false,
        show_dotdot: false,
    }
}

fn assert_unsupported(result: io::Result<()>) {
    let err = result.expect_err("operation should be refused");
    assert_eq!(err.kind(), io::ErrorKind::Unsupported, "{err}");
}

pub fn lists_children(f: &Fixture) {
    let root = f.vfs.list(&f.root).unwrap();
    assert_eq!(names(&root), [".hidden", "a.txt", "sub"]);
    let sub = root.iter().find(|e| e.name == "sub").unwrap();
    assert!(sub.is_real_dir());
    assert_eq!(sub.path, f.root.join("sub"));
    assert!(root.iter().find(|e| e.name == ".hidden").unwrap().hidden);
    assert_eq!(names(&f.vfs.list(&f.root.join("sub")).unwrap()), ["b.txt"]);
}

pub fn stats_files_and_folders(f: &Fixture) {
    let file = f.vfs.stat(&f.root.join("a.txt")).unwrap();
    assert!(!file.is_dir);
    assert_eq!(file.size, 5);
    assert!(f.vfs.stat(&f.root.join("sub")).unwrap().is_dir);
    assert!(f.vfs.exists(&f.root.join("sub/b.txt")));
    assert!(!f.vfs.exists(&f.root.join("missing")));
}

pub fn reads_contents(f: &Fixture) {
    let path = f.root.join("sub/b.txt");
    assert_eq!(read_all(f.vfs.as_ref(), &path), b"bravo");
    assert_eq!(f.vfs.read_prefix(&path, 2).unwrap(), b"br");
    let store = f.vfs.open_store(&path, 1024).unwrap();
    assert_eq!(store.read_range(0, 5).unwrap(), b"bravo");
}

pub fn walks_the_tree(f: &Fixture) {
    let mut found: Vec<PathBuf> = f.vfs.walk(&f.root).into_iter().map(|e| e.path).collect();
    found.sort();
    let mut want: Vec<PathBuf> = [".hidden", "a.txt", "sub", "sub/b.txt"]
        .iter()
        .map(|p| f.root.join(p))
        .collect();
    want.sort();
    assert_eq!(found, want);
}

pub fn builds_panel_listings(f: &Fixture) {
    let visible = f
        .vfs
        .read_panel(&f.root.join("sub"), &options(false))
        .unwrap();
    let names: Vec<_> = visible.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["..", "b.txt"]);
    assert_eq!(visible[0].path, f.root);
    let root = f.vfs.read_panel(&f.root, &options(false)).unwrap();
    assert!(root.iter().all(|e| e.name != ".hidden"));
    let all = f.vfs.read_panel(&f.root, &options(true)).unwrap();
    assert!(all.iter().any(|e| e.name == ".hidden"));
}

pub fn measures_folder_sizes(f: &Fixture) {
    let ctl = ScanControl {
        cancelled: &|| false,
        progress: &|_| {},
    };
    let size = scan(f.vfs.as_ref(), &f.root, false, &ctl).size;
    assert_eq!((size.bytes, size.files, size.dirs), (11, 3, 1));
    assert!(!size.partial);
}

pub fn writes_files(f: &Fixture) {
    let path = f.root.join("sub/new.txt");
    let result = f.vfs.write_file(&path, &mut &b"fresh"[..]);
    if !f.vfs.capabilities().write {
        return assert_unsupported(result);
    }
    result.unwrap();
    assert_eq!(read_all(f.vfs.as_ref(), &path), b"fresh");
    f.vfs.write_file(&path, &mut &b"again"[..]).unwrap();
    assert_eq!(read_all(f.vfs.as_ref(), &path), b"again");
}

pub fn creates_folders(f: &Fixture) {
    let path = f.root.join("made");
    let result = f.vfs.mkdir(&path);
    if !f.vfs.capabilities().mkdir {
        return assert_unsupported(result);
    }
    result.unwrap();
    assert!(f.vfs.stat(&path).unwrap().is_dir);
}

pub fn removes_trees(f: &Fixture) {
    let result = f.vfs.remove_all(&f.root.join("sub"));
    if !f.vfs.capabilities().remove {
        return assert_unsupported(result);
    }
    result.unwrap();
    assert_eq!(names(&f.vfs.list(&f.root).unwrap()), [".hidden", "a.txt"]);
    f.vfs.remove_all(&f.root.join("a.txt")).unwrap();
    f.vfs.remove_all(&f.root.join("missing")).unwrap();
    assert_eq!(names(&f.vfs.list(&f.root).unwrap()), [".hidden"]);
}

pub fn renames_entries(f: &Fixture) {
    let to = f.root.join("z.txt");
    let result = f.vfs.rename(&f.root.join("a.txt"), &to);
    if !f.vfs.capabilities().rename {
        return assert_unsupported(result);
    }
    result.unwrap();
    assert_eq!(read_all(f.vfs.as_ref(), &to), b"alpha");
    assert!(!f.vfs.exists(&f.root.join("a.txt")));
}

pub fn changes_attributes(f: &Fixture) {
    let path = f.root.join("a.txt");
    let change = |readonly| crate::fs::attrs::AttrChange {
        mode: None,
        readonly,
    };
    let result = f.vfs.set_attributes(&path, change(true));
    if !f.vfs.capabilities().attributes {
        assert!(f.vfs.attributes(&path).is_err());
        return assert_unsupported(result);
    }
    result.unwrap();
    let attrs = f.vfs.attributes(&path).unwrap();
    assert!(attrs.readonly);
    assert_eq!(attrs.size, 5);
    f.vfs.set_attributes(&path, change(false)).unwrap();
    assert!(!f.vfs.attributes(&path).unwrap().readonly);
}

/// Instantiates every contract check for one adapter fixture.
macro_rules! vfs_contract {
    ($name:ident, $fixture:expr) => {
        mod $name {
            use $crate::fs::vfs::contract as c;

            #[test]
            fn lists_children() {
                c::lists_children(&$fixture());
            }
            #[test]
            fn stats_files_and_folders() {
                c::stats_files_and_folders(&$fixture());
            }
            #[test]
            fn reads_contents() {
                c::reads_contents(&$fixture());
            }
            #[test]
            fn walks_the_tree() {
                c::walks_the_tree(&$fixture());
            }
            #[test]
            fn builds_panel_listings() {
                c::builds_panel_listings(&$fixture());
            }
            #[test]
            fn measures_folder_sizes() {
                c::measures_folder_sizes(&$fixture());
            }
            #[test]
            fn writes_files() {
                c::writes_files(&$fixture());
            }
            #[test]
            fn creates_folders() {
                c::creates_folders(&$fixture());
            }
            #[test]
            fn removes_trees() {
                c::removes_trees(&$fixture());
            }
            #[test]
            fn renames_entries() {
                c::renames_entries(&$fixture());
            }
            #[test]
            fn changes_attributes() {
                c::changes_attributes(&$fixture());
            }
        }
    };
}

vfs_contract!(local, super::local_fixture);

mod archives {
    use super::archive_fixture;
    use crate::fs::archive::test_fixtures::{write_7z, write_tar, write_tar_gz, write_zip};
    fn zip() -> super::Fixture {
        archive_fixture("t.zip", write_zip)
    }
    fn tar() -> super::Fixture {
        archive_fixture("t.tar", write_tar)
    }
    fn tar_gz() -> super::Fixture {
        archive_fixture("t.tar.gz", write_tar_gz)
    }
    fn seven_z() -> super::Fixture {
        archive_fixture("t.7z", write_7z)
    }
    vfs_contract!(zip_archive, super::zip);
    vfs_contract!(tar_archive, super::tar);
    vfs_contract!(tar_gz_archive, super::tar_gz);
    vfs_contract!(seven_z_archive, super::seven_z);
}
