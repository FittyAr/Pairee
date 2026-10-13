use super::*;

fn p(path: &str) -> PathBuf {
    PathBuf::from(path)
}

const NOW: u64 = 1_000_000_000;

#[test]
fn visits_raise_the_rank_and_recency_weighs_it() {
    let mut store = FrecencyStore::default();
    store.visit(&p("/a"), NOW);
    store.visit(&p("/a"), NOW);
    store.visit(&p("/b"), NOW);
    assert_eq!(store.folders.len(), 2);
    let a = &store.folders[0];
    assert_eq!(a.rank, 2.0);
    assert_eq!(a.score(NOW), 8.0);
    assert_eq!(a.score(NOW + 2 * HOUR), 4.0);
    assert_eq!(a.score(NOW + 2 * DAY), 1.0);
    assert_eq!(a.score(NOW + 2 * WEEK), 0.5);
}

#[test]
fn empty_query_lists_every_folder_by_score() {
    let mut store = FrecencyStore::default();
    store.visit(&p("/old"), NOW - 2 * WEEK);
    store.visit(&p("/old"), NOW - 2 * WEEK);
    store.visit(&p("/new"), NOW);
    let ranked: Vec<PathBuf> = store.ranked("", NOW).into_iter().map(|m| m.path).collect();
    assert_eq!(ranked, [p("/new"), p("/old")]);
}

#[test]
fn folders_named_like_the_last_word_come_first() {
    let mut store = FrecencyStore::default();
    for _ in 0..5 {
        store.visit(&p("/proj/src/deep/other"), NOW);
    }
    store.visit(&p("/work/proj"), NOW);
    store.visit(&p("/unrelated"), NOW);
    let ranked: Vec<PathBuf> = store
        .ranked("proj", NOW)
        .into_iter()
        .map(|m| m.path)
        .collect();
    assert_eq!(ranked, [p("/work/proj"), p("/proj/src/deep/other")]);
}

#[test]
fn every_word_must_match() {
    let mut store = FrecencyStore::default();
    store.visit(&p("/home/me/docs"), NOW);
    store.visit(&p("/srv/docs"), NOW);
    let ranked = store.ranked("me docs", NOW);
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].path, p("/home/me/docs"));
    assert!(store.ranked("zzz", NOW).is_empty());
}

#[test]
fn best_skips_folders_that_are_not_usable() {
    let mut store = FrecencyStore::default();
    store.visit(&p("/gone/docs"), NOW);
    store.visit(&p("/gone/docs"), NOW);
    store.visit(&p("/here/docs"), NOW);
    assert_eq!(
        store.best("docs", NOW, |path| path != Path::new("/gone/docs")),
        Some(p("/here/docs"))
    );
    assert_eq!(store.best("docs", NOW, |_| false), None);
}

#[test]
fn ranks_age_and_the_store_stays_bounded() {
    let mut store = FrecencyStore::default();
    for i in 0..(MAX_FOLDERS + 50) {
        store.visit(&p(&format!("/d{i}")), NOW);
    }
    assert!(store.folders.len() <= MAX_FOLDERS);
    let mut heavy = FrecencyStore::default();
    heavy.folders.push(FolderVisits {
        path: p("/busy"),
        rank: MAX_TOTAL_RANK,
        last_visit: NOW,
    });
    heavy.folders.push(FolderVisits {
        path: p("/rare"),
        rank: 1.0,
        last_visit: NOW,
    });
    heavy.visit(&p("/busy"), NOW);
    let total: f64 = heavy.folders.iter().map(|f| f.rank).sum();
    assert!(total <= MAX_TOTAL_RANK, "{total}");
    assert!(heavy.folders.iter().all(|f| f.path != p("/rare")));
}

#[test]
fn remove_forgets_a_folder() {
    let mut store = FrecencyStore::default();
    store.visit(&p("/a"), NOW);
    store.remove(&p("/a"));
    assert!(store.folders.is_empty());
}

#[test]
fn round_trips_through_its_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("frecency.toml");
    let mut store = FrecencyStore::default();
    store.visit(&p("/a"), NOW);
    store.save_to(&file).unwrap();
    assert_eq!(FrecencyStore::load_from(&file), store);
    std::fs::write(&file, "folders = 3").unwrap();
    assert_eq!(FrecencyStore::load_from(&file), FrecencyStore::default());
}
