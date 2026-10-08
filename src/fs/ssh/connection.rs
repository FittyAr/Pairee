//! SSH connection and host key verification helpers.

use crate::config::localization::t;
use anyhow::{Context, Result};
use ssh2::Session;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn open_session(
    host: &str,
    port: u16,
    username: &str,
    password: Option<&str>,
    key_path: Option<&str>,
    timeout: Duration,
) -> Result<Session> {
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
    sess.set_timeout(timeout_millis(timeout));
    sess.handshake().context(t("error_ssh_handshake_failed"))?;

    verify_host_key(&mut sess, host, port)?;
    authenticate(&mut sess, username, password, key_path)?;

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

fn authenticate(
    sess: &mut Session,
    username: &str,
    password: Option<&str>,
    key_path: Option<&str>,
) -> Result<()> {
    let mut authenticated = false;

    // Try key authentication if provided
    if let Some(kp) = key_path
        && !kp.trim().is_empty()
    {
        let path = Path::new(kp);
        if path.exists() {
            sess.userauth_pubkey_file(username, None, path, password)
                .context(t("error_ssh_key_auth_failed"))?;
            authenticated = true;
        }
    }

    // Try password authentication if key failed/not provided
    if !authenticated && let Some(pass) = password {
        sess.userauth_password(username, pass)
            .context(t("error_ssh_password_auth_failed"))?;
        authenticated = true;
    }

    // Try default keys if still not authenticated
    if !authenticated {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let keys = vec![
            format!("{}/.ssh/id_rsa", home),
            format!("{}/.ssh/id_ed25519", home),
        ];
        for key in keys {
            let path = Path::new(&key);
            if path.exists()
                && sess
                    .userauth_pubkey_file(username, None, path, None)
                    .is_ok()
            {
                authenticated = true;
                break;
            }
        }
    }

    // Try agent if still not authenticated
    if !authenticated && sess.userauth_agent(username).is_ok() {
        authenticated = true;
    }

    if !authenticated {
        anyhow::bail!(t("error_ssh_auth_failed"));
    }
    Ok(())
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
