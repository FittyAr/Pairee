//! Commit/tag/stash signature resolution shared by every Git operation.

/// Name used when neither the caller nor the Git config provide one.
const FALLBACK_NAME: &str = "Pairee User";
/// E-mail used when neither the caller nor the Git config provide one.
const FALLBACK_EMAIL: &str = "pairee@localhost";

/// Builds a signature from the explicit `name`/`email` (when non-empty),
/// then the repository/global `user.name`/`user.email`, and finally the
/// given `fallback` identity.
pub fn resolve_signature(
    repo: &git2::Repository,
    name: &str,
    email: &str,
    fallback: (&str, &str),
) -> anyhow::Result<git2::Signature<'static>> {
    let config = repo.config().ok();
    let pick = |explicit: &str, key: &str, default: &str| {
        if explicit.is_empty() {
            config
                .as_ref()
                .and_then(|c| c.get_string(key).ok())
                .unwrap_or_else(|| default.to_string())
        } else {
            explicit.to_string()
        }
    };
    let name = pick(name, "user.name", fallback.0);
    let email = pick(email, "user.email", fallback.1);
    Ok(git2::Signature::now(&name, &email)?)
}

/// The configured identity, or Pairee's built-in fallback identity.
pub fn default_signature(repo: &git2::Repository) -> anyhow::Result<git2::Signature<'static>> {
    resolve_signature(repo, "", "", fallback_identity())
}

/// Pairee's built-in `(name, email)` used when nothing else is configured.
pub fn fallback_identity() -> (&'static str, &'static str) {
    (FALLBACK_NAME, FALLBACK_EMAIL)
}
