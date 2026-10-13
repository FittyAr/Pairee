//! WinGet manifests: one folder per version under [`WINGET_DIR`], moved to
//! the new version and rewritten at each release. The installer hashes are
//! only known once the Release workflow has built the installers:
//! `cargo xtask winget-manifests` fills them in for the WinGet workflow.

use std::fs;
use std::path::{Path, PathBuf};

use super::version::Version;
use crate::release_assets as assets;
use crate::text::TextFile;
use crate::{Result, changelog};

pub const WINGET_DIR: &str = "manifests/f/FittyAr/Pairee";

/// The manifest folder of `current`, or else the newest version folder
/// (never an arbitrary directory entry). `None` when there is none.
pub fn source_dir(current: Version) -> Option<PathBuf> {
    let base = Path::new(WINGET_DIR);
    let exact = base.join(current.to_string());
    if exact.is_dir() {
        return Some(exact);
    }
    newest_version_dir(base)
}

fn newest_version_dir(base: &Path) -> Option<PathBuf> {
    fs::read_dir(base)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let version: Version = e.file_name().to_str()?.parse().ok()?;
            Some((version, e.path()))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, path)| path)
}

/// The `*.yaml` manifests directly in `dir`, sorted.
pub fn manifests(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "yaml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Moves `from` to the folder of `new` and updates the version, installer
/// URLs and release-notes URL in its manifests. Returns the new folder.
pub fn bump(from: &Path, new: Version) -> Result<PathBuf> {
    let to = Path::new(WINGET_DIR).join(new.to_string());
    if from != to {
        if to.exists() {
            return Err(format!("{} already exists", to.display()).into());
        }
        fs::rename(from, &to)?;
    }
    for path in manifests(&to)? {
        let mut manifest = TextFile::load(&path)?;
        for line in &mut manifest.lines {
            let key = line.trim_start();
            if key.starts_with("PackageVersion:") {
                *line = format!("PackageVersion: {new}");
            } else if key.starts_with("ReleaseDate:") {
                *line = format!("ReleaseDate: {}", changelog::today_utc());
            } else if key.starts_with("InstallerUrl:") || key.starts_with("ReleaseNotesUrl:") {
                *line = replace_versions(line, new);
            }
        }
        manifest.save()?;
    }
    Ok(to)
}

/// `cargo xtask winget-manifests <vX.Y.Z> <dist-dir> <out-dir>`: writes the
/// manifests of the release to `out-dir`, each installer with its download
/// URL and the SHA-256 published next to it (`<dist-dir>/<installer>.sha256`).
pub fn manifests_for_release(args: &[String]) -> Result {
    let [tag, dist, out] = args else {
        return Err("usage: cargo xtask winget-manifests <vX.Y.Z> <dist-dir> <out-dir>".into());
    };
    let version: Version = tag.parse()?;
    let source = Path::new(WINGET_DIR).join(version.to_string());
    if !source.is_dir() {
        return Err(format!("no WinGet manifests for {version} in {}", source.display()).into());
    }
    fs::create_dir_all(out)?;
    for path in manifests(&source)? {
        let mut text = fs::read_to_string(&path)?;
        if path.to_string_lossy().ends_with(".installer.yaml") {
            text = fill_installers(&text, version, Path::new(dist))?;
        }
        let name = path.file_name().ok_or("manifest without a file name")?;
        fs::write(Path::new(out).join(name), text)?;
    }
    Ok(())
}

/// Sets `InstallerUrl` and `InstallerSha256` of every `- Architecture: <arch>`
/// entry to the Windows installer of that architecture.
fn fill_installers(text: &str, version: Version, dist: &Path) -> Result<String> {
    let tag = format!("v{version}");
    let mut setup: Option<String> = None;
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let entry = line.trim_start().trim_start_matches("- ");
        let lead = &line[..line.len() - entry.len()];
        let mut line = line.to_string();
        if let Some(arch) = entry.strip_prefix("Architecture:") {
            setup = Some(installer_for(arch.trim(), &tag)?);
        } else if entry.starts_with("InstallerUrl:") || entry.starts_with("InstallerSha256:") {
            let name = setup
                .as_deref()
                .ok_or("installer field before its Architecture")?;
            line = if entry.starts_with("InstallerUrl:") {
                format!(
                    "{lead}InstallerUrl: {}{tag}/{name}",
                    assets::RELEASE_DOWNLOAD_PREFIX
                )
            } else {
                format!("{lead}InstallerSha256: {}", published_sha256(dist, name)?)
            };
        }
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

/// The Inno Setup installer of the Windows build for `arch`.
fn installer_for(arch: &str, tag: &str) -> Result<String> {
    assets::PLATFORMS
        .iter()
        .filter(|p| p.arch == arch)
        .find_map(|p| assets::setup_name(tag, p.triple))
        .ok_or_else(|| format!("no Windows installer for architecture '{arch}'").into())
}

/// The digest in `<dist>/<name>.sha256` (`<hex>` or `<hex>  <file>`).
fn published_sha256(dist: &Path, name: &str) -> Result<String> {
    let path = dist.join(format!("{name}.sha256"));
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let digest = text.split_whitespace().next().unwrap_or_default();
    if digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(digest.to_ascii_uppercase())
    } else {
        Err(format!("{}: not a SHA-256 digest", path.display()).into())
    }
}

/// Replaces every `X.Y.Z` in `line` with `new` (a leading `v` is kept).
fn replace_versions(line: &str, new: Version) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let len = tail
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(tail.len());
        let token = tail[..len].trim_end_matches('.');
        if token.parse::<Version>().is_ok() {
            out.push_str(&new.to_string());
        } else {
            out.push_str(token);
        }
        rest = &tail[token.len()..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_get_the_new_version() {
        let new = Version(1, 2, 3);
        assert_eq!(
            replace_versions(
                "    InstallerUrl: https://github.com/FittyAr/Pairee/releases/download/v0.9.0/pairee-setup-0.9.0-x64.exe",
                new
            ),
            "    InstallerUrl: https://github.com/FittyAr/Pairee/releases/download/v1.2.3/pairee-setup-1.2.3-x64.exe"
        );
        assert_eq!(
            replace_versions("schema.1.12.0.json", new),
            "schema.1.2.3.json"
        );
        assert_eq!(replace_versions("x64 and 2026", new), "x64 and 2026");
    }

    #[test]
    fn release_installers_get_their_url_and_hash() {
        let dist = std::env::temp_dir().join(format!("xtask-winget-dist-{}", std::process::id()));
        fs::create_dir_all(&dist).unwrap();
        let x64 = "ab".repeat(32);
        fs::write(
            dist.join("pairee-setup-1.2.3-x64.exe.sha256"),
            format!("{x64}\n"),
        )
        .unwrap();
        fs::write(
            dist.join("pairee-setup-1.2.3-arm64.exe.sha256"),
            "cd".repeat(32),
        )
        .unwrap();
        let template = "Installers:\n  - Architecture: x64\n    InstallerUrl: old\n    InstallerSha256: 0\n  - Architecture: arm64\n    InstallerUrl: old\n    InstallerSha256: 0\nManifestType: installer\n";
        let filled = fill_installers(template, Version(1, 2, 3), &dist);
        let missing = fill_installers(template, Version(1, 2, 4), &dist);
        fs::remove_dir_all(&dist).unwrap();
        let filled = filled.unwrap();
        assert!(filled.contains("    InstallerUrl: https://github.com/FittyAr/Pairee/releases/download/v1.2.3/pairee-setup-1.2.3-x64.exe\n"));
        assert!(filled.contains(&format!("    InstallerSha256: {}\n", x64.to_uppercase())));
        assert!(filled.contains("pairee-setup-1.2.3-arm64.exe\n    InstallerSha256: CDCD"));
        assert!(filled.ends_with("ManifestType: installer\n"));
        assert!(missing.is_err());
    }

    #[test]
    fn newest_version_folder_wins() {
        let dir = std::env::temp_dir().join(format!("xtask-winget-{}", std::process::id()));
        for name in ["0.9.0", "0.10.0", "0.2.0", "notes"] {
            fs::create_dir_all(dir.join(name)).unwrap();
        }
        let newest = newest_version_dir(&dir);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(newest, Some(dir.join("0.10.0")));
    }
}
