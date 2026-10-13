//! The release version: parsing, choosing the next one, and writing it into
//! every file that carries it.

use std::fmt;

use super::{Snapshot, branch, winget};
use crate::text::TextFile;
use crate::{Result, changelog, ui};

pub const CARGO_TOML: &str = "Cargo.toml";
pub const CARGO_LOCK: &str = "Cargo.lock";
pub const INSTALLER: &str = "installer.iss";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Version(pub u64, pub u64, pub u64);

impl Version {
    pub fn next_patch(self) -> Self {
        Self(self.0, self.1, self.2 + 1)
    }
}

impl std::str::FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, String> {
        let invalid = || format!("invalid version '{s}' (expected X.Y.Z)");
        let mut parts = s.trim().trim_start_matches('v').split('.');
        let mut next = || -> std::result::Result<u64, String> {
            parts
                .next()
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())
        };
        let version = Self(next()?, next()?, next()?);
        match parts.next() {
            None => Ok(version),
            Some(_) => Err(invalid()),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// `version` in the `[package]` table of Cargo.toml.
pub fn cargo_version() -> Result<Version> {
    let manifest = TextFile::load(CARGO_TOML)?;
    let line = package_version_line(&manifest.lines).ok_or("no [package] version in Cargo.toml")?;
    let value = manifest.lines[line]
        .split('"')
        .nth(1)
        .ok_or("malformed version line")?;
    Ok(value.parse()?)
}

fn package_version_line(lines: &[String]) -> Option<usize> {
    let mut in_package = false;
    lines.iter().position(|l| {
        let l = l.trim();
        if l.starts_with('[') {
            in_package = l == "[package]";
        }
        in_package && l.starts_with("version") && l[7..].trim_start().starts_with('=')
    })
}

/// The requested version, or the one the user types (next patch by
/// default). It must be newer and its tag must not exist locally nor, when
/// `check_remote`, on origin.
pub fn choose(current: Version, requested: Option<Version>, check_remote: bool) -> Result<Version> {
    println!("Current version: {current}");
    let new = match requested {
        Some(v) => v,
        None => ui::ask("New version", &current.next_patch().to_string())?.parse()?,
    };
    if new <= current {
        return Err(format!("{new} is not newer than {current}").into());
    }
    branch::ensure_tag_is_free(&format!("v{new}"), check_remote)?;
    Ok(new)
}

/// Writes `new` everywhere the version appears and records the WinGet move
/// in `snapshot`; returns the paths to commit.
pub fn bump_files(current: Version, new: Version, snapshot: &mut Snapshot) -> Result<Vec<String>> {
    ui::step(&format!("Bumping {current} -> {new}"));
    let mut files = vec![CARGO_TOML.to_string(), CARGO_LOCK.to_string()];

    let mut manifest = TextFile::load(CARGO_TOML)?;
    let line = package_version_line(&manifest.lines).ok_or("no [package] version in Cargo.toml")?;
    manifest.lines[line] = format!("version = \"{new}\"");
    manifest.save()?;

    let mut installer = TextFile::load(INSTALLER)?;
    if installer.replace_line(
        |l| l.starts_with("#define AppVersion"),
        |_| format!("#define AppVersion \"{new}\""),
    ) {
        installer.save()?;
        files.push(INSTALLER.into());
    } else {
        ui::warn("installer.iss has no `#define AppVersion` line");
    }

    match winget::source_dir(current) {
        Some(from) => {
            let to = winget::bump(&from, new)?;
            snapshot.record_move(from, to);
            files.push(winget::WINGET_DIR.to_string());
        }
        None => ui::warn("no WinGet manifest folder found; skipping"),
    }

    changelog::stamp(&new.to_string())?;
    files.extend([changelog::CHANGELOG.into(), changelog::UNRELEASED.into()]);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_parse_and_order() {
        assert_eq!("v0.9.0".parse::<Version>(), Ok(Version(0, 9, 0)));
        assert!("0.9".parse::<Version>().is_err());
        assert!("0.9.0.1".parse::<Version>().is_err());
        assert!(Version(0, 10, 0) > Version(0, 9, 9));
        assert_eq!(Version(0, 9, 0).next_patch(), Version(0, 9, 1));
    }

    #[test]
    fn package_version_ignores_other_tables() {
        let lines: Vec<String> = [
            "[dependencies]",
            "version = \"1\"",
            "[package]",
            "name = \"p\"",
            "version = \"0.1.0\"",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(package_version_line(&lines), Some(4));
    }
}
