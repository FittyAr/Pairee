//! Remote [`Vfs`] backends besides SFTP (FEAT-11): FTP / FTPS, WebDAV and
//! S3, each an adapter of the panel source port that passes the same
//! contract suite as the local, SFTP and archive ones.
//!
//! [`connect`] reads a URL and returns the adapter for its scheme plus the
//! folder to open; [`open`] is what the connection dialog calls with a
//! saved connection of any kind (SFTP included), asking for a missing
//! password. Panels, transfers, the journal and auto-refresh hold the
//! result as a [`RemoteConn`], whatever the protocol.
//!
//! | scheme | backend |
//! |---|---|
//! | `ftp://user:pass@host:21/dir` | FTP (passive mode) |
//! | `ftps://…` | FTP with explicit TLS (`AUTH TLS`) |
//! | `webdav://`, `dav://`, `http://` | WebDAV over HTTP |
//! | `webdavs://`, `davs://`, `https://` | WebDAV over HTTPS |
//! | `s3://key:secret@bucket/prefix?endpoint=…&region=…&path_style=true` | S3 and compatible stores |
//!
//! FTPS and HTTPS trust the system's roots plus the options `ca=<pem>`
//! and `insecure=true` and the certificates the user pinned (see [`tls`]).
//!
//! Every call performs network I/O: like SFTP, run them from background
//! jobs. Paths are absolute in the adapter's namespace (`/dir/file`).

mod conn;
mod ftp;
mod http;
mod open;
mod s3;
pub mod tls;
mod webdav;
mod xml;

#[cfg(test)]
pub(crate) mod tests;

pub use conn::{RemoteConn, RemoteInfo, endpoint_vfs, is_dir_on, user_at_host};
pub use ftp::FtpVfs;
pub use open::open;
pub use s3::S3Vfs;
pub use webdav::WebDavVfs;

use crate::config::localization::t;
use crate::fs::vfs::{Capabilities, Vfs};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// A connected remote filesystem and the folder its URL names.
#[derive(Debug, Clone)]
pub struct Remote {
    pub vfs: Arc<dyn Vfs>,
    pub start: PathBuf,
}

/// The backend a URL scheme selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Ftp { secure: bool },
    WebDav { secure: bool },
    S3,
}

impl Scheme {
    /// The backend of `scheme` (case-insensitive), if any.
    pub fn parse(scheme: &str) -> Option<Self> {
        Some(match scheme.to_ascii_lowercase().as_str() {
            "ftp" => Self::Ftp { secure: false },
            "ftps" => Self::Ftp { secure: true },
            "webdav" | "dav" | "http" => Self::WebDav { secure: false },
            "webdavs" | "davs" | "https" => Self::WebDav { secure: true },
            "s3" => Self::S3,
            _ => return None,
        })
    }
}

/// Opens the remote filesystem `url` names.
pub fn connect(url: &str) -> io::Result<Remote> {
    let parsed = url::Url::parse(url).map_err(|e| invalid(url, &e.to_string()))?;
    let scheme = Scheme::parse(parsed.scheme()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            t("remote_unsupported_scheme").replacen("{}", parsed.scheme(), 1),
        )
    })?;
    match scheme {
        Scheme::Ftp { secure } => FtpVfs::connect(&parsed, secure),
        Scheme::WebDav { secure } => WebDavVfs::connect(&parsed, secure),
        Scheme::S3 => S3Vfs::connect(&parsed),
    }
}

/// The error of an unusable URL (its password, if any, left out).
fn invalid(url: &str, why: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        t("remote_invalid_url")
            .replacen("{}", &without_password(url), 1)
            .replacen("{}", why, 1),
    )
}

/// `url` as it may be shown or logged: without its password (cut by hand
/// when the URL does not parse).
fn without_password(url: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(url) {
        let _ = parsed.set_password(None);
        return parsed.into();
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let authority = &rest[..rest.find('/').unwrap_or(rest.len())];
    match authority.rfind('@') {
        Some(at) => {
            let user = authority[..at].split(':').next().unwrap_or_default();
            format!("{scheme}://{user}@{}", &rest[at + 1..])
        }
        None => url.to_string(),
    }
}

/// What the remote backends support: everything but local tools and the
/// Attributes dialog.
const REMOTE: Capabilities = Capabilities {
    attributes: false,
    local_tools: false,
    ..Capabilities::FULL
};

/// `path` as the server sees it: `/`-separated, absolute, without `.` and
/// `..` (Windows paths built with `join` use `\`).
pub fn remote_path(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop();
            }
            _ => {}
        }
    }
    format!("/{}", parts.join("/"))
}

/// [`remote_path`] for a protocol that writes the path into a command line
/// (FTP): a name with a line break or NUL would end the command and start
/// another one of the attacker's choosing, so it is refused.
pub fn command_path(path: &Path) -> io::Result<String> {
    let full = remote_path(path);
    if full.contains(['\r', '\n', '\0']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            t("remote_invalid_name").replacen("{}", &full.escape_debug().to_string(), 1),
        ));
    }
    Ok(full)
}

/// A TCP connection to `host:port`, trying every address the name resolves
/// to (a host with an IPv6 address the network cannot reach still connects
/// over IPv4), each within `timeout`. IPv6 literals work as hosts.
pub fn connect_tcp(host: &str, port: u16, timeout: Duration) -> io::Result<TcpStream> {
    let mut last = io::Error::from(io::ErrorKind::AddrNotAvailable);
    for addr in (host, port).to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// `text` with its `%XX` escapes decoded (lossy UTF-8).
fn percent_decoded(text: &str) -> String {
    percent_encoding::percent_decode_str(text)
        .decode_utf8_lossy()
        .into_owned()
}

/// The user name and password of `url`, percent-decoded (`None`: none).
fn credentials(url: &url::Url) -> Option<(String, String)> {
    (!url.username().is_empty()).then(|| {
        (
            percent_decoded(url.username()),
            percent_decoded(url.password().unwrap_or_default()),
        )
    })
}

/// The path of `url` as the folder it names: percent-decoded (a saved
/// connection's `/My Docs` travels as `/My%20Docs`) and normalized like
/// [`remote_path`].
fn url_path(url: &url::Url) -> String {
    remote_path(Path::new(&percent_decoded(url.path())))
}

/// The entry name of `path` (its last component).
fn file_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// A "not found" error for `path`.
fn not_found(path: &Path) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, path.to_string_lossy().into_owned())
}
