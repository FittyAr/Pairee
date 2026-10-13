//! Credential negotiation always terminates.

use crate::config::localization::t;
use crate::git::credentials::CredentialState;
use git2::CredentialType;

/// Calls the negotiation until it gives up; returns (offers made, final error).
fn exhaust(state: &mut CredentialState, allowed: CredentialType) -> (usize, git2::Error) {
    for offers in 0..100 {
        if let Err(e) =
            state.next_credential("ssh://git@example.com/repo.git", Some("git"), allowed)
        {
            return (offers, e);
        }
    }
    panic!("credential negotiation never gave up");
}

#[test]
fn test_ssh_negotiation_is_bounded_and_explains_passphrases() {
    let missing = std::env::temp_dir().join("pairee-no-such-key");
    let mut state = CredentialState::new(vec![missing]);

    let (offers, err) = exhaust(&mut state, CredentialType::SSH_KEY);

    // At most the ssh-agent; the missing key file is skipped.
    assert!(offers <= 1, "offered {offers} credentials");
    assert_eq!(err.message(), t("git_error_ssh_auth_failed"));
}

#[test]
fn test_encrypted_keys_are_offered_with_the_asked_passphrase() {
    let dir = tempfile::tempdir().unwrap();
    let encrypted = dir.path().join("id_ed25519");
    let plain = dir.path().join("id_rsa");
    let key = crate::credential_prompt::key::tests::openssh_key;
    std::fs::write(&encrypted, key("aes256-ctr")).unwrap();
    std::fs::write(&plain, key("none")).unwrap();
    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = asked.clone();
    let mut state = CredentialState::new(vec![encrypted.clone(), plain]).with_prompt(move |req| {
        log.lock().unwrap().push(req);
        None
    });

    let (offers, _) = exhaust(&mut state, CredentialType::SSH_KEY);

    // The encrypted key was asked about once and skipped when cancelled;
    // the plain key is offered without asking.
    let asked = asked.lock().unwrap();
    assert_eq!(
        *asked,
        vec![crate::credential_prompt::CredentialRequest::passphrase(
            &encrypted
        )]
    );
    assert!((1..=2).contains(&offers), "offered {offers} credentials");
}

#[test]
fn test_repeated_username_requests_are_capped() {
    let mut state = CredentialState::new(Vec::new());

    let (offers, err) = exhaust(&mut state, CredentialType::USERNAME);

    assert!(offers < 100);
    assert_eq!(err.message(), t("git_error_auth_failed"));
}
