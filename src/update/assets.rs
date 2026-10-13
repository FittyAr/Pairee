//! Release asset names: the one table from Rust target triple to the
//! platform name used in release file names (`pairee-0.11.0-linux-x64-musl.tar.gz`
//! rather than `...-x86_64-unknown-linux-musl...`).
//!
//! Standard library only: `xtask` compiles this same file (`#[path]`) and
//! prints the names for the packaging scripts (`cargo xtask asset-names`).
//! `tests/release_assets.rs` keeps install.sh, install.ps1 and the WinGet
//! workflow in line with it.

/// Where every release asset is downloaded from.
pub const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/FittyAr/Pairee/releases/download/";

/// One release build.
pub struct Platform {
    /// Rust target triple of the build.
    pub triple: &'static str,
    /// Platform part of the archive name: `<os>-<arch>[-<libc>]`.
    pub slug: &'static str,
    /// Architecture part of the installer and MSIX names.
    pub arch: &'static str,
}

/// Every target the Release workflow builds.
pub const PLATFORMS: &[Platform] = &[
    Platform {
        triple: "x86_64-unknown-linux-gnu",
        slug: "linux-x64-gnu",
        arch: "x64",
    },
    Platform {
        triple: "x86_64-unknown-linux-musl",
        slug: "linux-x64-musl",
        arch: "x64",
    },
    Platform {
        triple: "aarch64-unknown-linux-musl",
        slug: "linux-arm64-musl",
        arch: "arm64",
    },
    Platform {
        triple: "x86_64-apple-darwin",
        slug: "macos-x64",
        arch: "x64",
    },
    Platform {
        triple: "aarch64-apple-darwin",
        slug: "macos-arm64",
        arch: "arm64",
    },
    Platform {
        triple: "x86_64-pc-windows-msvc",
        slug: "windows-x64",
        arch: "x64",
    },
    Platform {
        triple: "aarch64-pc-windows-msvc",
        slug: "windows-arm64",
        arch: "arm64",
    },
];

/// The release build of `triple`, if there is one.
pub fn platform(triple: &str) -> Option<&'static Platform> {
    PLATFORMS.iter().find(|p| p.triple == triple)
}

fn is_windows(triple: &str) -> bool {
    triple.contains("-windows-")
}

fn archive_ext(triple: &str) -> &'static str {
    if is_windows(triple) { "zip" } else { "tar.gz" }
}

/// `X.Y.Z` from `X.Y.Z` or a `vX.Y.Z` tag.
fn number(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// The archive of `triple`: `pairee-<X.Y.Z>-<slug>.<zip|tar.gz>`.
pub fn archive_name(version: &str, triple: &str) -> Option<String> {
    let p = platform(triple)?;
    Some(format!(
        "pairee-{}-{}.{}",
        number(version),
        p.slug,
        archive_ext(triple)
    ))
}

/// The name the archive had up to 0.10 (`pairee-v<X.Y.Z>-<triple>.<ext>`),
/// which the updater of those versions looks for.
///
/// TRANSITION: the releases up to xtask's `LEGACY_NAMES_UNTIL` (0.12.0) also
/// publish each archive under this name, and the updater falls back to it.
/// Once those releases are out, delete this function and its callers.
pub fn legacy_archive_name(version: &str, triple: &str) -> String {
    format!(
        "pairee-v{}-{triple}.{}",
        number(version),
        archive_ext(triple)
    )
}

/// The Inno Setup installer of a Windows `triple`:
/// `pairee-setup-<X.Y.Z>-<arch>.exe`.
pub fn setup_name(version: &str, triple: &str) -> Option<String> {
    let p = platform(triple).filter(|p| is_windows(p.triple))?;
    Some(format!("pairee-setup-{}-{}.exe", number(version), p.arch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_use_the_platform_and_a_bare_version() {
        assert_eq!(
            archive_name("v0.11.0", "x86_64-unknown-linux-musl").as_deref(),
            Some("pairee-0.11.0-linux-x64-musl.tar.gz")
        );
        assert_eq!(
            archive_name("0.11.0", "aarch64-pc-windows-msvc").as_deref(),
            Some("pairee-0.11.0-windows-arm64.zip")
        );
        assert_eq!(archive_name("0.11.0", "riscv64gc-unknown-linux-gnu"), None);
        assert_eq!(
            legacy_archive_name("0.11.0", "x86_64-pc-windows-msvc"),
            "pairee-v0.11.0-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            setup_name("v0.11.0", "x86_64-pc-windows-msvc").as_deref(),
            Some("pairee-setup-0.11.0-x64.exe")
        );
        assert_eq!(setup_name("0.11.0", "aarch64-apple-darwin"), None);
    }

    #[test]
    fn every_platform_has_a_distinct_name() {
        for (i, a) in PLATFORMS.iter().enumerate() {
            assert!(
                PLATFORMS[i + 1..]
                    .iter()
                    .all(|b| b.slug != a.slug && b.triple != a.triple)
            );
            assert!(a.slug.split('-').nth(1) == Some(a.arch), "{}", a.slug);
        }
    }
}
