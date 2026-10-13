//! The `pairee` binary and its non-interactive command line: nothing here
//! needs a terminal. Every run gets its own empty home, config, cache and
//! data folders, so the user's configuration is never read nor written.

use assert_cmd::Command;
use std::path::Path;

/// `pairee` with `args`, isolated in `home`.
fn pairee(home: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("pairee").expect("pairee binary");
    cmd.args(args)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APPDATA", home.join("appdata"))
        .env("LOCALAPPDATA", home.join("localappdata"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("RUST_LOG")
        .timeout(std::time::Duration::from_secs(60));
    cmd
}

fn stdout_of(home: &Path, args: &[&str]) -> String {
    let out = pairee(home, args).assert().success().get_output().clone();
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

#[test]
fn version_flags_print_the_crate_version() {
    let home = tempfile::tempdir().unwrap();
    let expected = format!("pairee {}\n", env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-V"] {
        assert_eq!(
            stdout_of(home.path(), &[flag]).replace("\r\n", "\n"),
            expected
        );
    }
}

#[test]
fn version_run_writes_its_log_in_the_cache_folder() {
    let home = tempfile::tempdir().unwrap();
    stdout_of(home.path(), &["--version"]);
    let logs: Vec<_> = walk(home.path())
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "app.log"))
        .collect();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let log = std::fs::read_to_string(&logs[0]).unwrap();
    assert!(log.contains("started (pid"), "{log}");
}

#[test]
fn unknown_options_fail_without_starting_the_interface() {
    let home = tempfile::tempdir().unwrap();
    for flag in ["--no-such-option", "--bogus=1"] {
        let out = pairee(home.path(), &[flag])
            .assert()
            .failure()
            .get_output()
            .clone();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(flag), "{flag}: {stderr}");
        assert!(stderr.contains("--cwd-file"), "{flag}: {stderr}");
    }
}

#[test]
fn keymap_print_renders_a_shipped_preset() {
    let home = tempfile::tempdir().unwrap();
    let text = stdout_of(home.path(), &["keymap", "print", "--preset", "norton"]);
    assert!(text.starts_with('#'), "not Markdown: {text:.200}");
    assert!(text.lines().count() > 20, "{text}");
}

#[test]
fn keymap_print_lists_presets_for_an_unknown_one() {
    let home = tempfile::tempdir().unwrap();
    let text = stdout_of(home.path(), &["keymap", "print", "--preset", "no-such"]);
    assert!(text.contains("no-such"), "{text}");
    assert!(text.contains("norton"), "{text}");
}

#[test]
fn help_flags_print_the_options_and_commands() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        &["--help"][..],
        &["-h"],
        &["/some/folder", "--help"],
        &["--help", "x"],
    ] {
        let text = stdout_of(home.path(), args);
        for word in [
            "Usage:",
            "--cwd-file",
            "--no-mouse",
            "--version",
            "plugin",
            "keymap",
        ] {
            assert!(
                text.contains(word),
                "{args:?}: {word} missing in
{text}"
            );
        }
    }
}

#[test]
fn subcommands_print_their_help() {
    let home = tempfile::tempdir().unwrap();
    for (args, expected) in [
        (&["keymap"][..], "pairee keymap print"),
        (&["keymap", "--help"], "--preset"),
        (&["plugin"], "pairee plugin <command>"),
        (&["plugin", "-h"], "check-updates"),
        (&["plugin", "install", "--help"], "install <name>"),
        (&["developer"], "pairee developer <command>"),
        (&["developer", "--help"], "submit"),
    ] {
        let text = stdout_of(home.path(), args);
        assert!(text.contains(expected), "{args:?}: {text}");
    }
}

#[test]
fn help_follows_the_configured_language() {
    let home = tempfile::tempdir().unwrap();
    // Wherever this platform keeps config.toml under the isolated home.
    for dir in [
        home.path().join("appdata").join("pairee").join("config"),
        home.path().join("config").join("pairee"),
        home.path()
            .join("Library")
            .join("Application Support")
            .join("com.pairee.Pairee"),
    ] {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.toml"),
            "language = \"Español\"
",
        )
        .unwrap();
    }
    let text = stdout_of(home.path(), &["--help"]);
    assert!(
        text.contains("Uso:") && text.contains("Opciones:"),
        "{text}"
    );
}

#[test]
fn plugin_list_works_offline_with_no_plugins() {
    let home = tempfile::tempdir().unwrap();
    let text = stdout_of(home.path(), &["plugin", "list"]);
    assert!(text.contains("(none)"), "{text}");
}

#[test]
fn plugin_commands_report_missing_arguments() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        &["plugin", "info"][..],
        &["plugin", "remove"],
        &["plugin", "bogus"],
    ] {
        let text = stdout_of(home.path(), args);
        assert!(text.contains(args[1]), "{args:?}: {text}");
        assert!(text.contains("pairee plugin <command>"), "{args:?}: {text}");
    }
}

/// Every file under `dir`.
fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}
