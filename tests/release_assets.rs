//! Release asset names live in one table (src/update/assets.rs), used by the
//! updater and, through `cargo xtask asset-names`, by the packaging scripts.
//! The install scripts and the WinGet workflow cannot run Rust, so they
//! spell the platform names out; these tests keep them in line with the table.

#[path = "../src/update/assets.rs"]
mod assets;

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every name an install script asks for: the current archive name of each
/// platform it knows and (TRANSITION) the pre-0.11 name of the same build.
fn assert_script_names(script: &str, triples: &[&str], platform_var: &str, legacy_var: &str) {
    let text = read(script);
    for triple in triples {
        let platform = assets::platform(triple).expect("release target");
        assert!(
            text.contains(&format!("{platform_var}\"{}\"", platform.slug)),
            "{script} does not name the {triple} platform as {platform_var}\"{}\"",
            platform.slug
        );
        assert!(
            text.contains(&format!("{legacy_var}\"{triple}\"")),
            "{script} does not name the legacy {triple} archive"
        );
        // The script builds both names from these pieces.
        let archive = assets::archive_name("0.11.0", triple).unwrap();
        assert!(archive.contains(platform.slug), "{archive}");
        assert!(assets::legacy_archive_name("v0.11.0", triple).contains(triple));
    }
}

#[test]
fn install_sh_names_the_linux_musl_archives() {
    assert_script_names(
        "install.sh",
        &["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"],
        "PLATFORM=",
        "LEGACY_TARGET=",
    );
}

#[test]
fn install_ps1_names_the_windows_archives() {
    assert_script_names(
        "install.ps1",
        &["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"],
        "$platform = ",
        "$legacyTarget = ",
    );
}

#[test]
fn winget_workflow_picks_every_windows_installer() {
    let workflow = read(".github/workflows/winget.yml");
    let prefix = "installers-regex: '^pairee-setup-";
    assert!(
        workflow.contains(prefix),
        "winget.yml installers-regex changed"
    );
    let windows: Vec<_> = assets::PLATFORMS
        .iter()
        .filter_map(|p| assets::setup_name("0.11.0", p.triple))
        .collect();
    assert_eq!(windows.len(), 2, "{windows:?}");
    for name in windows {
        assert!(
            name.starts_with("pairee-setup-") && name.ends_with(".exe"),
            "{name}"
        );
    }
}

#[test]
fn release_workflow_builds_every_platform() {
    let workflow = read(".github/workflows/release.yml");
    for platform in assets::PLATFORMS {
        assert!(
            workflow.contains(&format!("- target: {}", platform.triple)),
            "release.yml does not build {}",
            platform.triple
        );
    }
    assert!(assets::RELEASE_DOWNLOAD_PREFIX.starts_with("https://github.com/"));
}
