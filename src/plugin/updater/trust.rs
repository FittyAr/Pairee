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
//! * A missing signature is accepted with an "unsigned" warning, unless the
//!   user set `require_signature = true` under `[plugin_registry]`.
//!
//! The public key defaults to the Pairee release key (the registry is
//! signed by the same maintainers) and can be replaced with
//! `[plugin_registry] public_key` in `config.toml`.

use crate::config::settings::Settings;
use anyhow::Result;

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
    /// Refuse files without a signature.
    pub require_signature: bool,
}

impl Default for RegistryTrust {
    fn default() -> Self {
        Self {
            public_key: crate::update::signature::RELEASE_PUBLIC_KEY.to_string(),
            require_signature: false,
        }
    }
}

impl RegistryTrust {
    /// The registry trust configured in `settings`.
    pub fn from_settings(settings: &Settings) -> Self {
        let key = settings.plugin_registry.public_key.trim();
        Self {
            public_key: if key.is_empty() {
                Self::default().public_key
            } else {
                key.to_string()
            },
            require_signature: settings.plugin_registry.require_signature,
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
                Ok(SignatureStatus::Verified)
            }
            None if self.require_signature => anyhow::bail!(
                "{signed_name} is not signed and [plugin_registry] require_signature is on"
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
    fn missing_signature_warns_or_fails_by_policy() {
        assert_eq!(
            trust(false).verify(PAYLOAD, None, "index.toml").unwrap(),
            SignatureStatus::Unsigned
        );
        assert!(trust(true).verify(PAYLOAD, None, "index.toml").is_err());
    }

    #[test]
    fn settings_override_the_pinned_key() {
        let mut settings = Settings::default();
        assert_eq!(
            RegistryTrust::from_settings(&settings),
            RegistryTrust::default()
        );
        settings.plugin_registry.public_key = format!(" {TEST_PK} ");
        settings.plugin_registry.require_signature = true;
        assert_eq!(RegistryTrust::from_settings(&settings), trust(true));
    }
}
