//! SSH connection and host key verification helpers.

use super::SshLogin;
use crate::config::localization::t;
use crate::credential_prompt::CredentialRequest;
use anyhow::{Context, Result};
use ssh2::Session;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Time allowed to reach the server.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Times the passphrase of a key is asked for before giving up.
pub(crate) const MAX_PASSPHRASE_PROMPTS: usize = 3;
/// libssh2's error for a key file it cannot decrypt (a wrong passphrase),
/// as opposed to the server refusing the key.
const LIBSSH2_ERROR_FILE: std::ffi::c_int = -16;

pub fn open_session(login: &SshLogin<'_>) -> Result<Session> {
    let (host, port) = (login.host, login.port);
    // Every address of the host is tried (an unreachable IPv6 address no
    // longer hides a working IPv4 one); IPv6 literals work as hosts.
    let stream = crate::fs::remote::connect_tcp(host, port, CONNECT_TIMEOUT).map_err(|e| {
        use std::io::ErrorKind as K;
        let key = match e.kind() {
            K::AddrNotAvailable => "error_ssh_no_socket_addr",
            K::TimedOut
            | K::ConnectionRefused
            | K::ConnectionReset
            | K::HostUnreachable
            | K::NetworkUnreachable => "error_ssh_connect_timeout",
            _ => "error_ssh_resolve_host",
        };
        anyhow::Error::new(e).context(t(key).replace("{}", host))
    })?;

    let mut sess = Session::new().context(t("error_ssh_create_session"))?;
    sess.set_tcp_stream(stream);
    // Bound every blocking libssh2 call (handshake, auth, SFTP) so a dead
    // server cannot hang a worker forever. 0 = no limit.
    sess.set_timeout(timeout_millis(login.timeout));
    sess.handshake().context(t("error_ssh_handshake_failed"))?;

    verify_host_key(&mut sess, host, port)?;
    authenticate(&mut sess, login)?;

    Ok(sess)
}

/// libssh2 takes the timeout in milliseconds as `u32`.
pub(super) fn timeout_millis(timeout: Duration) -> u32 {
    u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX)
}

fn verify_host_key(sess: &mut Session, host: &str, port: u16) -> Result<()> {
    let known_hosts_path = known_hosts_path();
    let mut known_hosts = sess
        .known_hosts()
        .context("Failed to allocate SSH known_hosts handle")?;
    let kh_loaded = if let Some(ref p) = known_hosts_path {
        if p.exists() {
            match known_hosts.read_file(p, ssh2::KnownHostFileKind::OpenSSH) {
                Ok(_) => true,
                Err(e) => {
                    log::warn!(
                        "Failed to read known_hosts file {:?}: {} — host key \
                         verification is disabled for this connection; \
                         the session is vulnerable to MITM.",
                        p,
                        e
                    );
                    false
                }
            }
        } else {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            log::warn!(
                "known_hosts file {:?} does not exist; SSH host key \
                 verification is disabled for this connection. Add the \
                 server's host key to the file to enable verification.",
                p
            );
            false
        }
    } else {
        log::warn!(
            "Could not determine a known_hosts file path; SSH host key \
             verification is disabled for this connection."
        );
        false
    };

    if !kh_loaded {
        anyhow::bail!(host_key_message(
            "error_ssh_known_hosts_unavailable",
            host,
            port
        ));
    }

    let hostkey = sess
        .host_key()
        .ok_or_else(|| anyhow::anyhow!("SSH handshake returned no host key"))?;
    let (key_bytes, _key_type) = hostkey;
    host_key_verdict(
        known_hosts.check_port(host, port, key_bytes),
        host,
        port,
        known_hosts_path.as_deref(),
    )
}

/// Turns a `known_hosts` lookup result into the connect decision: only an
/// exact match is accepted; unknown, mismatching or failed checks refuse.
pub(super) fn host_key_verdict(
    result: ssh2::CheckResult,
    host: &str,
    port: u16,
    known_hosts_path: Option<&Path>,
) -> Result<()> {
    match result {
        ssh2::CheckResult::Match => Ok(()),
        ssh2::CheckResult::Mismatch => {
            anyhow::bail!(host_key_message("error_ssh_host_key_mismatch", host, port));
        }
        ssh2::CheckResult::NotFound => {
            let file = known_hosts_path
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            anyhow::bail!(
                host_key_message("error_ssh_host_key_unknown", host, port).replacen("{}", &file, 1)
            );
        }
        ssh2::CheckResult::Failure => {
            anyhow::bail!(host_key_message(
                "error_ssh_host_key_check_failed",
                host,
                port
            ));
        }
    }
}

/// The message under `key` with its first `{}` set to `host:port`.
fn host_key_message(key: &str, host: &str, port: u16) -> String {
    t(key).replacen("{}", &format!("{host}:{port}"), 1)
}

/// Logs in with, in order: the connection's key file (its passphrase is
/// the password, or asked for when the key is encrypted and none was
/// given), the password, ssh-agent, the default keys in `~/.ssh` and
/// finally a password asked from the user (`login.ask`).
fn authenticate(sess: &mut Session, login: &SshLogin<'_>) -> Result<()> {
    let username = login.username;
    if let Some(path) = login
        .key_path
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(Path::new)
        .filter(|p| p.exists())
    {
        return key_login(sess, login, path, login.password)
            .context(t("error_ssh_key_auth_failed"));
    }
    if let Some(pass) = login.password {
        return sess
            .userauth_password(username, pass)
            .context(t("error_ssh_password_auth_failed"));
    }
    if sess.userauth_agent(username).is_ok() {
        return Ok(());
    }
    for key in crate::credential_prompt::key::default_key_paths() {
        if key.exists() && key_login(sess, login, &key, None).is_ok() {
            return Ok(());
        }
    }
    let accepts_password = sess
        .auth_methods(username)
        .is_ok_and(|methods| methods.split(',').any(|m| m == "password"));
    if accepts_password {
        let subject = crate::fs::remote::user_at_host(username, login.host);
        if let Some(pass) = (login.ask)(CredentialRequest::password(subject)) {
            return sess
                .userauth_password(username, &pass)
                .context(t("error_ssh_password_auth_failed"));
        }
    }
    anyhow::bail!(t("error_ssh_auth_failed"))
}

/// Public key login with the key file at `path`. An encrypted key uses
/// `passphrase`, or one asked from the user; a wrong passphrase is asked
/// again, up to [`MAX_PASSPHRASE_PROMPTS`] times.
fn key_login(
    sess: &Session,
    login: &SshLogin<'_>,
    path: &Path,
    passphrase: Option<&str>,
) -> Result<()> {
    let username = login.username;
    if !crate::credential_prompt::key::is_encrypted(path) {
        return Ok(sess.userauth_pubkey_file(username, None, path, None)?);
    }
    let try_key = |secret: &str| sess.userauth_pubkey_file(username, None, path, Some(secret));
    let mut request = CredentialRequest::passphrase(path);
    if let Some(given) = passphrase {
        match try_key(given) {
            Ok(()) => return Ok(()),
            Err(e) if wrong_passphrase(&e) => request = request.again(),
            Err(_) => {}
        }
    }
    with_prompts(login.ask, request, MAX_PASSPHRASE_PROMPTS, |asked| {
        try_key(asked).map_err(|e| (wrong_passphrase(&e), e.into()))
    })
}

/// Whether `error` says the key file could not be decrypted.
fn wrong_passphrase(error: &ssh2::Error) -> bool {
    error.code() == ssh2::ErrorCode::Session(LIBSSH2_ERROR_FILE)
}

/// Asks `request` and hands the answer to `attempt`, asking again (marked
/// as a retry) while it fails with `(true, _)`, at most `prompts` times.
/// Cancelling the prompt ends with the "cancelled" error.
pub(crate) fn with_prompts(
    ask: crate::credential_prompt::Ask<'_>,
    mut request: CredentialRequest,
    prompts: usize,
    mut attempt: impl FnMut(&str) -> std::result::Result<(), (bool, anyhow::Error)>,
) -> Result<()> {
    let mut last = None;
    for _ in 0..prompts {
        let asked = ask(request.clone())
            .ok_or_else(|| anyhow::anyhow!(t("credential_prompt_cancelled")))?;
        match attempt(&asked) {
            Ok(()) => return Ok(()),
            Err((true, e)) => {
                last = Some(e);
                request = request.again();
            }
            Err((false, e)) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!(t("credential_prompt_cancelled"))))
}

/// Environment variable holding the user's home directory.
#[cfg(target_os = "windows")]
const HOME_VAR: &str = "USERPROFILE";
#[cfg(not(target_os = "windows"))]
const HOME_VAR: &str = "HOME";

fn known_hosts_path() -> Option<PathBuf> {
    #[cfg(all(test, pairee_sftp_test))]
    if let Some(path) = TEST_KNOWN_HOSTS.get() {
        return Some(path.clone());
    }
    known_hosts_path_in(std::env::var_os(HOME_VAR))
}

/// The known_hosts file of the opt-in tests against a real server (it
/// holds the test container's key, never the user's own file).
#[cfg(all(test, pairee_sftp_test))]
pub(crate) static TEST_KNOWN_HOSTS: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// `<home>/.ssh/known_hosts`, or `None` when the home directory is unknown.
pub(super) fn known_hosts_path_in(home: Option<std::ffi::OsString>) -> Option<PathBuf> {
    home.filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".ssh").join("known_hosts"))
}

#[cfg(test)]
mod timeout_tests {
    use super::timeout_millis;
    use std::time::Duration;

    #[test]
    fn a_wrong_passphrase_is_asked_again_a_bounded_number_of_times() {
        use super::{CredentialRequest, MAX_PASSPHRASE_PROMPTS, with_prompts};
        use crate::credential_prompt::Secret;
        use std::sync::Mutex;
        let asked: Mutex<Vec<bool>> = Mutex::new(Vec::new());
        let ask = |req: CredentialRequest| {
            asked.lock().unwrap().push(req.retry);
            Some(Secret::new(format!("try{}", asked.lock().unwrap().len())))
        };
        let request = CredentialRequest::passphrase(std::path::Path::new("k"));
        // Right on the second try.
        let result = with_prompts(&ask, request.clone(), MAX_PASSPHRASE_PROMPTS, |p| {
            if p == "try2" {
                Ok(())
            } else {
                Err((true, anyhow::anyhow!("wrong")))
            }
        });
        assert!(result.is_ok());
        assert_eq!(*asked.lock().unwrap(), [false, true]);
        // Always wrong: gives up after the last prompt.
        asked.lock().unwrap().clear();
        let result = with_prompts(&ask, request.clone(), MAX_PASSPHRASE_PROMPTS, |_| {
            Err((true, anyhow::anyhow!("wrong")))
        });
        assert!(result.is_err());
        assert_eq!(asked.lock().unwrap().len(), MAX_PASSPHRASE_PROMPTS);
        // Another failure (the server refused the key): no new prompt.
        asked.lock().unwrap().clear();
        let result = with_prompts(&ask, request, MAX_PASSPHRASE_PROMPTS, |_| {
            Err((false, anyhow::anyhow!("refused")))
        });
        assert!(result.is_err());
        assert_eq!(asked.lock().unwrap().len(), 1);
    }

    #[test]
    fn timeout_is_clamped_to_u32_millis() {
        assert_eq!(timeout_millis(Duration::from_secs(30)), 30_000);
        assert_eq!(timeout_millis(Duration::ZERO), 0);
        assert_eq!(timeout_millis(Duration::from_secs(u64::MAX / 2)), u32::MAX);
    }
}
