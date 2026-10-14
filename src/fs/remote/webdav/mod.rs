//! The WebDAV adapter (RFC 4918 class 1): `PROPFIND` lists, `GET` reads
//! (with `Range` for resumed copies), `PUT` writes as a stream, `MKCOL`,
//! `DELETE` and `MOVE` change the tree. Basic authentication.

mod propfind;

use super::http::{self, Upload};
use super::tls::{self, Handshake, TlsTrust};
use super::{
    REMOTE, Remote, credentials, file_name, invalid, not_found, percent_decoded, remote_path,
};
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry, is_plain_name};
use propfind::DavItem;
use reqwest::Method;
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
        let mut builder = crate::update::http::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT);
        let host = base.host_str().unwrap_or_default().to_string();
        let port = base.port_or_known_default().unwrap_or(443);
        let trust = match secure {
            true => Some(TlsTrust::for_server(url, &host, port)?),
            false => None,
        };
        if let Some(trust) = &trust {
            builder = trust.apply(builder)?;
        }
        let client = builder.build().map_err(http::io_error)?;
        let vfs = Self { client, base, auth };
        vfs.stat(Path::new("/")).map_err(|e| match &trust {
            Some(trust) => tls::classify(e, &host, port, Handshake::Direct, trust),
            None => e,
        })?;
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
        PathBuf::from(href_path(self.base.path(), href))
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

/// The adapter path of `href` below the server path `base`: an absolute
/// URL or a path, parsed first and percent-decoded after (decoding first
/// would let an encoded `?` or `#` in a name cut it short, and the URL
/// parser would re-encode the rest). Paths outside `base` are kept whole.
fn href_path(base: &str, href: &str) -> String {
    let raw = Url::parse(href)
        .map(|u| u.path().to_string())
        .unwrap_or_else(|_| {
            href.split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_string()
        });
    let path = percent_decoded(&raw);
    let base = percent_decoded(base);
    // Component-wise: `/dav` is not a prefix of `/davfoo/x`.
    let inner = Path::new(&path)
        .strip_prefix(Path::new(&base))
        .unwrap_or(Path::new(&path));
    remote_path(inner)
}

impl Vfs for WebDavVfs {
    fn capabilities(&self) -> Capabilities {
        REMOTE
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let own = remote_path(dir);
        // Only direct children with plain names (a hostile `href` can name
        // any path on the server).
        let child_of_dir =
            |entry: &VfsEntry| entry.path.parent().is_some_and(|p| remote_path(p) == own);
        Ok(self
            .propfind(dir, "1")?
            .into_iter()
            .map(|item| self.entry(item))
            .filter(|entry| remote_path(&entry.path) != own)
            .filter(|entry| is_plain_name(&entry.name) && child_of_dir(entry))
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
        Ok(Box::new(http::body_from(response, offset)?))
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

    /// `DELETE` of a collection takes its contents with it: only an empty
    /// one is deleted.
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        crate::fs::vfs::ensure_empty_dir(self, path)?;
        http::send(self.request(Method::DELETE, self.url(path, true)), "DELETE").map(drop)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.move_to(from, to, false)
    }
}

#[cfg(test)]
mod tests {
    use super::href_path;

    #[test]
    fn hrefs_are_decoded_after_parsing() {
        assert_eq!(
            href_path("/dav", "/dav/My%20Docs/a%25b.txt"),
            "/My Docs/a%b.txt"
        );
        assert_eq!(
            href_path("/dav", "https://h:8080/dav/%C3%A9l/ni%C3%B1o.txt"),
            "/él/niño.txt"
        );
        // An encoded `?` belongs to the name, not to a query.
        assert_eq!(href_path("", "/q%3Fx/f"), "/q?x/f");
    }

    #[test]
    fn the_base_is_stripped_by_whole_components() {
        assert_eq!(href_path("/dav", "/dav/x"), "/x");
        assert_eq!(href_path("/dav", "/davfoo/x"), "/davfoo/x");
        assert_eq!(href_path("/dav", "/dav"), "/");
        assert_eq!(href_path("/My Docs", "/My%20Docs/f"), "/f");
    }
}
