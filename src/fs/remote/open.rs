//! Opens a saved connection of any kind: SFTP through the SSH client, the
//! others through [`super::connect`] with a URL built from the fields.
//! A missing or rejected password (an S3 secret key) is asked from the
//! user through `ask` (see [`crate::credential_prompt`]) and the login
//! tried once more.
//!
//! Network I/O and a possible prompt: call it from a background job.

use super::conn::{RemoteConn, RemoteInfo, user_at_host};
use crate::config::localization::t;
use crate::config::settings::{Connection, RemoteKind};
use crate::credential_prompt::{Ask, CredentialRequest, Secret};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

/// Connects to `conn` and returns the connection plus the folder to show.
pub fn open(
    conn: &Connection,
    timeout: Duration,
    ask: Ask<'_>,
) -> anyhow::Result<(RemoteConn, PathBuf)> {
    if conn.kind == RemoteKind::Sftp {
        let client = crate::fs::ssh::SharedSshClient::connect(&crate::fs::ssh::SshLogin {
            host: &conn.host,
            port: conn.port_number().unwrap_or_default(),
            username: &conn.username,
            password: conn.secret(),
            key_path: conn.key_path.as_deref(),
            timeout,
            ask,
        })?;
        return Ok((RemoteConn::from(client), start_folder(conn)));
    }
    let mut password = conn.password.clone();
    if password.is_none() && !conn.username.is_empty() {
        password = Some(ask_password(conn, ask)?);
    }
    let password = password.as_ref().map(|s| s.as_str());
    let remote = match connect_trusting(conn, &url(conn, password)?, ask) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && !conn.username.is_empty() => {
            let retry = ask_password(conn, ask)?;
            super::connect(&url(conn, Some(retry.as_str()))?)?
        }
        other => other?,
    };
    let info = RemoteInfo {
        kind: conn.kind,
        host: conn.host.clone(),
        port: conn.port_number(),
        username: conn.username.clone(),
    };
    // FTP opens the URL's folder; WebDAV and S3 root the adapter there.
    Ok((RemoteConn::new(remote.vfs, info), remote.start))
}

/// Connects to `url`; a server certificate the system does not trust is
/// shown to the user (server and SHA-256 fingerprint) and, when they trust
/// it, pinned for that server and the connection tried again.
fn connect_trusting(conn: &Connection, url: &str, ask: Ask<'_>) -> io::Result<super::Remote> {
    use super::tls::{self, Handshake};
    let err = match super::connect(url) {
        Err(err) => err,
        ok => return ok,
    };
    let Some(untrusted) = tls::untrusted(&err) else {
        return Err(err);
    };
    let handshake = match conn.kind {
        RemoteKind::Ftps => Handshake::FtpAuthTls,
        _ => Handshake::Direct,
    };
    let (host, port) = (untrusted.host.clone(), untrusted.port);
    let cert = tls::fetch_certificate(&host, port, handshake)?;
    let request =
        CredentialRequest::certificate(&format!("{host}:{port}"), &tls::fingerprint(&cert));
    if ask(request).is_none() {
        return Err(err);
    }
    tls::pin(&host, port, &cert)?;
    super::connect(url)
}

/// The folder an SFTP panel opens (the others take it from the URL).
fn start_folder(conn: &Connection) -> PathBuf {
    let path = conn.path.trim();
    PathBuf::from(if path.is_empty() { "/" } else { path })
}

/// The password of `conn`, asked from the user (`Err`: cancelled).
fn ask_password(conn: &Connection, ask: Ask<'_>) -> anyhow::Result<Secret> {
    let subject = user_at_host(&conn.username, &conn.host);
    ask(CredentialRequest::password(subject))
        .ok_or_else(|| anyhow::anyhow!(t("credential_prompt_cancelled")))
}

/// The backend URL of a non-SFTP connection (`scheme://user:pass@host:port/path?options`).
pub fn url(conn: &Connection, password: Option<&str>) -> io::Result<String> {
    let invalid = |why: String| super::invalid(&conn.host, &why);
    let mut url = url::Url::parse(&format!("{}://placeholder", conn.kind.scheme()))
        .map_err(|e| invalid(e.to_string()))?;
    url.set_host(Some(conn.host.trim()))
        .map_err(|e| invalid(e.to_string()))?;
    if conn.kind != RemoteKind::S3
        && let Some(port) = conn.port.trim().parse::<u16>().ok()
    {
        let _ = url.set_port(Some(port));
    }
    if !conn.username.is_empty() {
        let _ = url.set_username(&conn.username);
        let _ = url.set_password(password);
    }
    url.set_path(conn.path.trim());
    let options = conn.options.trim().trim_start_matches('?');
    if !options.is_empty() {
        url.set_query(Some(options));
    }
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(kind: RemoteKind) -> Connection {
        Connection {
            name: "n".into(),
            kind,
            host: "example.com".into(),
            port: "2121".into(),
            username: "me@corp".into(),
            path: "/pub/files".into(),
            ..Connection::default()
        }
    }

    #[test]
    fn urls_carry_every_field_encoded() {
        let ftp = url(&connection(RemoteKind::Ftps), Some("p:w/d")).unwrap();
        assert_eq!(ftp, "ftps://me%40corp:p%3Aw%2Fd@example.com:2121/pub/files");
        let mut s3 = connection(RemoteKind::S3);
        s3.host = "bucket".into();
        s3.options = "endpoint=http://127.0.0.1:9000&path_style=true".into();
        let s3 = url(&s3, Some("secret")).unwrap();
        assert!(s3.starts_with("s3://me%40corp:secret@bucket/pub/files?endpoint="));
        let mut anonymous = connection(RemoteKind::WebDav);
        anonymous.username.clear();
        anonymous.port.clear();
        assert_eq!(
            url(&anonymous, None).unwrap(),
            "webdav://example.com/pub/files"
        );
    }

    #[test]
    fn a_missing_password_is_asked_and_cancelling_stops_the_login() {
        let asked = std::sync::Mutex::new(Vec::new());
        let ask = |req: CredentialRequest| {
            asked.lock().unwrap().push(req);
            None
        };
        let err = open(&connection(RemoteKind::Ftp), Duration::from_secs(1), &ask).unwrap_err();
        assert_eq!(err.to_string(), t("credential_prompt_cancelled"));
        assert_eq!(
            *asked.lock().unwrap(),
            vec![CredentialRequest::password("me@corp@example.com")]
        );
    }

    fn webdavs() -> Connection {
        let server = crate::fs::remote::tests::webdavs_server();
        Connection {
            name: "secure".into(),
            kind: RemoteKind::WebDavs,
            host: "127.0.0.1".into(),
            port: server.addr.port().to_string(),
            ..Connection::default()
        }
    }

    #[test]
    fn an_untrusted_certificate_is_shown_and_pinned_once_trusted() {
        let _root = crate::config::paths::test_root::redirect(&tempfile::tempdir().unwrap().keep());
        let conn = webdavs();
        let refuse = |_req: CredentialRequest| None;
        let err = open(&conn, Duration::from_secs(5), &refuse).unwrap_err();
        assert!(err.to_string().contains("127.0.0.1"), "{err}");

        let asked = std::sync::Mutex::new(Vec::new());
        let trust = |req: CredentialRequest| {
            asked.lock().unwrap().push(req);
            Some(crate::credential_prompt::Secret::default())
        };
        open(&conn, Duration::from_secs(5), &trust).unwrap();
        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 1);
        assert_eq!(
            asked[0].kind,
            crate::credential_prompt::SecretKind::Certificate
        );
        let fingerprint =
            crate::fs::remote::tls::fingerprint(&crate::fs::remote::tests::test_cert().der);
        assert!(
            asked[0].subject.ends_with(&fingerprint),
            "{}",
            asked[0].subject
        );

        // Pinned: the next connection does not ask.
        let ask_again = |_req: CredentialRequest| -> Option<crate::credential_prompt::Secret> { panic!("asked again") };
        open(&conn, Duration::from_secs(5), &ask_again).unwrap();
    }

    #[test]
    fn only_tls_kinds_can_turn_checks_off() {
        let mut conn = webdavs();
        assert!(!conn.insecure_tls());
        conn.options = "insecure=true".into();
        assert!(conn.insecure_tls());
        conn.kind = RemoteKind::WebDav;
        assert!(!conn.insecure_tls());
    }
}
