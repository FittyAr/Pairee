//! minisign verification of release assets.
//!
//! Every release asset `X` ships `X.minisig`, made with the Pairee release
//! key whose public half is embedded below. The signature (and its trusted
//! comment, which is covered by the global signature) must verify, and the
//! trusted comment must name the asset (`file:X`), so a validly signed file
//! from another release or platform cannot be substituted.

use anyhow::{Context as _, Result, bail};
use minisign_verify::{PublicKey, Signature};

/// Pairee release signing public key (minisign, key id `B478F4237C0DE585`).
/// The secret half never lives in the repository; see
/// `docs/technical/installer_guide.md`.
pub const RELEASE_PUBLIC_KEY: &str = "RWSF5Q18I/R4tADVZ5LQzgP2gRzPD/yzWj0p5kw13d6g4+Ycwbn27Fm6";

/// Verify `data` against the `.minisig` text using the embedded release key.
pub fn verify_release_asset(data: &[u8], minisig_text: &str, asset_name: &str) -> Result<()> {
    verify_with_key(RELEASE_PUBLIC_KEY, data, minisig_text, asset_name)
}

fn verify_with_key(
    public_key_b64: &str,
    data: &[u8],
    minisig_text: &str,
    asset_name: &str,
) -> Result<()> {
    let pk = PublicKey::from_base64(public_key_b64).context("invalid embedded public key")?;
    let sig = Signature::decode(minisig_text).context("malformed minisign signature")?;
    // Legacy (non-prehashed) signatures are refused.
    pk.verify(data, &sig, false)
        .context("minisign signature does not match the release key")?;
    let expected = format!("file:{asset_name}");
    if !sig
        .trusted_comment()
        .split_whitespace()
        .any(|token| token == expected)
    {
        bail!(
            "signature is for a different file (trusted comment: {:?}, expected {expected})",
            sig.trusted_comment()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway test keypair (secret key discarded after signing).
    const TEST_PK: &str = "RWS8fbZeHqnTnJgbZr63z4iY02QZjcjm+xh6RpoT3+SY0XW4D/NN/Ndt";
    const PAYLOAD: &[u8] = b"pairee test payload\n";
    const SIG: &str = "untrusted comment: signature from rsign secret key
RUS8fbZeHqnTnIbRWwkPi8MI37uON8JXdFrhAZTEIQiLYQ/Uu3rLcb/Tlim68oJ3/oWp6dATDcNgS/p3YfIMXqPTjOhFiN1kbwI=
trusted comment: timestamp:1700000000\tfile:payload.tar.gz\thashed
1ee8bVjUxfBG4/2q+EEljUXLaTaBS+/ALoE1j8vyagm09P6G/Itnu4txCveRfvA4D9IzweLC6RVmAUzOqgMZDQ==
";

    #[test]
    fn valid_signature_verifies() {
        verify_with_key(TEST_PK, PAYLOAD, SIG, "payload.tar.gz").unwrap();
    }

    #[test]
    fn tampered_payload_is_rejected() {
        assert!(verify_with_key(TEST_PK, b"pairee test payloaX\n", SIG, "payload.tar.gz").is_err());
    }

    #[test]
    fn signature_for_another_asset_is_rejected() {
        let err = verify_with_key(TEST_PK, PAYLOAD, SIG, "other.tar.gz").unwrap_err();
        assert!(err.to_string().contains("different file"), "{err}");
    }

    #[test]
    fn tampered_trusted_comment_is_rejected() {
        let forged = SIG.replace("file:payload.tar.gz", "file:other.tar.gz");
        assert!(verify_with_key(TEST_PK, PAYLOAD, &forged, "other.tar.gz").is_err());
    }

    #[test]
    fn wrong_key_and_garbage_are_rejected() {
        // The real release key did not sign the test payload.
        assert!(verify_release_asset(PAYLOAD, SIG, "payload.tar.gz").is_err());
        assert!(verify_with_key(TEST_PK, PAYLOAD, "not a signature", "payload.tar.gz").is_err());
        assert!(verify_with_key(TEST_PK, PAYLOAD, "", "payload.tar.gz").is_err());
    }

    #[test]
    fn embedded_release_key_parses() {
        assert!(PublicKey::from_base64(RELEASE_PUBLIC_KEY).is_ok());
    }
}
