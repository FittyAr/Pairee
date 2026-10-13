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
fn test_repeated_username_requests_are_capped() {
    let mut state = CredentialState::new(Vec::new());

    let (offers, err) = exhaust(&mut state, CredentialType::USERNAME);

    assert!(offers < 100);
    assert_eq!(err.message(), t("git_error_auth_failed"));
}
