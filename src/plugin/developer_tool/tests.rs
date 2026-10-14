//! Publishing checks, lint report and progress reporting of the plugin
//! developer tool, on plugin folders built in a temporary directory.

use super::*;
use crate::config::localization::t;
use std::path::Path;
use tempfile::TempDir;

/// A plugin folder with `manifest` (after the name and version lines).
fn plugin(manifest: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let text = format!("name = \"demo\"\nversion = \"1.0.0\"\n{manifest}");
    std::fs::write(dir.path().join("manifest.toml"), text).unwrap();
    dir
}

fn png(path: &Path, (width, height): (u32, u32)) {
    image::RgbImage::new(width, height).save(path).expect("png");
}

#[test]
fn publishing_needs_a_manifest_an_icon_of_the_right_size_and_screenshots() {
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        validate_for_publish(empty.path()),
        Err(t("plugin_dev_submit_no_manifest"))
    );

    let dir = plugin("icon = \"icon.png\"\nscreenshots = [\"shot.png\"]\n");
    assert_eq!(
        validate_for_publish(dir.path()),
        Err(t("plugin_dev_publish_no_icon")),
        "the icon file is missing"
    );

    png(&dir.path().join("icon.png"), (100, 100));
    let wrong = t("plugin_dev_publish_icon_invalid_size")
        .replace("{w}", "100")
        .replace("{h}", "100");
    assert_eq!(validate_for_publish(dir.path()), Err(wrong));

    png(&dir.path().join("icon.png"), (256, 256));
    let missing = t("plugin_dev_publish_screenshot_not_found").replace("{}", "shot.png");
    assert_eq!(validate_for_publish(dir.path()), Err(missing));

    png(&dir.path().join("shot.png"), (320, 200));
    let small = t("plugin_dev_publish_screenshot_invalid_size")
        .replace("{}", "shot.png")
        .replace("{w}", "320")
        .replace("{h}", "200");
    assert_eq!(validate_for_publish(dir.path()), Err(small));

    png(&dir.path().join("shot.png"), (640, 480));
    assert_eq!(
        validate_for_publish(dir.path()),
        Err(t("plugin_dev_publish_no_main")),
        "main.lua is required"
    );
    std::fs::write(dir.path().join("main.lua"), "return {}").unwrap();
    assert_eq!(validate_for_publish(dir.path()), Ok(()));
}

/// A valid plugin folder (icon, screenshot, main.lua).
fn publishable() -> TempDir {
    let dir = plugin("icon = \"icon.png\"\nscreenshots = [\"shot.png\"]\n");
    png(&dir.path().join("icon.png"), (256, 256));
    png(&dir.path().join("shot.png"), (640, 480));
    std::fs::write(dir.path().join("main.lua"), "return {}").unwrap();
    dir
}

#[test]
fn icon_and_screenshots_must_be_files_inside_the_plugin_folder() {
    let dir = publishable();
    let outside = tempfile::tempdir().unwrap();
    png(&outside.path().join("big.png"), (512, 512));
    let manifest = |icon: &str| {
        format!(
            "name = \"demo\"\nversion = \"1.0.0\"\nicon = {icon:?}\nscreenshots = [\"shot.png\"]\n"
        )
    };
    for icon in [
        outside
            .path()
            .join("big.png")
            .to_string_lossy()
            .into_owned(),
        format!(
            "../{}/big.png",
            outside.path().file_name().unwrap().to_string_lossy()
        ),
    ] {
        std::fs::write(dir.path().join("manifest.toml"), manifest(&icon)).unwrap();
        let bad = t("plugin_dev_publish_bad_path").replace("{}", &icon);
        let result = validate_for_publish(dir.path());
        assert!(
            result == Err(bad.clone()) || result == Err(t("plugin_dev_publish_no_icon")),
            "{icon}: {result:?}"
        );
    }
    // A hidden icon is not published, so it cannot be the icon.
    png(&dir.path().join(".icon.png"), (256, 256));
    std::fs::write(dir.path().join("manifest.toml"), manifest(".icon.png")).unwrap();
    assert_eq!(
        validate_for_publish(dir.path()),
        Err(t("plugin_dev_publish_bad_path").replace("{}", ".icon.png"))
    );
}

#[test]
fn file_names_the_installer_would_refuse_are_refused_before_publishing() {
    let dir = publishable();
    std::fs::write(dir.path().join("aux.lua"), "x").unwrap();
    let err = validate_for_publish(dir.path()).unwrap_err();
    assert!(err.contains("aux.lua"), "{err}");
    std::fs::remove_file(dir.path().join("aux.lua")).unwrap();
    std::fs::write(dir.path().join("trailing."), "x").ok();
    // (Windows cannot create that name; the check is exercised on Unix.)
    if dir.path().join("trailing.").exists() {
        assert!(validate_for_publish(dir.path()).is_err());
    }
}

#[test]
fn a_single_file_over_the_limit_is_refused() {
    use crate::plugin::updater::http::MAX_FILE_BYTES;
    let dir = publishable();
    let big = std::fs::File::create(dir.path().join("big.bin")).unwrap();
    big.set_len(MAX_FILE_BYTES as u64 + 1).unwrap();
    let err = validate_for_publish(dir.path()).unwrap_err();
    assert!(err.contains("big.bin"), "{err}");
}

#[test]
fn publishing_without_screenshots_is_refused() {
    let dir = plugin("icon = \"icon.png\"\n");
    png(&dir.path().join("icon.png"), (512, 512));
    assert_eq!(
        validate_for_publish(dir.path()),
        Err(t("plugin_dev_publish_no_screenshots"))
    );
}

#[test]
fn validation_reports_each_screenshot_it_checks() {
    use super::package::validate::validate_for_publish_with_progress;
    let dir = plugin("icon = \"icon.png\"\nscreenshots = [\"a.png\", \"\", \"b.png\"]\n");
    png(&dir.path().join("icon.png"), (256, 256));
    png(&dir.path().join("a.png"), (800, 600));
    png(&dir.path().join("b.png"), (800, 600));
    std::fs::write(dir.path().join("main.lua"), "return {}").unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    validate_for_publish_with_progress(dir.path(), Some(tx)).expect("valid plugin");
    let steps: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|p| p.current.zip(p.total))
        .collect();
    assert_eq!(steps, [(1, 2), (2, 2)], "blank entries are skipped");
}

#[test]
fn lint_requires_the_manifest_a_default_language_and_main_lua() {
    let empty = tempfile::tempdir().unwrap();
    assert!(lint_with_progress(empty.path(), None).is_err());

    let no_language = plugin("");
    let err = lint_with_progress(no_language.path(), None).unwrap_err();
    assert_eq!(err.to_string(), t("plugin_dev_lint_err_default_lang"));

    let no_code = plugin("default_language = \"en\"\n");
    let err = lint_with_progress(no_code.path(), None).unwrap_err();
    assert_eq!(err.to_string(), t("plugin_dev_lint_err_lua").trim());
}

#[test]
fn lint_flags_unsafe_calls_of_untrusted_plugins_only() {
    let code = "os.execute(\"ls\")\nlocal f = io.open(\"x\")\n";
    let untrusted = plugin("default_language = \"en\"\n");
    std::fs::write(untrusted.path().join("main.lua"), code).unwrap();
    let report = lint_with_progress(untrusted.path(), None).unwrap();
    assert!(report.contains(&t("plugin_dev_lint_warn_unsafe").replace("{}", "os.execute")));
    assert!(report.contains(&t("plugin_dev_lint_warn_unsafe").replace("{}", "io.open")));
    assert!(report.contains(&t("plugin_dev_lint_warn_total").replace("{}", "2")));

    let trusted = plugin("default_language = \"en\"\nrequires_trust = true\n");
    std::fs::write(trusted.path().join("main.lua"), code).unwrap();
    let report = lint_with_progress(trusted.path(), None).unwrap();
    assert!(report.contains(&t("plugin_dev_lint_ok")), "{report}");
}

#[test]
fn progress_goes_to_the_channel_and_nowhere_without_one() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let sender = Some(tx);
    progress_status(&sender, "one");
    progress_progress(&sender, "two", 3, 4);
    progress_finish(sender, None, Some("failed".into()));
    let updates: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(updates.len(), 3);
    assert_eq!(updates[0].status, "one");
    assert_eq!((updates[1].current, updates[1].total), (Some(3), Some(4)));
    assert!(updates[2].done && updates[2].error.as_deref() == Some("failed"));
    // No receiver side (the CLI): nothing happens.
    progress_status(&None, "ignored");
    progress_finish(None, Some("ignored".into()), None);
}

#[test]
fn only_the_pairee_repository_with_the_branch_is_a_local_source() {
    for url in [
        "https://github.com/FittyAr/Pairee",
        "https://github.com/FittyAr/Pairee.git",
        "git@github.com:FittyAr/Pairee.git",
        "https://github.com/fittyar/pairee/",
    ] {
        assert!(super::is_pairee_url(url), "{url}");
    }
    for url in [
        "https://github.com/someone/Pairee-fork",
        "https://example.com/FittyAr/Pairee.git",
        "https://github.com/FittyAr/Other.git",
        "",
    ] {
        assert!(!super::is_pairee_url(url), "{url}");
    }

    // The user's home or any unrelated project is not a Pairee checkout.
    let unrelated = tempfile::tempdir().unwrap();
    git2::Repository::init(unrelated.path()).unwrap();
    assert!(!super::is_pairee_repo(unrelated.path(), "plugin-registry"));

    // The right remote but without the branch is not enough either.
    let checkout = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(checkout.path()).unwrap();
    repo.remote("origin", "https://github.com/FittyAr/Pairee.git")
        .unwrap();
    assert!(!super::is_pairee_repo(checkout.path(), "plugin-registry"));
    let sig = git2::Signature::now("t", "t@example.com").unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let id = repo
        .commit(Some("HEAD"), &sig, &sig, "x", &tree, &[])
        .unwrap();
    repo.branch("plugin-registry", &repo.find_commit(id).unwrap(), false)
        .unwrap();
    assert!(super::is_pairee_repo(checkout.path(), "plugin-registry"));
}
