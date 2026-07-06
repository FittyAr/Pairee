use crate::plugin::manager::PluginRequest;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use tokio::sync::mpsc;

fn is_secure_mode(lua: &mlua::Lua) -> bool {
    if let Ok(pairee) = lua.globals().get::<_, mlua::Table>("pairee") {
        pairee.get::<_, bool>("_secure_mode").unwrap_or(false)
    } else {
        false
    }
}

fn validate_path(lua: &mlua::Lua, path_str: &str) -> mlua::Result<PathBuf> {
    let path = PathBuf::from(path_str);
    if is_secure_mode(lua) {
        let abs_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let workspace = std::env::current_dir().unwrap_or_default();
        let config = crate::config::paths::get_config_dir();
        let cache = crate::config::paths::get_cache_dir();

        let in_workspace = abs_path.starts_with(&workspace);
        let in_config = abs_path.starts_with(&config);
        let in_cache = abs_path.starts_with(&cache);

        if !in_workspace && !in_config && !in_cache {
            return Err(mlua::Error::RuntimeError(format!(
                "Security violation: path {:?} is outside permitted sandboxed directories in Secure Mode",
                path
            )));
        }
    }
    Ok(path)
}

pub fn bind(
    lua: &mlua::Lua,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<mlua::Table<'_>> {
    let fs = lua.create_table()?;

    // read(path) — async via tokio::fs (M3 roadmap §5.B1).
    fs.set(
        "read",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path(lua_ctx, &path_str)?;
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read file: {}", e)))
        })?,
    )?;

    // write(path, data) — async via tokio::fs (M3 roadmap §5.B1).
    fs.set(
        "write",
        lua.create_async_function(
            move |lua_ctx, (path_str, data): (String, String)| async move {
                let path = validate_path(lua_ctx, &path_str)?;
                tokio::fs::write(&path, data)
                    .await
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to write file: {}", e)))
            },
        )?,
    )?;

    // exists(path) — async, non-blocking existence check (M3 §5.B1).
    fs.set(
        "exists",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path(lua_ctx, &path_str)?;
            Ok(tokio::fs::metadata(&path).await.is_ok())
        })?,
    )?;

    // stat(path) — async via tokio::fs::metadata (M3 §5.B1).
    fs.set(
        "stat",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path(lua_ctx, &path_str)?;
            let m = match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(_) => return Ok(mlua::Value::Nil),
            };
            let is_dir = m.is_dir();
            let is_symlink = m.is_symlink();
            let size = m.len();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let t = lua_ctx.create_table()?;
            t.set("name", name)?;
            t.set("url", path_str.clone())?;
            t.set("path", path_str)?;
            t.set("size", size)?;
            t.set("is_dir", is_dir)?;
            t.set("is_symlink", is_symlink)?;
            Ok(mlua::Value::Table(t))
        })?,
    )?;

    // list(path) — async via tokio::fs::read_dir (M3 §5.B1).
    // We must allocate the path string from each `Entry` *before*
    // any `.await` (an `Entry` cannot be held across an await
    // point because of the `!Send` / lifetime issues in the
    // tokio `read_dir` API).
    fs.set(
        "list",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path(lua_ctx, &path_str)?;
            let mut entries_vec = Vec::new();
            let mut rd = match tokio::fs::read_dir(&path).await {
                Ok(rd) => rd,
                Err(_) => return Ok(entries_vec),
            };
            loop {
                // Pull the next entry synchronously. If the read
                // errors, just bail out (the legacy behaviour
                // returned an empty list for read errors).
                let next = match rd.next_entry().await {
                    Ok(Some(e)) => e,
                    Ok(None) => break,
                    Err(_) => break,
                };
                // Allocate all the data we need *before* awaiting on
                // metadata.
                let p = next.path();
                let p_str = p.to_string_lossy().to_string();
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                // `tokio::fs::metadata` requires the path to live
                // across the await — clone the path buffer into a
                // local `PathBuf` first to avoid borrowing the
                // `Entry`.
                let p_owned: std::path::PathBuf = p.clone();
                let (is_dir, is_symlink, size) = match tokio::fs::metadata(&p_owned).await {
                    Ok(m) => (m.is_dir(), m.is_symlink(), m.len()),
                    Err(_) => (false, false, 0),
                };

                let t = lua_ctx.create_table()?;
                t.set("name", name)?;
                t.set("url", p_str.clone())?;
                t.set("path", p_str)?;
                t.set("size", size)?;
                t.set("is_dir", is_dir)?;
                t.set("is_symlink", is_symlink)?;
                entries_vec.push(t);
            }
            Ok(entries_vec)
        })?,
    )?;

    // spawn(cmd, args)
    fs.set("spawn", lua.create_async_function(move |lua_ctx, (cmd, args): (String, Vec<String>)| {
        async move {
            if !trusted {
                return Err(mlua::Error::RuntimeError(
                    "Security violation: spawning external processes is blocked in sandboxed mode.".to_string()
                ));
            }
            if is_secure_mode(lua_ctx) && !crate::plugin::sandbox::is_command_safe(&cmd) {
                return Err(mlua::Error::RuntimeError(format!(
                    "Security violation: Command '{}' is blacklisted in Secure Mode",
                    cmd
                )));
            }

            // Execute process
            let output = tokio::process::Command::new(&cmd)
                .args(&args)
                .output()
                .await;

            match output {
                Ok(out) => {
                    let t = lua_ctx.create_table()?;
                    t.set("stdout", String::from_utf8_lossy(&out.stdout).to_string())?;
                    t.set("stderr", String::from_utf8_lossy(&out.stderr).to_string())?;
                    t.set("status", out.status.code().unwrap_or(0))?;
                    Ok(t)
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!("Failed to spawn process: {}", e))),
            }
        }
    })?)?;

    // spawn_copy_task(from, to)
    let tx_copy = tx.clone();
    fs.set(
        "spawn_copy_task",
        lua.create_async_function(move |lua_ctx, (from_str, to_str): (String, String)| {
            let tx = tx_copy.clone();
            async move {
                let from = validate_path(lua_ctx, &from_str)?;
                let to = validate_path(lua_ctx, &to_str)?;
                let _ = tx.send(PluginRequest::SpawnCopyTask { from, to }).await;
                Ok(())
            }
        })?,
    )?;

    // ── M3: new `fs.*` operations per roadmap §5.B2 ─────────────

    // mkdir(type, url) — `type ∈ {"dir", "dir_all"}`.
    fs.set(
        "mkdir",
        lua.create_function(move |lua_ctx, (kind, url): (String, String)| {
            let path = validate_path(lua_ctx, &url)?;
            let recursive = kind == "dir_all";
            let res = if recursive {
                std::fs::create_dir_all(&path)
            } else {
                std::fs::create_dir(&path)
            };
            res.map_err(|e| mlua::Error::RuntimeError(format!("mkdir failed: {e}")))
        })?,
    )?;

    // remove(type, url) — `type ∈ {"file", "dir", "dir_all", "dir_clean"}`.
    fs.set(
        "remove",
        lua.create_function(move |lua_ctx, (kind, url): (String, String)| {
            let path = validate_path(lua_ctx, &url)?;
            let res = match kind.as_str() {
                "file" => std::fs::remove_file(&path),
                "dir" => std::fs::remove_dir(&path),
                "dir_all" => std::fs::remove_dir_all(&path),
                // "dir_clean" = empty the directory but keep it.
                "dir_clean" => {
                    let mut failed = None;
                    if let Ok(entries) = std::fs::read_dir(&path) {
                        for entry in entries.flatten() {
                            let ep = entry.path();
                            let r = if ep.is_dir() {
                                std::fs::remove_dir_all(&ep)
                            } else {
                                std::fs::remove_file(&ep)
                            };
                            if let Err(e) = r {
                                failed = Some(e);
                                break;
                            }
                        }
                    }
                    match failed {
                        Some(e) => Err(e),
                        None => Ok(()),
                    }
                }
                other => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "fs.remove: unknown type {other:?}"
                    )));
                }
            };
            res.map_err(|e| mlua::Error::RuntimeError(format!("remove failed: {e}")))
        })?,
    )?;

    // rename(from, to)
    fs.set(
        "rename",
        lua.create_function(move |lua_ctx, (from, to): (String, String)| {
            let from_path = validate_path(lua_ctx, &from)?;
            let to_path = validate_path(lua_ctx, &to)?;
            std::fs::rename(&from_path, &to_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("rename failed: {e}")))
        })?,
    )?;

    // copy(from, to) — sync (returns the number of bytes copied).
    fs.set(
        "copy",
        lua.create_function(move |lua_ctx, (from, to): (String, String)| {
            let from_path = validate_path(lua_ctx, &from)?;
            let to_path = validate_path(lua_ctx, &to)?;
            if let Some(parent) = to_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::copy(&from_path, &to_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("copy failed: {e}")))
        })?,
    )?;

    // read_dir(url, {glob?, limit?, resolve?}) — return `File[]`.
    fs.set(
        "read_dir",
        lua.create_function(move |lua_ctx, (url, opts): (String, mlua::Table)| {
            let path = validate_path(lua_ctx, &url)?;
            let limit: Option<usize> = opts.get("limit").ok();
            let _glob: Option<String> = opts.get("glob").ok();
            let mut files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if let Some(limit) = limit {
                        if files.len() >= limit {
                            break;
                        }
                    }
                    let url = crate::plugin::types::Url::parse(&p.to_string_lossy());
                    let cha = match std::fs::metadata(&p) {
                        Ok(m) => crate::plugin::types::Cha::from_metadata(&m, true),
                        Err(_) => crate::plugin::types::Cha::dummy(),
                    };
                    let f = crate::plugin::types::File {
                        url,
                        cha,
                        link_to: None,
                    };
                    let ud = lua_ctx.create_userdata(f)?;
                    files.push(mlua::Value::UserData(ud));
                }
            }
            Ok(files)
        })?,
    )?;

    // cha(url, follow?) — return Cha userdata.
    fs.set(
        "cha",
        lua.create_function(move |lua_ctx, (url, follow): (String, Option<bool>)| {
            let path = validate_path(lua_ctx, &url)?;
            let follow = follow.unwrap_or(true);
            match std::fs::metadata(&path) {
                Ok(m) => {
                    let cha = crate::plugin::types::Cha::from_metadata(&m, follow);
                    lua_ctx.create_userdata(cha).map(mlua::Value::UserData)
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!("fs.cha failed: {e}"))),
            }
        })?,
    )?;

    // file(url) — return File userdata.
    fs.set(
        "file",
        lua.create_function(move |lua_ctx, url: String| {
            let path = validate_path(lua_ctx, &url)?;
            let url = crate::plugin::types::Url::parse(&path.to_string_lossy());
            let f = match std::fs::metadata(&path) {
                Ok(m) => crate::plugin::types::File::from_url_and_metadata(url, m, true),
                Err(_) => crate::plugin::types::File::from_url(url),
            };
            lua_ctx.create_userdata(f).map(mlua::Value::UserData)
        })?,
    )?;

    // ── More M3 fs.* operations per roadmap §5.B2 ────────────────

    // unique(type, url) — return a unique Url. `type ∈ {"file", "dir",
    // "dir_all", "none"}` controls the create-before-return mode.
    // Synchronous helper (it's a single O(1) `exists` probe plus an
    // optional create; not a hot path).
    fs.set(
        "unique",
        lua.create_function(move |lua_ctx, (kind, url): (String, String)| {
            let base = validate_path(lua_ctx, &url)?;
            for _ in 0..16 {
                let mut hasher = DefaultHasher::new();
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
                    .hash(&mut hasher);
                kind.hash(&mut hasher);
                url.hash(&mut hasher);
                let h = hasher.finish();
                // 6-char ascii-lowercase hash.
                let tag: String = format!("{:012x}", h & 0xffffffffffff)
                    .chars()
                    .take(6)
                    .collect();
                let mut candidate = base.clone().into_os_string();
                candidate.push(format!(".{tag}"));
                let candidate_path = std::path::PathBuf::from(candidate);
                if !candidate_path.exists() {
                    let result_path = match kind.as_str() {
                        "dir" => {
                            std::fs::create_dir(&candidate_path)
                                .map_err(|e| mlua::Error::RuntimeError(format!(
                                    "fs.unique: create_dir failed: {e}"
                                )))?;
                            candidate_path
                        }
                        "dir_all" => {
                            std::fs::create_dir_all(&candidate_path)
                                .map_err(|e| mlua::Error::RuntimeError(format!(
                                    "fs.unique: create_dir_all failed: {e}"
                                )))?;
                            candidate_path
                        }
                        "file" => {
                            std::fs::File::create(&candidate_path)
                                .map_err(|e| mlua::Error::RuntimeError(format!(
                                    "fs.unique: File::create failed: {e}"
                                )))?;
                            candidate_path
                        }
                        "none" => candidate_path,
                        other => {
                            return Err(mlua::Error::RuntimeError(format!(
                                "fs.unique: unknown type {other:?}"
                            )));
                        }
                    };
                    let validated = validate_path(
                        lua_ctx,
                        &result_path.to_string_lossy(),
                    )?;
                    return lua_ctx
                        .create_userdata(crate::plugin::types::Url::parse(
                            &validated.to_string_lossy(),
                        ))
                        .map(mlua::Value::UserData);
                }
            }
            Err(mlua::Error::RuntimeError(
                "fs.unique: could not find a free slot after 16 attempts".to_string(),
            ))
        })?,
    )?;

    // expand_url(value) — coalesce a string or a Url userdata into
    // a Url userdata. Strings are parsed via `Url::parse`; Url
    // userdata are cloned.
    fs.set(
        "expand_url",
        lua.create_function(move |lua_ctx, value: mlua::Value| {
            match value {
                mlua::Value::String(s) => {
                    let s = s.to_str().map_err(|e| {
                        mlua::Error::RuntimeError(format!("fs.expand_url: {e}"))
                    })?;
                    let u = crate::plugin::types::Url::parse(s);
                    lua_ctx.create_userdata(u).map(mlua::Value::UserData)
                }
                mlua::Value::UserData(ud) => {
                    let url = ud.borrow::<crate::plugin::types::Url>().map_err(|e| {
                        mlua::Error::RuntimeError(format!(
                            "fs.expand_url: expected Url userdata: {e}"
                        ))
                    })?;
                    let cloned = url.clone();
                    lua_ctx
                        .create_userdata(cloned)
                        .map(mlua::Value::UserData)
                }
                other => Err(mlua::Error::RuntimeError(format!(
                    "fs.expand_url: expected string or Url, got {}",
                    other.type_name()
                ))),
            }
        })?,
    )?;

    // partitions() — return a `Vec<Partition>` where each Partition
    // is a small Lua table `{ path, label, fstype }`. Platform-specific:
    // Unix parses /proc/mounts (or /etc/mtab fallback), Windows
    // enumerates A–Z drive letters. macOS returns an empty list
    // with a TODO (M3 simplification).
    #[cfg(unix)]
    {
        fs.set("partitions", lua.create_function(move |lua_ctx, ()| {
            let mut seen_mounts: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let skip_fstypes: std::collections::HashSet<&str> = [
                "proc", "sysfs", "cgroup", "cgroup2", "tmpfs", "devtmpfs",
                "securityfs", "pstore", "mqueue", "hugetlbfs", "debugfs",
                "tracefs", "configfs", "fusectl", "bpf", "fuse.gvfsd-fuse",
                "fuse.portal",
            ]
            .iter()
            .copied()
            .collect();
            let mut out: Vec<mlua::Value> = Vec::new();
            let source = if std::path::Path::new("/proc/mounts").exists() {
                "/proc/mounts"
            } else {
                "/etc/mtab"
            };
            if let Ok(s) = std::fs::read_to_string(source) {
                for line in s.lines() {
                    let mut parts = line.split_whitespace();
                    let _dev = parts.next();
                    let mountpoint = match parts.next() {
                        Some(m) => m,
                        None => continue,
                    };
                    let fstype = parts.next().unwrap_or("");
                    if skip_fstypes.contains(fstype) {
                        continue;
                    }
                    if !seen_mounts.insert(mountpoint.to_string()) {
                        continue;
                    }
                    let t = lua_ctx.create_table()?;
                    t.set("path", mountpoint.to_string())?;
                    t.set("label", mountpoint.to_string())?;
                    t.set("fstype", fstype.to_string())?;
                    out.push(mlua::Value::Table(t));
                }
            }
            Ok(out)
        })?)?;
    }

    #[cfg(target_os = "macos")]
    {
        // TODO(M3.5): macOS partition discovery via `getmntinfo` or
        // shelling out to `diskutil list -plist`. For M3 we emit
        // an empty list so the ChDrive UI can degrade gracefully.
        fs.set("partitions", lua.create_function(|_lua_ctx, ()| Ok(Vec::<mlua::Value>::new()))?)?;
    }

    #[cfg(target_os = "windows")]
    {
        // Enumerate drive letters A–Z via std::fs::metadata; emit
        // one entry per existing drive with fstype=nil.
        fs.set("partitions", lua.create_function(move |lua_ctx, ()| {
            let mut out: Vec<mlua::Value> = Vec::new();
            for letter in b'A'..=b'Z' {
                let p = format!("{}:\\", letter as char);
                if std::fs::metadata(&p).is_ok() {
                    let t = lua_ctx.create_table()?;
                    t.set("path", p.clone())?;
                    t.set("label", p)?;
                    t.set("fstype", mlua::Value::Nil)?;
                    out.push(mlua::Value::Table(t));
                }
            }
            Ok(out)
        })?)?;
    }

    // calc_size(url) — synchronous helper. Walks a directory (or
    // single file) and sums `len()` across all regular files. Bounded
    // at 100k entries for M3 (warns if hit).
    fs.set(
        "calc_size",
        lua.create_function(move |lua_ctx, url: String| {
            let path = validate_path(lua_ctx, &url)?;
            const MAX_ENTRIES: usize = 100_000;
            let mut total: u64 = 0;
            let mut count: usize = 0;
            let mut stack: Vec<std::path::PathBuf> = vec![path];
            while let Some(p) = stack.pop() {
                if count >= MAX_ENTRIES {
                    log::warn!("fs.calc_size: hit {MAX_ENTRIES}-entry cap, result is a lower bound");
                    break;
                }
                count += 1;
                let m = match std::fs::symlink_metadata(&p) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if m.is_dir() {
                    if let Ok(rd) = std::fs::read_dir(&p) {
                        for entry in rd.flatten() {
                            stack.push(entry.path());
                        }
                    }
                } else if m.is_file() {
                    total = total.saturating_add(m.len());
                }
            }
            Ok(total)
        })?,
    )?;

    Ok(fs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Run `chunk` with a fresh `fs` table assigned to the global
    /// `fs`. Keeps the Lua state borrowed only for the duration of
    /// the closure (avoids the "cannot move out of `lua`" error
    /// when the fs Table borrows from the Lua handle).
    fn with_fs<F, R>(f: F) -> R
    where
        F: for<'lua> FnOnce(&'lua mlua::Lua) -> R,
    {
        let lua = mlua::Lua::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let fs = bind(&lua, true, tx).expect("fs.bind should succeed");
        lua.globals().set("fs", fs).expect("set fs");
        f(&lua)
    }

    #[test]
    fn test_fs_unique_file() {
        let tmp = TempDir::new().expect("tempdir");
        let base_str = tmp.path().join("uniq").to_string_lossy().to_string();
        with_fs(|lua| {
            let code = format!(
                "return fs.unique('file', '{base}'):path()",
                base = base_str
            );
            let path: String = lua.load(&code).eval().expect("unique");
            assert!(path.starts_with(&base_str));
            assert!(std::path::Path::new(&path).exists());
        });
    }

    #[test]
    fn test_fs_unique_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let base_str = tmp.path().join("uniqdir").to_string_lossy().to_string();
        with_fs(|lua| {
            let code = format!(
                "return fs.unique('dir', '{base}'):path()",
                base = base_str
            );
            let path: String = lua.load(&code).eval().expect("unique");
            let p = std::path::Path::new(&path);
            assert!(p.is_dir(), "fs.unique dir should create a directory at {path}");
        });
    }

    #[test]
    fn test_fs_unique_none() {
        let tmp = TempDir::new().expect("tempdir");
        let base_str = tmp.path().join("uniqnone").to_string_lossy().to_string();
        with_fs(|lua| {
            let code = format!(
                "return fs.unique('none', '{base}'):path()",
                base = base_str
            );
            let path: String = lua.load(&code).eval().expect("unique");
            // "none" does NOT create the file, so it should not exist.
            assert!(!std::path::Path::new(&path).exists());
        });
    }

    #[test]
    fn test_fs_expand_url_string() {
        with_fs(|lua| {
            let code = "return fs.expand_url('/tmp/some/path'):path()";
            let path: String = lua.load(code).eval().expect("expand_url");
            assert_eq!(path, "/tmp/some/path");
        });
    }

    #[test]
    fn test_fs_expand_url_userdata() {
        with_fs(|lua| {
            let original = crate::plugin::types::Url::parse("/etc/hosts");
            lua.globals()
                .set("orig", lua.create_userdata(original.clone()).unwrap())
                .expect("set orig");
            let code = "return fs.expand_url(orig):path()";
            let path: String = lua.load(code).eval().expect("expand_url");
            assert_eq!(path, original.path.to_string_lossy());
        });
    }

    #[test]
    fn test_fs_calc_size_single_file() {
        let tmp = TempDir::new().expect("tempdir");
        let f = tmp.path().join("hello.txt");
        std::fs::write(&f, b"hello, world!").expect("write");
        let f_str = f.to_string_lossy().to_string();
        with_fs(|lua| {
            let code = format!("return fs.calc_size('{f}')", f = f_str);
            let total: u64 = lua.load(&code).eval().expect("calc_size");
            assert_eq!(total, 13);
        });
    }

    #[test]
    fn test_fs_calc_size_dir() {
        let tmp = TempDir::new().expect("tempdir");
        std::fs::write(tmp.path().join("a.txt"), b"aaaa").expect("write a");
        std::fs::write(tmp.path().join("b.txt"), b"bbbbbb").expect("write b");
        let dir_str = tmp.path().to_string_lossy().to_string();
        with_fs(|lua| {
            let code = format!("return fs.calc_size('{d}')", d = dir_str);
            let total: u64 = lua.load(&code).eval().expect("calc_size");
            assert_eq!(total, 10);
        });
    }
}
