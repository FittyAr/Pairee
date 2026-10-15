use super::*;

fn v(text: &str) -> toml::Value {
    parse(text).unwrap()
}

#[test]
fn recent_lists_keep_both_instances_pushes() {
    let base = ["a", "b"];
    // This instance pushed "c"; another one pushed "d" meanwhile.
    let merged = merge_recent(&base, &["c", "a", "b"], &["d", "a", "b"], 10);
    assert_eq!(merged, ["c", "d", "a", "b"]);
}

#[test]
fn recent_lists_drop_what_this_instance_removed_only() {
    let base = ["a", "b", "c"];
    let merged = merge_recent(&base, &["a", "c"], &["d", "a", "b", "c"], 10);
    assert_eq!(merged, ["d", "a", "c"]);
    // A cleared list keeps what the other instance added since.
    assert_eq!(merge_recent(&base, &[], &["d", "a", "b", "c"], 10), ["d"]);
}

#[test]
fn recent_lists_move_a_repushed_entry_to_the_front_and_respect_the_cap() {
    let base = ["a", "b", "c"];
    let merged = merge_recent(&base, &["c", "a", "b"], &["x", "a", "b", "c"], 3);
    assert_eq!(merged, ["c", "x", "a"]);
}

#[test]
fn toml_merge_keeps_the_keys_the_other_instance_changed() {
    let base = v("a = 1\nb = 1\n[t]\nx = 1\ny = 1\n");
    let mine = v("a = 2\nb = 1\n[t]\nx = 2\ny = 1\n");
    let disk = v("a = 1\nb = 3\nnew = true\n[t]\nx = 1\ny = 3\n");
    let merged = merge_toml(&base, &mine, &disk);
    assert_eq!(merged, v("a = 2\nb = 3\nnew = true\n[t]\nx = 2\ny = 3\n"));
}

#[test]
fn toml_merge_applies_removals_and_replaces_changed_arrays_whole() {
    let base = v("gone = 1\nlist = [1, 2]\nkept = 1\n");
    let mine = v("list = [1, 2, 3]\nkept = 1\n");
    let disk = v("gone = 1\nlist = [9]\nkept = 5\n");
    assert_eq!(
        merge_toml(&base, &mine, &disk),
        v("list = [1, 2, 3]\nkept = 5\n")
    );
}

#[test]
fn a_toml_save_merges_only_when_the_file_changed_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let first = TomlBase::default();
    let second = TomlBase::default();
    let start = "theme = \"dark\"\nlanguage = \"en\"\n";
    std::fs::write(&path, start).unwrap();
    first.remember(start);
    second.remember(start);

    // Unchanged on disk: written exactly as given (formatting kept).
    let first_text = "theme = \"light\"\nlanguage = \"en\"\n";
    first.save(&path, first_text).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), first_text);

    // The second instance changed only the language: the first's theme stays.
    second
        .save(&path, "theme = \"dark\"\nlanguage = \"es\"\n")
        .unwrap();
    let on_disk = v(&std::fs::read_to_string(&path).unwrap());
    assert_eq!(on_disk, v("theme = \"light\"\nlanguage = \"es\"\n"));

    // And the first saving again (no new change) keeps the second's language.
    first.save(&path, first_text).unwrap();
    let on_disk = v(&std::fs::read_to_string(&path).unwrap());
    assert_eq!(on_disk, v("theme = \"light\"\nlanguage = \"es\"\n"));
}

#[test]
fn a_toml_save_without_a_base_writes_the_whole_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("k.toml");
    std::fs::write(&path, "old = 1\n").unwrap();
    TomlBase::default().save(&path, "new = 2\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new = 2\n");
}

#[test]
fn saving_over_an_unparsable_file_sets_it_aside() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("k.toml");
    std::fs::write(&path, "broken = [\n").unwrap();
    TomlBase::default().save(&path, "new = 2\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new = 2\n");
    let bak = dir.path().join("k.toml.bak");
    assert_eq!(std::fs::read_to_string(bak).unwrap(), "broken = [\n");
}

#[derive(Debug, Default, Serialize, serde::Deserialize)]
struct Items {
    items: Vec<u32>,
}

#[test]
fn concurrent_merge_saves_lose_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = std::sync::Arc::new(dir.path().join("items.toml"));
    let writers: Vec<_> = (0..8)
        .map(|i| {
            let path = std::sync::Arc::clone(&path);
            std::thread::spawn(move || {
                for j in 0..5 {
                    merge_save(&path, |disk: Option<Items>| {
                        let mut items = disk.unwrap_or_default();
                        items.items.push(i * 100 + j);
                        items
                    })
                    .unwrap();
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let saved: Items = toml_store::load(&path).unwrap();
    assert_eq!(saved.items.len(), 40, "every save kept the others'");
    assert!(lock_path(&path).exists());
}
