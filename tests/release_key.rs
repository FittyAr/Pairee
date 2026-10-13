//! The minisign release public key is written in several places (the
//! updater, both install scripts, the installer guide); they must agree, and
//! the release workflow must keep checking its signatures against the key
//! embedded in the updater.

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The first double-quoted string on the line that contains `marker`.
fn quoted_after(text: &str, marker: &str, file: &str) -> String {
    let line = text
        .lines()
        .find(|l| l.contains(marker))
        .unwrap_or_else(|| panic!("{file}: no line with {marker:?}"));
    let rest = &line[line.find(marker).unwrap() + marker.len()..];
    let start = rest.find('"').expect("opening quote") + 1;
    let end = rest[start..].find('"').expect("closing quote") + start;
    rest[start..end].to_string()
}

fn updater_key() -> String {
    quoted_after(
        &read("src/update/signature.rs"),
        "pub const RELEASE_PUBLIC_KEY",
        "signature.rs",
    )
}

#[test]
fn updater_key_looks_like_a_minisign_key() {
    let key = updater_key();
    // "RW" + base64 of (algorithm, key id, Ed25519 key) = 56 characters.
    assert_eq!(key.len(), 56, "{key}");
    assert!(key.starts_with("RW"), "{key}");
}

#[test]
fn install_scripts_use_the_updater_key() {
    let key = updater_key();
    assert_eq!(
        quoted_after(&read("install.sh"), "MINISIGN_PUBKEY=", "install.sh"),
        key
    );
    assert_eq!(
        quoted_after(&read("install.ps1"), "$minisignPubKey =", "install.ps1"),
        key
    );
}

#[test]
fn installer_guide_documents_the_updater_key() {
    let guide = read("docs/technical/installer_guide.md");
    assert!(guide.contains(&updater_key()));
}

#[test]
fn release_workflow_verifies_against_the_updater_key() {
    let workflow = read(".github/workflows/release.yml");
    assert!(workflow.contains("src/update/signature.rs"));
    assert!(workflow.contains("RELEASE_PUBLIC_KEY"));
    // A key pasted into the workflow would silently drift from the updater.
    assert!(
        !workflow.contains("RWS"),
        "release.yml must not hard-code a key"
    );
}
