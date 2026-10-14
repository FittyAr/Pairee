//! `cargo xtask asset-names <target-triple> <version>`: the release file
//! names of one build, as `KEY=value` lines the packaging scripts `eval`
//! (.github/scripts/package.sh and verify-dist.sh). The names come from the
//! table the updater uses (`src/update/assets.rs`).

use crate::Result;
use crate::release::version::Version;
use crate::release_assets as table;

/// The last release that also publishes each archive under its pre-0.11
/// name (with the Rust target triple, e.g. `x86_64-unknown-linux-musl`), so
/// installs of 0.10 and older could update themselves: only 0.11.0, the
/// release of the rename. Later releases carry the platform names only; an
/// install of 0.10 or older updates to 0.11.0 by hand first.
///
/// TRANSITION: delete this constant, the `LEGACY` line and its uses in
/// package.sh, `legacy_archive_name` and the fallbacks in install.sh,
/// install.ps1 and the updater.
const LEGACY_NAMES_UNTIL: Version = Version(0, 11, 0);

/// The names of the build of `triple` at `version`, one `KEY=value` per line.
/// Empty values: no Windows installers elsewhere; no legacy copy after
/// [`LEGACY_NAMES_UNTIL`].
pub fn names(triple: &str, version: &str) -> Result<String> {
    let parsed: Version = version.parse()?;
    let platform = table::platform(triple)
        .ok_or_else(|| format!("'{triple}' is not a release target (src/update/assets.rs)"))?;
    let archive = table::archive_name(version, triple).ok_or("no archive name")?;
    let stem = archive
        .strip_suffix(".zip")
        .or_else(|| archive.strip_suffix(".tar.gz"))
        .ok_or("unknown archive extension")?;
    let legacy = if parsed <= LEGACY_NAMES_UNTIL {
        table::legacy_archive_name(version, triple)
    } else {
        String::new()
    };
    let setup = table::setup_name(version, triple);
    let msix = setup
        .as_ref()
        .map(|_| format!("pairee-{parsed}-{}.msix", platform.arch))
        .unwrap_or_default();
    Ok(format!(
        "STEM={stem}\nARCHIVE={archive}\nLEGACY={legacy}\nSETUP={}\nMSIX={msix}\nARCH={}\n",
        setup.unwrap_or_default(),
        platform.arch
    ))
}

/// `cargo xtask asset-names <target-triple> <version>`.
pub fn print(args: &[String]) -> Result {
    let [triple, version] = args else {
        return Err("usage: cargo xtask asset-names <target-triple> <vX.Y.Z>".into());
    };
    print!("{}", names(triple, version)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_build_names() {
        let names = names("aarch64-pc-windows-msvc", "v0.11.0").unwrap();
        assert_eq!(
            names,
            "STEM=pairee-0.11.0-windows-arm64\n\
             ARCHIVE=pairee-0.11.0-windows-arm64.zip\n\
             LEGACY=pairee-v0.11.0-aarch64-pc-windows-msvc.zip\n\
             SETUP=pairee-setup-0.11.0-arm64.exe\n\
             MSIX=pairee-0.11.0-arm64.msix\n\
             ARCH=arm64\n"
        );
    }

    #[test]
    fn legacy_copies_stop_after_the_transition() {
        let names = names("x86_64-unknown-linux-musl", "0.11.1").unwrap();
        assert!(names.contains("ARCHIVE=pairee-0.11.1-linux-x64-musl.tar.gz\n"));
        assert!(names.contains("LEGACY=\nSETUP=\nMSIX=\n"), "{names}");
        let last = super::names("x86_64-unknown-linux-musl", "0.11.0").unwrap();
        assert!(last.contains("LEGACY=pairee-v0.11.0-x86_64-unknown-linux-musl.tar.gz\n"));
    }

    #[test]
    fn unknown_targets_and_versions_are_errors() {
        assert!(names("riscv64gc-unknown-linux-gnu", "0.11.0").is_err());
        assert!(names("x86_64-pc-windows-msvc", "latest").is_err());
    }
}
