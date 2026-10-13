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
use crate::credential_prompt::{Ask, CredentialRequest};
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
            password: conn.password.as_deref(),
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
    let remote = match super::connect(&url(conn, password.as_deref())?) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && !conn.username.is_empty() => {
            let retry = ask_password(conn, ask)?;
            super::connect(&url(conn, Some(&retry))?)?
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

/// The folder an SFTP panel opens (the others take it from the URL).
fn start_folder(conn: &Connection) -> PathBuf {
    let path = conn.path.trim();
    PathBuf::from(if path.is_empty() { "/" } else { path })
}

/// The password of `conn`, asked from the user (`Err`: cancelled).
fn ask_password(conn: &Connection, ask: Ask<'_>) -> anyhow::Result<String> {
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
}
