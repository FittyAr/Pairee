use super::mask::DateValues;
use super::preview::Issue;
use super::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub(super) const CS: TargetFs = TargetFs {
    windows_names: false,
    case_insensitive: false,
};
pub(super) const WIN: TargetFs = TargetFs {
    windows_names: true,
    case_insensitive: true,
};

fn src(name: &str) -> RenameSource {
    RenameSource {
        path: Path::new("dir").join("photos").join(name),
        is_dir: false,
        modified: None,
    }
}

fn rules(name: &str, ext: &str) -> RenameRules {
    RenameRules {
        name_mask: name.into(),
        ext_mask: ext.into(),
        ..RenameRules::default()
    }
}

fn rename(rules: &RenameRules, name: &str, index: usize) -> String {
    rules.compile().unwrap().new_name(&src(name), index)
}

// ── Masks ────────────────────────────────────────────────────────────────

#[test]
fn default_rules_keep_the_name() {
    let r = RenameRules::default();
    assert_eq!(rename(&r, "report.final.pdf", 0), "report.final.pdf");
    assert_eq!(rename(&r, "README", 0), "README");
    assert_eq!(rename(&r, ".bashrc", 0), ".bashrc");
}

#[test]
fn name_ranges() {
    assert_eq!(rename(&rules("[N2-4]", "[E]"), "abcdef.txt", 0), "bcd.txt");
    assert_eq!(rename(&rules("[N3]", ""), "abcdef.txt", 0), "c");
    assert_eq!(rename(&rules("[N3-]", "[E]"), "abcdef.txt", 0), "cdef.txt");
    assert_eq!(rename(&rules("[N2,3]", "[E]"), "abcdef.txt", 0), "bcd.txt");
    assert_eq!(rename(&rules("[N1-99]", "[E1]"), "ab.txt", 0), "ab.t");
    assert_eq!(rename(&rules("x[N9]", "[E]"), "ab.txt", 0), "x.txt");
    // Multi-byte characters are sliced by character, not byte.
    assert_eq!(
        rename(&rules("[N2-3]", "[E]"), "日本語ファイル.md", 0),
        "本語.md"
    );
}

#[test]
fn parent_and_literals() {
    assert_eq!(rename(&rules("[P]_[N]", "[E]"), "a.jpg", 0), "photos_a.jpg");
    assert_eq!(rename(&rules("[P1-3]-[N]", "[E]"), "a.jpg", 0), "pho-a.jpg");
    // Unknown, malformed or unclosed placeholders are copied literally.
    assert_eq!(
        rename(&rules("[X][N0][N", "[E]"), "a.jpg", 0),
        "[X][N0][N.jpg"
    );
    assert_eq!(rename(&rules("[N1-x]]", ""), "a.jpg", 0), "[N1-x]]");
}

#[test]
fn empty_extension_mask_drops_the_dot() {
    assert_eq!(rename(&rules("[N]", ""), "a.txt", 0), "a");
    assert_eq!(rename(&rules("[N]", "bak"), "noext", 0), "noext.bak");
    assert_eq!(rename(&rules("[N]", "[E]"), "noext", 0), "noext");
}

#[test]
fn folders_have_no_extension() {
    let folder = RenameSource {
        path: PathBuf::from("my.folder"),
        is_dir: true,
        modified: None,
    };
    let compiled = rules("[N]-x", "[E]").compile().unwrap();
    assert_eq!(compiled.new_name(&folder, 0), "my.folder-x");
}

#[test]
fn date_placeholders_use_the_modification_time() {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let date = DateValues::from_system_time(time);
    let source = RenameSource {
        modified: Some(time),
        ..src("a.jpg")
    };
    let compiled = rules("[Y]-[M]-[D]_[h][m][s]", "[E]").compile().unwrap();
    let expected = format!(
        "{:04}-{:02}-{:02}_{:02}{:02}{:02}.jpg",
        date.year, date.month, date.day, date.hour, date.minute, date.second
    );
    assert_eq!(compiled.new_name(&source, 0), expected);
    // Unknown date: placeholders are empty.
    assert_eq!(compiled.new_name(&src("a.jpg"), 0), "--_.jpg");
}

// ── Counter ──────────────────────────────────────────────────────────────

#[test]
fn counter_start_step_and_padding() {
    let mut r = rules("IMG_[C]", "[E]");
    r.counter = Counter {
        start: 5,
        step: 10,
        digits: 3,
    };
    assert_eq!(rename(&r, "x.jpg", 0), "IMG_005.jpg");
    assert_eq!(rename(&r, "x.jpg", 2), "IMG_025.jpg");
    assert_eq!(Counter::default().format(9), "10");
    let down = Counter {
        start: 1,
        step: -1,
        digits: 2,
    };
    assert_eq!(down.format(3), "-02");
    let huge = Counter {
        digits: 99,
        ..Counter::default()
    };
    assert_eq!(huge.format(0).len(), Counter::MAX_DIGITS);
}

// ── Search & replace, case ───────────────────────────────────────────────

#[test]
fn plain_search_is_literal() {
    let mut r = RenameRules {
        search: "a.b".into(),
        replace: "$1".into(),
        ..RenameRules::default()
    };
    assert_eq!(rename(&r, "a.b-axb.txt", 0), "$1-axb.txt");
    r.search = "X".into();
    r.replace = "y".into();
    assert_eq!(rename(&r, "xX.txt", 0), "xy.txt");
    r.ignore_case = true;
    assert_eq!(rename(&r, "xX.txt", 0), "yy.tyt");
}

#[test]
fn regex_search_with_groups() {
    let mut r = RenameRules {
        regex: true,
        search: r"(\d+)-(\d+)".into(),
        replace: "${2}_$1".into(),
        ..RenameRules::default()
    };
    assert_eq!(rename(&r, "shot 12-34.png", 0), "shot 34_12.png");
    r.search = "^IMG".into();
    r.replace = "photo".into();
    r.ignore_case = true;
    assert_eq!(rename(&r, "img_1.jpg", 0), "photo_1.jpg");
}

#[test]
fn invalid_regex_is_reported() {
    let mut r = RenameRules {
        regex: true,
        search: "(unclosed".into(),
        ..RenameRules::default()
    };
    assert!(r.compile().is_err());
    r.regex = false;
    assert!(r.compile().is_ok());
}

#[test]
fn case_transforms() {
    let mut r = RenameRules {
        case: CaseMode::Upper,
        ..RenameRules::default()
    };
    assert_eq!(rename(&r, "Hello world.txt", 0), "HELLO WORLD.TXT");
    r.case = CaseMode::Lower;
    assert_eq!(rename(&r, "Hello World.TXT", 0), "hello world.txt");
    r.case = CaseMode::Title;
    assert_eq!(
        rename(&r, "hELLO wORLD-foo_bar.txt", 0),
        "Hello World-Foo_Bar.Txt"
    );
    assert_eq!(CaseMode::Unchanged.cycle(false), CaseMode::Title);
    assert_eq!(CaseMode::Title.cycle(true), CaseMode::Unchanged);
    assert_eq!(CaseMode::Lower.cycle(true), CaseMode::Upper);
}

// ── Name validity ────────────────────────────────────────────────────────

#[test]
fn invalid_names_per_platform() {
    for bad in ["", ".", "..", "a/b", "nul\0"] {
        assert!(!CS.is_valid_name(bad), "{bad:?}");
        assert!(!WIN.is_valid_name(bad), "{bad:?}");
    }
    for bad in [
        "a:b",
        "a?",
        "trailing.",
        "trailing ",
        "CON",
        "nul.txt",
        "com1",
        "LPT9.log",
    ] {
        assert!(CS.is_valid_name(bad), "{bad:?} is fine on POSIX");
        assert!(!WIN.is_valid_name(bad), "{bad:?} is invalid on Windows");
    }
    for ok in ["COM10", "console.txt", "LPT", "a b.c"] {
        assert!(WIN.is_valid_name(ok), "{ok:?}");
    }
}

// ── Preview & conflicts ──────────────────────────────────────────────────

pub(super) fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn preview(sources: &[&str], siblings: &[&str], r: &RenameRules, fs: TargetFs) -> Preview {
    let sources: Vec<RenameSource> = sources.iter().map(|n| src(n)).collect();
    Preview::build(&sources, &r.compile().unwrap(), &names(siblings), fs)
}

#[test]
fn preview_flags_duplicates_existing_invalid_and_empty() {
    let r = rules("x", "[E]");
    let p = preview(
        &["a.txt", "b.txt", "c.md"],
        &["a.txt", "b.txt", "c.md"],
        &r,
        CS,
    );
    let issues: Vec<_> = p.rows.iter().map(|r| r.issue).collect();
    assert_eq!(
        issues,
        [Some(Issue::Duplicate), Some(Issue::Duplicate), None]
    );
    assert_eq!(p.conflicts(), 2);
    assert!(!p.is_runnable());

    let p = preview(&["a.txt"], &["a.txt", "x.txt"], &r, CS);
    assert_eq!(p.rows[0].issue, Some(Issue::Exists));

    let p = preview(&["a.txt"], &["a.txt"], &rules("", ""), CS);
    assert_eq!(p.rows[0].issue, Some(Issue::Empty));

    let p = preview(&["a.txt"], &["a.txt"], &rules("a/b", ""), CS);
    assert_eq!(p.rows[0].issue, Some(Issue::InvalidName));
}

/// `[C]` counting from `start` by `step`: with sources `1`, `2`, start 2 and
/// step -1 this swaps the two names.
pub(super) fn counter_rules(start: i64, step: i64) -> RenameRules {
    RenameRules {
        name_mask: "[C]".into(),
        ext_mask: "[E]".into(),
        counter: Counter {
            start,
            step,
            digits: 1,
        },
        ..RenameRules::default()
    }
}

#[test]
fn renaming_onto_another_source_is_not_a_conflict() {
    let swap = counter_rules(2, -1);
    let p = preview(&["1.txt", "2.txt"], &["1.txt", "2.txt"], &swap, CS);
    assert_eq!(p.rows[0].new, "2.txt");
    assert_eq!(p.rows[1].new, "1.txt");
    assert!(p.is_runnable());
    // ...but onto a file that is not part of the batch it is.
    let p = preview(&["1.txt"], &["1.txt", "2.txt"], &swap, CS);
    assert_eq!(p.rows[0].issue, Some(Issue::Exists));
}

#[test]
fn case_only_rename_on_case_insensitive_fs() {
    let r = RenameRules {
        case: CaseMode::Upper,
        ..RenameRules::default()
    };
    let p = preview(&["a.txt"], &["a.txt"], &r, WIN);
    assert_eq!(p.rows[0].new, "A.TXT");
    assert_eq!(p.rows[0].issue, None);
    assert!(p.is_runnable());
    // On a case-insensitive fs two names differing only in case collide.
    let p = preview(
        &["a.txt", "b.txt"],
        &["a.txt", "b.txt"],
        &rules("X", "[E]"),
        WIN,
    );
    assert_eq!(p.conflicts(), 2);
}

#[test]
fn unchanged_rows_are_not_moved() {
    let r = RenameRules::default();
    let sources = [src("a.txt"), src("b.txt")];
    let p = Preview::build(
        &sources,
        &r.compile().unwrap(),
        &names(&["a.txt", "b.txt"]),
        CS,
    );
    assert_eq!(p.changes(), 0);
    assert!(!p.is_runnable());
    assert!(p.moves(&sources).is_empty());
}
