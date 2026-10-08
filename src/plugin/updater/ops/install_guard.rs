//! Input validation for registry installs: identifiers, manifest file keys,
//! and in-memory hash checks. Everything coming from the registry is treated
//! as untrusted.

use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

/// Registry identifiers (plugin name, author) must match `[A-Za-z0-9_-]+`.
/// They are interpolated into URLs and local directory names.
pub fn validate_identifier(kind: &str, value: &str) -> anyhow::Result<()> {
    let ok = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok {
        Ok(())
    } else {
        anyhow::bail!("Invalid plugin {kind} '{value}': only [A-Za-z0-9_-] is allowed")
    }
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
            Component::Normal(_) => has_normal = true,
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

/// Verify the downloaded bytes before they touch the final location.
pub fn verify_bytes_sha256(bytes: &[u8], expected_hex: &str) -> anyhow::Result<()> {
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
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
    fn bytes_hash_is_checked_in_memory() {
        let hello = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_bytes_sha256(b"hello", hello).is_ok());
        assert!(verify_bytes_sha256(b"hello", &hello.to_uppercase()).is_ok());
        assert!(verify_bytes_sha256(b"hellO", hello).is_err());
    }
}
