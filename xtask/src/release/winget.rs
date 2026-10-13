//! WinGet manifests: one folder per version under [`WINGET_DIR`], moved to
//! the new version and rewritten at each release.

use std::fs;
use std::path::{Path, PathBuf};

use super::version::Version;
use crate::Result;
use crate::text::TextFile;

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
            } else if key.starts_with("InstallerUrl:") || key.starts_with("ReleaseNotesUrl:") {
                *line = replace_versions(line, new);
            }
        }
        manifest.save()?;
    }
    Ok(to)
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
