//! Input validation for registry installs: identifiers, manifest file keys,
//! and in-memory hash checks. Everything coming from the registry is treated
//! as untrusted.

use std::path::{Component, Path, PathBuf};

/// Registry identifiers (plugin name, author) must match `[A-Za-z0-9_-]+`.
/// They are interpolated into URLs and local directory names.
pub fn validate_identifier(kind: &str, value: &str) -> anyhow::Result<()> {
    if is_identifier(value) {
        Ok(())
    } else {
        anyhow::bail!("Invalid plugin {kind} '{value}': only [A-Za-z0-9_-] is allowed")
    }
}

/// A non-empty `[A-Za-z0-9_-]` name: safe as a single path component
/// (plugin and author names, `require` segments, language codes).
pub fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Join a manifest `[files]` key onto `base`, rejecting anything that could
/// land outside it (`..`, absolute paths, drive prefixes, backslashes, `:`).
pub fn safe_join(base: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    if rel.is_empty() || rel.contains(['\\', ':', '\0']) {
        anyhow::bail!("Refusing unsafe plugin file path '{rel}'");
    }
    let rel_path = Path::new(rel);
    let mut has_normal = false;
    for comp in rel_path.components() {
        match comp {
            Component::Normal(part) => {
                has_normal = true;
                let part = part.to_string_lossy();
                if !is_portable_component(&part) {
                    anyhow::bail!(
                        "Refusing plugin file path '{rel}': '{part}' is not a portable file name"
                    );
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("Refusing unsafe plugin file path '{rel}'")
            }
        }
    }
    if !has_normal || rel_path.is_absolute() {
        anyhow::bail!("Refusing unsafe plugin file path '{rel}'");
    }
    let joined = base.join(rel_path);
    if !joined.starts_with(base) {
        anyhow::bail!("Refusing plugin file path '{rel}' outside the plugin directory");
    }
    Ok(joined)
}

/// Names Windows reserves for devices (with or without an extension).
const RESERVED_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9", "conin$",
    "conout$",
];

/// A file name that means the same on every platform: no Windows device
/// name, no trailing dot or space (Windows drops them, so `main.lua.` would
/// replace `main.lua`), no characters Windows forbids, no control codes.
fn is_portable_component(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    !name.ends_with(['.', ' '])
        && !name.contains(['<', '>', '"', '|', '?', '*'])
        && !name.chars().any(char::is_control)
        && !RESERVED_NAMES.contains(&stem.to_ascii_lowercase().as_str())
}

/// Refuses `[files]` keys that name the same file once case is ignored (two
/// keys landing on one file on Windows and macOS) or that need a name to be
/// both a file and a folder (`a` and `a/b`).
pub fn check_unique_keys<'a>(keys: impl IntoIterator<Item = &'a String>) -> anyhow::Result<()> {
    let mut seen = std::collections::HashSet::new();
    let mut folders = std::collections::HashSet::new();
    let normalized: Vec<(String, &String)> = keys
        .into_iter()
        .map(|k| (k.trim_start_matches("./").to_lowercase(), k))
        .collect();
    for (key, original) in &normalized {
        if !seen.insert(key.clone()) {
            anyhow::bail!("Plugin lists '{original}' twice (names are compared ignoring case)");
        }
        let mut end = 0;
        while let Some(i) = key[end..].find('/') {
            end += i;
            folders.insert(key[..end].to_string());
            end += 1;
        }
    }
    if let Some((_, original)) = normalized.iter().find(|(k, _)| folders.contains(k)) {
        anyhow::bail!("Plugin lists '{original}' as both a file and a folder");
    }
    Ok(())
}

/// Verify the downloaded bytes before they touch the final location.
pub fn verify_bytes_sha256(bytes: &[u8], expected_hex: &str) -> anyhow::Result<()> {
    let actual = crate::update::downloader::sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        anyhow::bail!("SHA-256 mismatch: expected {expected_hex}, got {actual}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_traversal_and_separators() {
        assert!(validate_identifier("name", "git-status_2").is_ok());
        for bad in ["", "..", "a/b", "a\\b", "x y", "évil", "a.b", "../../x"] {
            assert!(validate_identifier("name", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn safe_join_accepts_nested_relative_paths() {
        let base = Path::new("plugins").join("demo.pairee");
        let p = safe_join(&base, "lib/util.lua").unwrap();
        assert_eq!(p, base.join("lib").join("util.lua"));
        assert!(safe_join(&base, "./main.lua").is_ok());
    }

    #[test]
    fn safe_join_rejects_escapes() {
        let base = Path::new("plugins").join("demo.pairee");
        for bad in [
            "",
            ".",
            "../evil.lua",
            "lib/../../evil.lua",
            "/etc/passwd",
            "..\\evil.lua",
            "C:\\Windows\\evil.dll",
            "C:evil.dll",
            "\\\\server\\share\\x",
        ] {
            assert!(safe_join(&base, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn safe_join_rejects_names_windows_would_reinterpret() {
        let base = Path::new("plugins").join("demo.pairee");
        for bad in [
            "con",
            "lib/NUL.lua",
            "aux.txt",
            "COM1",
            "main.lua.",
            "main.lua ",
            "lib/x.lua./y",
            "a?.lua",
            "a|b",
        ] {
            assert!(safe_join(&base, bad).is_err(), "{bad}");
        }
        for ok in ["console.lua", "lib/communicate.lua", "com10.lua", "a.b.lua"] {
            assert!(safe_join(&base, ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn colliding_keys_are_refused() {
        let keys = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(check_unique_keys(&keys(&["main.lua", "lib/a.lua"])).is_ok());
        assert!(check_unique_keys(&keys(&["Main.lua", "main.lua"])).is_err());
        assert!(check_unique_keys(&keys(&["main.lua", "./main.lua"])).is_err());
        assert!(check_unique_keys(&keys(&["lib", "lib/a.lua"])).is_err());
        assert!(check_unique_keys(&keys(&["LIB", "lib/a.lua"])).is_err());
    }

    #[test]
    fn bytes_hash_is_checked_in_memory() {
        let hello = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_bytes_sha256(b"hello", hello).is_ok());
        assert!(verify_bytes_sha256(b"hello", &hello.to_uppercase()).is_ok());
        assert!(verify_bytes_sha256(b"hellO", hello).is_err());
    }
}
