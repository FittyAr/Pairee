//! The WebDAV adapter (RFC 4918 class 1): `PROPFIND` lists, `GET` reads
//! (with `Range` for resumed copies), `PUT` writes as a stream, `MKCOL`,
//! `DELETE` and `MOVE` change the tree. Basic authentication.

mod propfind;

use super::http::{self, BodyReader, Upload};
use super::{REMOTE, Remote, credentials, file_name, invalid, not_found, remote_path};
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry};
use propfind::DavItem;
use reqwest::{Method, StatusCode};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// Time allowed to connect to the server.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// Time allowed between two network reads.
const READ_TIMEOUT: Duration = Duration::from_secs(60);

pub struct WebDavVfs {
    client: reqwest::Client,
    /// The server URL the paths are below (path without trailing `/`).
    base: Url,
    auth: Option<(String, String)>,
}

impl std::fmt::Debug for WebDavVfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebDavVfs")
            .field("host", &self.base.host_str())
            .finish()
    }
}

impl WebDavVfs {
    /// The server of `url` (`webdav[s]://`, `dav[s]://` or `http[s]://`);
    /// its path is the root of the adapter's namespace.
    pub fn connect(url: &Url, secure: bool) -> io::Result<Remote> {
        let mut base = url.clone();
        let scheme = if secure { "https" } else { "http" };
        if base.set_scheme(scheme).is_err() {
            // `Url` refuses to turn custom schemes into special ones.
            let rest = &url.as_str()[url.scheme().len()..];
            base = Url::parse(&format!("{scheme}{rest}"))
                .map_err(|e| invalid(url.as_str(), &e.to_string()))?;
        }
        let auth = credentials(&base);
        let _ = base.set_username("");
        let _ = base.set_password(None);
        base.set_query(None);
        let path = base.path().trim_end_matches('/').to_string();
        base.set_path(&path);
        let client = crate::update::http::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()
            .map_err(http::io_error)?;
        let vfs = Self { client, base, auth };
        vfs.stat(Path::new("/"))?;
        Ok(Remote {
            vfs: Arc::new(vfs),
            start: PathBuf::from("/"),
        })
    }

    /// The URL of `path` (folders end with `/`).
    fn url(&self, path: &Path, dir: bool) -> Url {
        let mut url = self.base.clone();
        let mut segments: Vec<String> = Vec::new();
        segments.extend(
            remote_path(path)
                .split('/')
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        );
        if let Ok(mut parts) = url.path_segments_mut() {
            parts.pop_if_empty().extend(&segments);
            if dir {
                parts.push("");
            }
        }
        url
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        let request = self.client.request(method, url);
        match &self.auth {
            Some((user, password)) => request.basic_auth(user, Some(password)),
            None => request,
        }
    }

    /// `PROPFIND` of `path` with `depth` (0: itself, 1: and its children).
    fn propfind(&self, path: &Path, depth: &str) -> io::Result<Vec<DavItem>> {
        let method = Method::from_bytes(b"PROPFIND").map_err(io::Error::other)?;
        let request = self
            .request(method, self.url(path, false))
            .header("Depth", depth)
            .header("Content-Type", "application/xml")
            .body(propfind::REQUEST);
        let response = http::send(request, "PROPFIND")?;
        let xml = http::text(response)?;
        propfind::parse(&xml).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// The adapter path of a `href` the server sent.
    fn path_of(&self, href: &str) -> PathBuf {
        let decoded = percent_encoding::percent_decode_str(href).decode_utf8_lossy();
        // Absolute URLs are allowed in `href` too.
        let path = Url::parse(&decoded)
            .map(|u| u.path().to_string())
            .unwrap_or_else(|_| decoded.into_owned());
        let base = percent_encoding::percent_decode_str(self.base.path()).decode_utf8_lossy();
        let inner = path.strip_prefix(base.as_ref()).unwrap_or(&path);
        PathBuf::from(remote_path(Path::new(inner)))
    }

    fn entry(&self, item: DavItem) -> VfsEntry {
        let path = self.path_of(&item.href);
        let name = file_name(&remote_path(&path));
        VfsEntry {
            hidden: name.starts_with('.'),
            name,
            path,
            is_dir: item.is_dir,
            size: if item.is_dir { 0 } else { item.size },
            modified: item.modified,
            ..VfsEntry::default()
        }
    }

    /// `MOVE` of `from` to `to`, replacing what is there when `overwrite`.
    fn move_to(&self, from: &Path, to: &Path, overwrite: bool) -> io::Result<()> {
        let is_dir = self.stat(from)?.is_dir;
        let method = Method::from_bytes(b"MOVE").map_err(io::Error::other)?;
        let request = self
            .request(method, self.url(from, is_dir))
            .header("Destination", self.url(to, is_dir).as_str())
            .header("Overwrite", if overwrite { "T" } else { "F" });
        http::send(request, "MOVE").map(drop)
    }
}

impl Vfs for WebDavVfs {
    fn capabilities(&self) -> Capabilities {
        REMOTE
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let own = remote_path(dir);
        Ok(self
            .propfind(dir, "1")?
            .into_iter()
            .map(|item| self.entry(item))
            .filter(|entry| remote_path(&entry.path) != own)
            .collect())
    }

    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let item = self
            .propfind(path, "0")?
            .into_iter()
            .next()
            .ok_or_else(|| not_found(path))?;
        let mut entry = self.entry(item);
        entry.path = path.to_path_buf();
        Ok(entry)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        self.open_read_at(path, 0)
    }

    /// `Range` request; a server that ignores it sends everything, and the
    /// first `offset` bytes are skipped here.
    fn open_read_at(&self, path: &Path, offset: u64) -> io::Result<Box<dyn Read + Send>> {
        let mut request = self.request(Method::GET, self.url(path, false));
        if offset > 0 {
            request = request.header("Range", format!("bytes={offset}-"));
        }
        let response = http::send(request, "GET")?;
        let partial = response.status() == StatusCode::PARTIAL_CONTENT;
        let mut reader = BodyReader::new(response);
        if offset > 0 && !partial {
            io::copy(&mut (&mut reader).take(offset), &mut io::sink())?;
        }
        Ok(Box::new(reader))
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let request = self.request(Method::PUT, self.url(path, false));
        Ok(Box::new(Upload::start(|body| request.body(body), "PUT")))
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.move_to(from, to, true)
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        let method = Method::from_bytes(b"MKCOL").map_err(io::Error::other)?;
        http::send(self.request(method, self.url(path, true)), "MKCOL").map(drop)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        http::send(
            self.request(Method::DELETE, self.url(path, false)),
            "DELETE",
        )
        .map(drop)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        http::send(self.request(Method::DELETE, self.url(path, true)), "DELETE").map(drop)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.move_to(from, to, false)
    }
}
