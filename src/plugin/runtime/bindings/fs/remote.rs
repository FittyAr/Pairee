//! `pairee.fs` on remote URLs (`sftp://user@host:22/dir/file`, `ftp://`,
//! `ftps://`, `webdav://`, `webdavs://`, `s3://`).
//!
//! A remote URL reaches a connection that is **open in a panel tab** (the
//! main loop looks it up by scheme, host, port and user; plugins never
//! open connections or see secrets). What a plugin may do follows its
//! local jail:
//!
//! * untrusted plugins: nothing (they cannot read the user's local files
//!   either), every call is refused;
//! * trusted plugins in Secure Mode: read only (`read`, `exists`, `stat`,
//!   `file`, `list`, `read_dir`, `cha`);
//! * trusted plugins outside Secure Mode: also `write`, `mkdir`, `remove`
//!   and `rename` (within one connection).
//!
//! The functions stay usable synchronously for local paths: each one is
//! wrapped so a local argument runs the local binding as before and only a
//! remote one awaits the network (on the blocking pool).

use super::jail::Access;
use crate::fs::vfs::{Vfs, VfsEntry};
use crate::plugin::manager::PluginRequest;
use crate::plugin::runtime::types::{LuaError, LuaFile, LuaUrl};
use mlua::{Function, Lua, MultiValue, Table, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// What a plugin may do on remote URLs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteMode {
    Denied,
    ReadOnly,
    ReadWrite,
}

impl RemoteMode {
    pub fn new(trusted: bool, secure_mode: bool) -> Self {
        match (trusted, secure_mode) {
            (false, _) => Self::Denied,
            (true, true) => Self::ReadOnly,
            (true, false) => Self::ReadWrite,
        }
    }
}

/// The remote side of `pairee.fs`, captured by its closures.
#[derive(Clone)]
pub struct RemoteAccess {
    pub mode: RemoteMode,
    pub tx: mpsc::Sender<PluginRequest>,
}

/// A connection found for a URL, and the path on it.
struct Located {
    vfs: Arc<dyn Vfs>,
    path: PathBuf,
    /// `scheme://user@host:port`, to build URLs of listed files.
    prefix: String,
}

fn refused(message: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(format!("Security violation: {}", message.into()))
}

fn io_error(e: std::io::Error) -> mlua::Error {
    mlua::Error::RuntimeError(e.to_string())
}

/// The remote URL `value` names, if it is one (a `Url` with a scheme or a
/// `scheme://` string).
pub fn remote_url(value: &Value) -> Option<LuaUrl> {
    let url = match value {
        Value::String(s) => LuaUrl::parse(&s.to_string_lossy()),
        Value::UserData(ud) => ud.borrow::<LuaUrl>().ok()?.clone(),
        _ => return None,
    };
    url.scheme.is_some().then_some(url)
}

impl RemoteAccess {
    /// The open connection `url` names, if the plugin may `access` it.
    async fn locate(&self, url: &LuaUrl, access: Access) -> mlua::Result<Located> {
        match (self.mode, access) {
            (RemoteMode::Denied, _) => {
                return Err(refused("untrusted plugins cannot use remote URLs"));
            }
            (RemoteMode::ReadOnly, Access::Write) => {
                return Err(refused("Secure Mode only lets plugins read remote URLs"));
            }
            _ => {}
        }
        let (reply, answer) = oneshot::channel();
        let request = PluginRequest::RemoteConnection {
            scheme: url.scheme.clone().unwrap_or_default(),
            domain: url.domain.clone().unwrap_or_default(),
            reply,
        };
        let not_open = || {
            mlua::Error::RuntimeError(format!("no open connection for {url} (open it in a panel)"))
        };
        self.tx.send(request).await.map_err(|_| not_open())?;
        let conn = answer.await.ok().flatten().ok_or_else(not_open)?;
        Ok(Located {
            vfs: conn.vfs(),
            path: url.path.clone(),
            prefix: conn.info().url_prefix(),
        })
    }
}

/// Runs `work` on the blocking pool (network I/O).
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> mlua::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?
        .map_err(io_error)
}

fn file_of(prefix: &str, entry: &VfsEntry) -> LuaFile {
    let path = entry.path.to_string_lossy().replace('\\', "/");
    LuaFile {
        name: entry.name.clone(),
        url: format!("{prefix}/{}", path.trim_start_matches('/')),
        path,
        size: entry.size,
        is_dir: entry.is_dir,
        is_symlink: entry.is_symlink,
        mime: crate::plugin::manager::snapshot::guess_mime(&entry.name, entry.is_dir),
        mtime: crate::plugin::manager::snapshot::mtime_unix(entry.modified),
        is_hidden: entry.hidden,
        is_exec: false,
    }
}

/// The access an operation needs.
fn access_of(op: &str) -> Access {
    match op {
        "write" | "mkdir" | "remove" | "rename" => Access::Write,
        _ => Access::Read,
    }
}

/// Runs `op` on the remote `url` (`args` are the Lua arguments).
async fn run(
    lua: &Lua,
    access: &RemoteAccess,
    op: &str,
    url: LuaUrl,
    args: Vec<Value>,
) -> mlua::Result<MultiValue> {
    let at = access.locate(&url, access_of(op)).await?;
    let (vfs, path) = (at.vfs.clone(), at.path.clone());
    let one = |v: Value| Ok(MultiValue::from_iter([v]));
    match op {
        "read" => {
            let bytes = blocking(move || super::target::read_capped(vfs.open_read(&path)?)).await?;
            one(Value::String(lua.create_string(bytes)?))
        }
        "write" => {
            let data = match args.get(1) {
                Some(Value::String(s)) => s.as_bytes().to_vec(),
                _ => return Err(mlua::Error::RuntimeError("write expects a string".into())),
            };
            blocking(move || vfs.write_file(&path, &mut data.as_slice())).await?;
            one(Value::Nil)
        }
        "exists" => one(Value::Boolean(
            blocking(move || Ok(vfs.exists(&path))).await?,
        )),
        "stat" | "file" => match blocking(move || vfs.stat(&path)).await {
            Ok(entry) => one(Value::UserData(
                lua.create_userdata(file_of(&at.prefix, &entry))?,
            )),
            Err(_) if op == "stat" => one(Value::Nil),
            Err(e) => Err(e),
        },
        "list" | "read_dir" => {
            let entries = blocking(move || vfs.list(&path)).await?;
            let list = lua.create_table()?;
            for (i, entry) in entries.iter().enumerate() {
                list.set(i + 1, file_of(&at.prefix, entry))?;
            }
            one(Value::Table(list))
        }
        "cha" => {
            let result = blocking(move || vfs.stat(&path))
                .await
                .map(|e| file_of(&at.prefix, &e).cha())
                .map_err(|e| LuaError::custom(e.to_string()));
            LuaError::pair(lua, result)
        }
        _ => mutate(access, op, at, &args)
            .await
            .map(|()| MultiValue::new()),
    }
}

/// `mkdir`, `remove` and `rename` on a remote URL.
async fn mutate(access: &RemoteAccess, op: &str, at: Located, args: &[Value]) -> mlua::Result<()> {
    let kind = match args.first() {
        Some(Value::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    let (vfs, path) = (at.vfs, at.path);
    match op {
        "mkdir" => {
            blocking(move || {
                if kind == "dir_all" {
                    vfs.mkdir_all(&path);
                    Ok(())
                } else {
                    vfs.mkdir(&path)
                }
            })
            .await
        }
        "remove" => {
            blocking(move || match kind.as_str() {
                "file" => vfs.remove_file(&path),
                "dir_all" => vfs.remove_all(&path),
                _ => vfs.remove_dir(&path),
            })
            .await
        }
        "rename" => {
            let to = args.get(1).and_then(remote_url).ok_or_else(|| {
                mlua::Error::RuntimeError("rename: both URLs must be on one connection".into())
            })?;
            let target = access.locate(&to, Access::Write).await?;
            if target.prefix != at.prefix {
                return Err(mlua::Error::RuntimeError(
                    "rename: both URLs must be on one connection".into(),
                ));
            }
            blocking(move || vfs.rename(&path, &target.path)).await
        }
        _ => Err(mlua::Error::RuntimeError(format!(
            "pairee.fs.{op} does not work on remote URLs"
        ))),
    }
}

#[cfg(test)]
mod tests;

/// Which argument of each wrapped function is the location.
const REMOTE_OPS: &[(&str, usize)] = &[
    ("read", 0),
    ("write", 0),
    ("exists", 0),
    ("stat", 0),
    ("file", 0),
    ("list", 0),
    ("read_dir", 0),
    ("cha", 0),
    ("mkdir", 1),
    ("remove", 1),
    ("rename", 0),
    ("copy", 0),
];

/// Wraps the local `pairee.fs` functions so remote URLs go to the open
/// connection (or are refused) and local paths run as before.
pub fn wrap(lua: &Lua, fs: &Table, access: RemoteAccess) -> mlua::Result<()> {
    for (op, at) in REMOTE_OPS {
        let local: Function = fs.get(*op)?;
        let access = access.clone();
        let wrapped = lua.create_async_function(move |lua, args: MultiValue| {
            let local = local.clone();
            let access = access.clone();
            async move {
                let args: Vec<Value> = args.into_iter().collect();
                let remote = args.get(*at).and_then(remote_url);
                // `copy` / `rename` with a remote second argument.
                let two_sided = matches!(*op, "copy" | "rename");
                let remote_target = two_sided && args.get(1).and_then(remote_url).is_some();
                match remote {
                    None if !remote_target => local.call::<MultiValue>(MultiValue::from_iter(args)),
                    _ if *op == "copy" => Err(mlua::Error::RuntimeError(
                        "pairee.fs.copy works on local paths; read and write remote files instead"
                            .into(),
                    )),
                    None => Err(mlua::Error::RuntimeError(
                        "rename: both URLs must be on one connection".into(),
                    )),
                    Some(url) => run(&lua, &access, op, url, args).await,
                }
            }
        })?;
        fs.set(*op, wrapped)?;
    }
    Ok(())
}
