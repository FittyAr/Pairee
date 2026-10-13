//! minisign signatures of the plugin registry (`index.toml`,
//! `blocklist.toml` and every plugin `manifest.toml`, whose `[files]`
//! table pins the SHA-256 of each plugin file).
//!
//! Each signed file `X` may have `X.minisig` next to it, made with the
//! registry key; the trusted comment must name the file by its path inside
//! `registry/` (`file:index.toml`, `file:plugins/a/alice/demo/manifest.toml`),
//! so a signature cannot be replayed for another file. Verification reuses
//! the updater's minisign code ([`crate::update::signature::verify_with_key`]).
//!
//! * A signature that is present but does not verify is always an error.
//! * A missing signature is refused once this installation has seen the
//!   registry signed with the configured key (recorded in
//!   [`SIGNED_MARKER`]): stripping the signatures cannot downgrade it to
//!   unsigned files. Until then (the registry is being signed for the first
//!   time) it is accepted with an "unsigned" warning. This is the default,
//!   `registry_require_signature = true` under `[security]`; `false` always
//!   only warns.
//!
//! The public key defaults to the Pairee release key (the registry is
//! signed by the same maintainers) and can be replaced with
//! `registry_public_key` under `[security]` in `config.toml`.

use crate::config::settings::Settings;
use anyhow::Result;
use std::path::PathBuf;

/// File in the config folder holding the key the registry was last seen
/// signed with.
pub const SIGNED_MARKER: &str = "registry_signed";

/// How a registry file was authenticated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureStatus {
    Verified,
    /// No `.minisig` was published for the file.
    Unsigned,
}

/// The key and policy registry files are checked with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryTrust {
    /// minisign public key (base64).
    pub public_key: String,
    /// Refuse files without a signature once the registry was seen signed.
    pub require_signature: bool,
    /// Where that is recorded (`None`: nowhere, so never).
    pub signed_marker: Option<PathBuf>,
}

impl Default for RegistryTrust {
    fn default() -> Self {
        Self {
            public_key: crate::update::signature::RELEASE_PUBLIC_KEY.to_string(),
            require_signature: true,
            signed_marker: None,
        }
    }
}

impl RegistryTrust {
    /// The registry trust configured in `settings`.
    pub fn from_settings(settings: &Settings) -> Self {
        let key = settings.security.registry_public_key.trim();
        Self {
            public_key: if key.is_empty() {
                Self::default().public_key
            } else {
                key.to_string()
            },
            require_signature: settings.security.registry_require_signature,
            signed_marker: Some(crate::config::paths::get_config_dir().join(SIGNED_MARKER)),
        }
    }

    /// Whether a file signed with this key has been verified before.
    fn seen_signed(&self) -> bool {
        self.signed_marker
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .is_some_and(|key| key.trim() == self.public_key)
    }

    fn record_signed(&self) {
        let Some(path) = &self.signed_marker else {
            return;
        };
        if !self.seen_signed()
            && let Err(e) = std::fs::write(path, &self.public_key)
        {
            tracing::warn!("Cannot record that the plugin registry is signed: {e}");
        }
    }

    /// Checks `data` (the registry file `signed_name`) against its
    /// `.minisig` text, if one was published.
    pub fn verify(
        &self,
        data: &[u8],
        minisig: Option<&str>,
        signed_name: &str,
    ) -> Result<SignatureStatus> {
        match minisig {
            Some(sig) => {
                crate::update::signature::verify_with_key(&self.public_key, data, sig, signed_name)
                    .map_err(|e| {
                        anyhow::anyhow!("registry signature of {signed_name} is invalid: {e:#}")
                    })?;
                self.record_signed();
                Ok(SignatureStatus::Verified)
            }
            None if self.require_signature && self.seen_signed() => anyhow::bail!(
                "{signed_name} is not signed, but the registry was signed before; \
                 refusing it ([security] registry_require_signature)"
            ),
            None => {
                tracing::warn!("Plugin registry file {signed_name} is not signed");
                Ok(SignatureStatus::Unsigned)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway keypair shared with update::signature's tests.
    const TEST_PK: &str = "RWS8fbZeHqnTnJgbZr63z4iY02QZjcjm+xh6RpoT3+SY0XW4D/NN/Ndt";
    const PAYLOAD: &[u8] = b"pairee test payload\n";
    const SIG: &str = "untrusted comment: signature from rsign secret key
RUS8fbZeHqnTnIbRWwkPi8MI37uON8JXdFrhAZTEIQiLYQ/Uu3rLcb/Tlim68oJ3/oWp6dATDcNgS/p3YfIMXqPTjOhFiN1kbwI=
trusted comment: timestamp:1700000000\tfile:payload.tar.gz\thashed
1ee8bVjUxfBG4/2q+EEljUXLaTaBS+/ALoE1j8vyagm09P6G/Itnu4txCveRfvA4D9IzweLC6RVmAUzOqgMZDQ==
";

    fn trust(require: bool) -> RegistryTrust {
        RegistryTrust {
            public_key: TEST_PK.to_string(),
            require_signature: require,
            signed_marker: None,
        }
    }

    fn with_marker(require: bool, dir: &tempfile::TempDir) -> RegistryTrust {
        RegistryTrust {
            signed_marker: Some(dir.path().join(SIGNED_MARKER)),
            ..trust(require)
        }
    }

    #[test]
    fn valid_signature_is_verified() {
        assert_eq!(
            trust(true)
                .verify(PAYLOAD, Some(SIG), "payload.tar.gz")
                .unwrap(),
            SignatureStatus::Verified
        );
    }

    #[test]
    fn bad_signature_fails_even_when_not_required() {
        assert!(
            trust(false)
                .verify(b"tampered", Some(SIG), "payload.tar.gz")
                .is_err()
        );
        assert!(
            trust(false)
                .verify(PAYLOAD, Some(SIG), "index.toml")
                .is_err()
        );
        let other_key = RegistryTrust::default();
        assert!(
            other_key
                .verify(PAYLOAD, Some(SIG), "payload.tar.gz")
                .is_err()
        );
    }

    #[test]
    fn unsigned_files_are_accepted_until_the_registry_is_seen_signed() {
        let dir = tempfile::tempdir().unwrap();
        let strict = with_marker(true, &dir);
        assert_eq!(
            strict.verify(PAYLOAD, None, "index.toml").unwrap(),
            SignatureStatus::Unsigned
        );
        strict.verify(PAYLOAD, Some(SIG), "payload.tar.gz").unwrap();
        // Signatures stripped later: refused, also by a new process.
        assert!(strict.verify(PAYLOAD, None, "index.toml").is_err());
        assert!(
            with_marker(true, &dir)
                .verify(PAYLOAD, None, "index.toml")
                .is_err()
        );
        // Opted out: only a warning.
        assert_eq!(
            with_marker(false, &dir)
                .verify(PAYLOAD, None, "index.toml")
                .unwrap(),
            SignatureStatus::Unsigned
        );
    }

    #[test]
    fn another_registry_key_starts_unsigned_again() {
        let dir = tempfile::tempdir().unwrap();
        with_marker(true, &dir)
            .verify(PAYLOAD, Some(SIG), "payload.tar.gz")
            .unwrap();
        let other = RegistryTrust {
            public_key: crate::update::signature::RELEASE_PUBLIC_KEY.to_string(),
            ..with_marker(true, &dir)
        };
        assert_eq!(
            other.verify(PAYLOAD, None, "index.toml").unwrap(),
            SignatureStatus::Unsigned
        );
    }

    #[test]
    fn settings_override_the_pinned_key() {
        let mut settings = Settings::default();
        let pinned = RegistryTrust::from_settings(&settings);
        assert_eq!(pinned.public_key, RegistryTrust::default().public_key);
        assert!(pinned.require_signature);
        assert!(pinned.signed_marker.is_some());
        settings.security.registry_public_key = format!(" {TEST_PK} ");
        settings.security.registry_require_signature = false;
        let custom = RegistryTrust::from_settings(&settings);
        assert_eq!(
            custom,
            RegistryTrust {
                signed_marker: pinned.signed_marker,
                ..trust(false)
            }
        );
    }
}
