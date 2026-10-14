//! The S3 adapter (AWS S3 and compatible stores: MinIO, Garage, Ceph,
//! R2...). Requests are signed by `rusty-s3` (sans-IO: it only signs
//! URLs) and sent with the project's HTTP client.
//!
//! S3 has keys, not folders: a folder is a common prefix of keys (listed
//! with the `/` delimiter) or a zero-byte `name/` marker object, which
//! `mkdir` creates. Renames copy and delete (`CopyObject`); writes go up in
//! one `PUT`, or as a multipart upload past the first part (see
//! [`upload`]). Times and modes cannot be set.

mod listing;
mod upload;

use super::http;
use super::{REMOTE, Remote, file_name, invalid, not_found, remote_path};
use crate::fs::vfs::{Capabilities, Vfs, VfsEntry, is_plain_name};
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use url::Url;

/// Validity of signed request URLs.
const SIGNED_FOR: Duration = Duration::from_secs(3600);
/// Region used when the URL and the environment name none.
const DEFAULT_REGION: &str = "us-east-1";
/// Time allowed to connect to the server.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// What every request needs (shared with running uploads).
#[derive(Debug)]
struct Store {
    client: reqwest::Client,
    bucket: Bucket,
    credentials: Option<Credentials>,
    /// Key prefix of the adapter's root (`""` or ending with `/`).
    prefix: String,
}

impl Store {
    /// The key of `path` (`folder`: with the trailing `/` of a folder).
    fn key(&self, path: &Path, folder: bool) -> String {
        let inner = remote_path(path);
        let mut key = format!("{}{}", self.prefix, inner.trim_start_matches('/'));
        if folder && !key.is_empty() && !key.ends_with('/') {
            key.push('/');
        }
        key
    }

    /// Sends the signed request `action` with `method`.
    fn send<'a>(
        &self,
        method: reqwest::Method,
        action: &impl S3Action<'a>,
        what: &str,
    ) -> io::Result<reqwest::Response> {
        http::send(self.client.request(method, action.sign(SIGNED_FOR)), what)
    }

    fn delete(&self, key: &str) -> io::Result<()> {
        let action = self.bucket.delete_object(self.credentials.as_ref(), key);
        self.send(reqwest::Method::DELETE, &action, "DeleteObject")
            .map(drop)
    }

    /// `CopyObject`: `from` copied to `to` on the server.
    fn copy(&self, from: &str, to: &str) -> io::Result<()> {
        let source = format!("/{}/{}", self.bucket.name(), percent_encode(from));
        let mut action = self.bucket.put_object(self.credentials.as_ref(), to);
        action
            .headers_mut()
            .insert("x-amz-copy-source", source.clone());
        let request = self
            .client
            .put(action.sign(SIGNED_FOR))
            .header("x-amz-copy-source", source);
        http::send(request, "CopyObject").map(drop)
    }

    /// One page of the listing of `prefix` (`delimited`: one level).
    fn list_page(
        &self,
        prefix: &str,
        delimited: bool,
        token: Option<&str>,
    ) -> io::Result<listing::Page> {
        let mut action = self.bucket.list_objects_v2(self.credentials.as_ref());
        action.with_prefix(prefix.to_string());
        if delimited {
            action.with_delimiter("/");
        }
        if let Some(token) = token {
            action.with_continuation_token(token.to_string());
        }
        let response = self.send(reqwest::Method::GET, &action, "ListObjectsV2")?;
        let xml = http::text(response)?;
        listing::parse(&xml).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Every page of the listing of `prefix`, handed to `each`.
    fn list_all(
        &self,
        prefix: &str,
        delimited: bool,
        mut each: impl FnMut(listing::Page),
    ) -> io::Result<()> {
        let mut token: Option<String> = None;
        loop {
            let page = self.list_page(prefix, delimited, token.as_deref())?;
            token = page.next_token.clone();
            each(page);
            if token.is_none() {
                return Ok(());
            }
        }
    }
}

/// `key` with the characters a copy source header cannot carry encoded.
fn percent_encode(key: &str) -> String {
    const SET: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'/')
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(key, SET).to_string()
}

/// An S3 time stamp (`2009-10-12T17:50:30.000Z`).
fn s3_time(text: &str) -> Option<SystemTime> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(SystemTime::from)
}

#[derive(Debug, Clone)]
pub struct S3Vfs {
    store: Arc<Store>,
}

impl S3Vfs {
    /// The bucket of `s3://[key:secret@]bucket[/prefix][?endpoint=URL&region=R&path_style=true]`.
    /// Without credentials in the URL, `AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY` are used (none: anonymous requests).
    pub fn connect(url: &Url) -> io::Result<Remote> {
        let name = url
            .host_str()
            .ok_or_else(|| invalid(url.as_str(), "no bucket"))?
            .to_string();
        let query = |key: &str| {
            url.query_pairs()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.into_owned())
        };
        let region = query("region")
            .or_else(|| std::env::var("AWS_REGION").ok())
            .unwrap_or_else(|| DEFAULT_REGION.to_string());
        let endpoint =
            query("endpoint").unwrap_or_else(|| format!("https://s3.{region}.amazonaws.com"));
        let endpoint = Url::parse(&endpoint).map_err(|e| invalid(&endpoint, &e.to_string()))?;
        let style = if query("path_style").is_some_and(|v| v == "true") {
            UrlStyle::Path
        } else {
            UrlStyle::VirtualHost
        };
        let bucket = Bucket::new(endpoint, style, name, region)
            .map_err(|e| invalid(url.as_str(), &e.to_string()))?;
        let credentials = super::credentials(url)
            .map(|(key, secret)| Credentials::new(key, secret))
            .or_else(Credentials::from_env);
        let prefix = url.path().trim_matches('/').to_string();
        let store = Store {
            client: crate::update::http::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .build()
                .map_err(http::io_error)?,
            bucket,
            credentials,
            prefix: if prefix.is_empty() {
                prefix
            } else {
                format!("{prefix}/")
            },
        };
        let vfs = Self {
            store: Arc::new(store),
        };
        vfs.list(Path::new("/"))?;
        Ok(Remote {
            vfs: Arc::new(vfs),
            start: PathBuf::from("/"),
        })
    }

    /// The entries of `dir` from one level of keys.
    fn entries(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        let prefix = self.store.key(dir, true);
        let mut entries = Vec::new();
        self.store.list_all(&prefix, true, |page| {
            for folder in page.prefixes {
                let name = file_name(&folder);
                if !is_plain_name(&name) {
                    continue;
                }
                entries.push(VfsEntry::dir(name.clone(), dir.join(name)));
            }
            for object in page.objects {
                // The folder's own marker.
                if object.key == prefix || object.key.ends_with('/') {
                    continue;
                }
                let name = file_name(&object.key);
                if !is_plain_name(&name) {
                    continue;
                }
                entries.push(VfsEntry {
                    hidden: name.starts_with('.'),
                    path: dir.join(&name),
                    name,
                    size: object.size,
                    modified: s3_time(&object.last_modified),
                    ..VfsEntry::default()
                });
            }
        })?;
        Ok(entries)
    }

    /// Every key below the folder `dir` (markers included).
    fn keys_below(&self, dir: &Path) -> io::Result<Vec<String>> {
        let mut keys = Vec::new();
        self.store
            .list_all(&self.store.key(dir, true), false, |page| {
                keys.extend(page.objects.into_iter().map(|o| o.key));
            })?;
        Ok(keys)
    }
}

impl Vfs for S3Vfs {
    fn capabilities(&self) -> Capabilities {
        REMOTE
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<VfsEntry>> {
        self.entries(dir)
    }

    /// An object (`HEAD`), else a folder when keys lie below it.
    fn stat(&self, path: &Path) -> io::Result<VfsEntry> {
        let name = file_name(&remote_path(path));
        let key = self.store.key(path, false);
        if key.is_empty() || key.ends_with('/') {
            return Ok(VfsEntry::dir(name, path.to_path_buf()));
        }
        let store = &self.store;
        let action = store.bucket.head_object(store.credentials.as_ref(), &key);
        if let Ok(response) = store.send(reqwest::Method::HEAD, &action, "HeadObject") {
            let header = |h| response.headers().get(h).and_then(|v| v.to_str().ok());
            return Ok(VfsEntry {
                hidden: name.starts_with('.'),
                path: path.to_path_buf(),
                size: header("content-length")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
                modified: header("last-modified")
                    .and_then(|v| chrono::DateTime::parse_from_rfc2822(v).ok())
                    .map(SystemTime::from),
                name,
                ..VfsEntry::default()
            });
        }
        let folder = store.key(path, true);
        let page = store.list_page(&folder, true, None)?;
        if page.objects.is_empty() && page.prefixes.is_empty() {
            return Err(not_found(path));
        }
        Ok(VfsEntry::dir(name, path.to_path_buf()))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn Read + Send>> {
        self.open_read_at(path, 0)
    }

    fn open_read_at(&self, path: &Path, offset: u64) -> io::Result<Box<dyn Read + Send>> {
        let key = self.store.key(path, false);
        let action = self
            .store
            .bucket
            .get_object(self.store.credentials.as_ref(), &key);
        let mut request = self.store.client.get(action.sign(SIGNED_FOR));
        if offset > 0 {
            request = request.header("Range", format!("bytes={offset}-"));
        }
        let response = http::send(request, "GetObject")?;
        Ok(Box::new(http::body_from(response, offset)?))
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let key = self.store.key(path, false);
        Ok(Box::new(upload::S3Upload::new(
            Arc::clone(&self.store),
            key,
        )))
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.rename(from, to)
    }

    /// A zero-byte `name/` marker.
    fn mkdir(&self, path: &Path) -> io::Result<()> {
        let key = self.store.key(path, true);
        let action = self
            .store
            .bucket
            .put_object(self.store.credentials.as_ref(), &key);
        let request = self
            .store
            .client
            .put(action.sign(SIGNED_FOR))
            .body(Vec::new());
        http::send(request, "PutObject").map(drop)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.store.delete(&self.store.key(path, false))
    }

    /// Its marker (a folder without one disappears with its last key).
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.store.delete(&self.store.key(path, true))
    }

    /// Copy and delete: one object, or every key below a folder.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if !self.stat(from)?.is_dir {
            let (src, dst) = (self.store.key(from, false), self.store.key(to, false));
            self.store.copy(&src, &dst)?;
            return self.store.delete(&src);
        }
        let (src, dst) = (self.store.key(from, true), self.store.key(to, true));
        for key in self.keys_below(from)? {
            let moved = format!("{dst}{}", &key[src.len()..]);
            self.store.copy(&key, &moved)?;
            self.store.delete(&key)?;
        }
        Ok(())
    }
}
