//! `Access` builder + `Fd` file-descriptor userdata (M3
//! roadmap §5.B6).
//!
//! `fs.access():read(true):write(true):open(url) → Fd` is the
//! builder pattern. M3 provides a thin shim around `std::fs::File`
//! exposed to Lua; the read/write streaming is intentionally
//! minimal (we don't expose `AsyncRead`/`AsyncWrite` to Lua yet —
//! that's a follow-up).

use mlua::{UserData, UserDataMethods};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Builder for an `Fd`. Created by `fs.access()`.
#[derive(Debug, Clone, Default)]
pub struct Access {
    read: bool,
    write: bool,
    create: bool,
    truncate: bool,
    append: bool,
}

impl UserData for Access {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("read", |_lua, this, yes: bool| {
            if yes {
                this.read = true;
            }
            let _ = yes;
            Ok(this.clone())
        });
        methods.add_method_mut("write", |_lua, this, yes: bool| {
            if yes {
                this.write = true;
            }
            let _ = yes;
            Ok(this.clone())
        });
        methods.add_method_mut("create", |_lua, this, yes: bool| {
            this.create = yes;
            Ok(this.clone())
        });
        methods.add_method_mut("truncate", |_lua, this, yes: bool| {
            this.truncate = yes;
            Ok(this.clone())
        });
        methods.add_method_mut("append", |_lua, this, yes: bool| {
            this.append = yes;
            Ok(this.clone())
        });
        methods.add_async_method("open", |lua, this, url: String| async move {
            let path = PathBuf::from(url);
            let mut opts = std::fs::OpenOptions::new();
            opts.read(this.read)
                .write(this.write)
                .create(this.create)
                .truncate(this.truncate)
                .append(this.append);
            let file = opts
                .open(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("Access.open failed: {e}")))?;
            let fd = Fd { inner: Some(file) };
            let ud = lua.create_userdata(fd)?;
            Ok(ud)
        });
    }
}

/// Register `fs.access()` on the given `fs` table.
pub fn register(lua: &mlua::Lua, fs_table: &mlua::Table<'_>) -> mlua::Result<()> {
    fs_table.set(
        "access",
        lua.create_function(|lua, ()| {
            let a = Access::default();
            lua.create_userdata(a).map(mlua::Value::UserData)
        })?,
    )?;
    Ok(())
}

/// The M3 `Fd` userdata. M3 only exposes synchronous
/// `read`/`write`/`seek`; the streaming async surface is a
/// follow-up.
pub struct Fd {
    pub inner: Option<File>,
}

impl UserData for Fd {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("read", |_lua, this, len: usize| {
            let mut buf = vec![0u8; len];
            let file = this.inner.as_ref().ok_or_else(|| {
                mlua::Error::RuntimeError("Fd.read: handle already consumed".to_string())
            })?;
            let n = file
                .read(&mut buf)
                .map_err(|e| mlua::Error::RuntimeError(format!("Fd.read failed: {e}")))?;
            buf.truncate(n);
            Ok(mlua::Value::String(_lua.create_string(&buf)?))
        });
        methods.add_method_mut("write_all", |_lua, this, src: mlua::String| {
            let file = this.inner.as_mut().ok_or_else(|| {
                mlua::Error::RuntimeError("Fd.write_all: handle already consumed".to_string())
            })?;
            file.write_all(src.as_bytes())
                .map_err(|e| mlua::Error::RuntimeError(format!("Fd.write_all failed: {e}")))?;
            file.flush()
                .map_err(|e| mlua::Error::RuntimeError(format!("Fd.flush failed: {e}")))?;
            Ok(true)
        });
        methods.add_method_mut("seek", |_lua, this, offset: i64| {
            let file = this.inner.as_mut().ok_or_else(|| {
                mlua::Error::RuntimeError("Fd.seek: handle already consumed".to_string())
            })?;
            file.seek(SeekFrom::Start(offset as u64))
                .map(|n| n as i64)
                .map_err(|e| mlua::Error::RuntimeError(format!("Fd.seek failed: {e}")))
        });
        methods.add_method_mut("close", |_lua, this, ()| {
            // Drop the inner file (closes the OS handle).
            this.inner.take();
            Ok(true)
        });
    }
}
