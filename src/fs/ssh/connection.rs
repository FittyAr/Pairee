//! SSH connection and host key verification helpers.

use super::SshLogin;
use crate::config::localization::t;
use crate::credential_prompt::CredentialRequest;
use anyhow::{Context, Result};
use ssh2::Session;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn open_session(login: &SshLogin<'_>) -> Result<Session> {
    let (host, port) = (login.host, login.port);
    let addr = format!("{}:{}", host, port);
    let socket_addrs = addr
        .to_socket_addrs()
        .context(t("error_ssh_resolve_host"))?
        .collect::<Vec<SocketAddr>>();

    if socket_addrs.is_empty() {
        anyhow::bail!(t("error_ssh_no_socket_addr").replace("{}", host));
    }

    // Connect with a 5 second timeout
    let stream = TcpStream::connect_timeout(&socket_addrs[0], Duration::from_secs(5))
        .context(t("error_ssh_connect_timeout"))?;

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
        anyhow::bail!(
            "Refusing SSH connection to {}:{} because host key \
             verification is not available. Add the server's host key to \
             your known_hosts file and try again.",
            host,
            port
        );
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
            anyhow::bail!(
                "SSH host key for {}:{} does NOT match the key in known_hosts.                  This may indicate a man-in-the-middle attack.",
                host,
                port
            );
        }
        ssh2::CheckResult::NotFound => {
            anyhow::bail!(
                "SSH host key for {}:{} is not in known_hosts. Refusing to                  connect. Add the server's host key to {:?} and try again.",
                host,
                port,
                known_hosts_path
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
        }
        ssh2::CheckResult::Failure => {
            anyhow::bail!("SSH host key check failed for {}:{}", host, port);
        }
    }
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
/// `passphrase`, or one asked from the user (also after a wrong one).
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
    if let Some(given) = passphrase
        && sess
            .userauth_pubkey_file(username, None, path, Some(given))
            .is_ok()
    {
        return Ok(());
    }
    let asked = (login.ask)(CredentialRequest::passphrase(path))
        .ok_or_else(|| anyhow::anyhow!(t("credential_prompt_cancelled")))?;
    Ok(sess.userauth_pubkey_file(username, None, path, Some(&asked))?)
}

/// Environment variable holding the user's home directory.
#[cfg(target_os = "windows")]
const HOME_VAR: &str = "USERPROFILE";
#[cfg(not(target_os = "windows"))]
const HOME_VAR: &str = "HOME";

fn known_hosts_path() -> Option<PathBuf> {
    known_hosts_path_in(std::env::var_os(HOME_VAR))
}

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
    fn timeout_is_clamped_to_u32_millis() {
        assert_eq!(timeout_millis(Duration::from_secs(30)), 30_000);
        assert_eq!(timeout_millis(Duration::ZERO), 0);
        assert_eq!(timeout_millis(Duration::from_secs(u64::MAX / 2)), u32::MAX);
    }
}
