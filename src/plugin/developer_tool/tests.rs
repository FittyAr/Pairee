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
    assert_eq!(validate_for_publish(dir.path()), Ok(()));
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
