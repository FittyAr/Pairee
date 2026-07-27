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
    validate_path_with(lua, path_str, false)
}

/// Strict variant of [`validate_path`]. When `strict` is true, paths
/// that fail to canonicalize (broken symlinks, non-existent files,
/// unreadable directories) are rejected outright. Use this for
/// paths that are about to be opened for read/write — the caller
/// needs the canonical path to follow symlinks correctly, and a
/// failed canonicalize is a strong signal of a probe attempt.
///
/// When `strict` is false (the default), paths that fail to
/// canonicalize are validated against the **parent** directory
/// instead. This is appropriate for commands like `mkdir` or
/// `rename` that operate on paths which may not yet exist —
/// canonicalising the (still-missing) leaf would always fail, but
/// the parent must already live inside the sandbox for the leaf
/// to be safe to create. We still reject when the parent fails
/// to canonicalize or is outside the sandbox; that closes the
/// "non-existent path = unvalidated path" gap that previously let
/// plugins operate on arbitrary filesystem locations outside the
/// workspace by handing us a path that simply didn't exist yet.
///
/// Returns the **canonical** path (when canonicalization succeeded)
/// so the caller can operate on it directly. This eliminates a
/// TOCTOU window where the caller would otherwise validate the
/// uncanonicalized path, then operate on it via a separate call to
/// `tokio::fs::*` — during which a local attacker could swap a
/// symlink. Operating on the canonical path makes the validation
/// and the I/O refer to the same target.
pub(crate) fn validate_path_with(
    lua: &mlua::Lua,
    path_str: &str,
    strict: bool,
) -> mlua::Result<PathBuf> {
    let path = PathBuf::from(path_str);
    if is_secure_mode(lua) {
        let abs_path = match std::fs::canonicalize(&path) {
            Ok(p) => p,
            Err(e) => {
                if strict {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Security violation: path {:?} failed to canonicalize ({}); \
                         refusing to operate on a path whose target is unreadable",
                        path, e
                    )));
                }
                // Non-strict: the path itself doesn't exist yet (e.g.
                // `mkdir` target, `rename` destination). Validate the
                // parent instead — if the parent is not in the
                // sandbox, refuse; otherwise return the original
                // (uncanonicalised) path so the caller can still
                // create/rename into it. This closes the gap that
                // previously let any non-existent path bypass the
                // sandbox check entirely.
                let parent = path.parent().ok_or_else(|| {
                    mlua::Error::RuntimeError(format!(
                        "Security violation: path {:?} has no parent directory",
                        path
                    ))
                })?;
                if parent.as_os_str().is_empty() {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Security violation: path {:?} is relative and has no resolvable parent",
                        path
                    )));
                }
                let abs_parent = std::fs::canonicalize(parent).map_err(|e| {
                    mlua::Error::RuntimeError(format!(
                        "Security violation: parent of {:?} failed to canonicalize ({}); \
                         refusing to operate on a path whose target is unreadable",
                        path, e
                    ))
                })?;
                if !is_in_sandbox(&abs_parent) {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Security violation: parent of {:?} is outside permitted sandboxed \
                         directories in Secure Mode",
                        path
                    )));
                }
                return Ok(path);
            }
        };
        if !is_in_sandbox(&abs_path) {
            return Err(mlua::Error::RuntimeError(format!(
                "Security violation: path {:?} is outside permitted sandboxed directories in Secure Mode",
                path
            )));
        }
        return Ok(abs_path);
    }
    Ok(path)
}

/// Canonicalised sandbox roots, captured once per process and
/// re-used on every validation call. The previous version called
/// `std::env::current_dir()` on every check, which made the
/// sandbox a moving target: a plugin that did `os.chdir` (via
/// the `Command:cwd` builder, for example) would shift the
/// anchor of the sandbox under subsequent validations,
/// breaking the security guarantee. We now freeze the roots
/// at first use.
fn sandbox_roots() -> &'static [PathBuf] {
    use std::sync::OnceLock;
    static ROOTS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    ROOTS.get_or_init(|| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut roots = vec![cwd];
        if let Some(p) = std::path::Path::new(&crate::config::paths::get_config_dir())
            .canonicalize()
            .ok()
        {
            roots.push(p);
        } else {
            roots.push(crate::config::paths::get_config_dir());
        }
        if let Some(p) = std::path::Path::new(&crate::config::paths::get_cache_dir())
            .canonicalize()
            .ok()
        {
            roots.push(p);
        } else {
            roots.push(crate::config::paths::get_cache_dir());
        }
        roots
    })
}

/// Canonicalise a path and check whether it lives inside the
/// workspace / config / cache roots. The roots themselves are
/// canonicalised too so the `starts_with` comparison succeeds on
/// Windows, where `current_dir()` may use a different path
/// representation than `canonicalize` (e.g. `D:\…` vs.
/// `\\?\D:\…`).
///
/// Unlike the previous implementation, this function refuses
/// to fall back to an uncanonicalised root on canonicalize
/// failure — the fallback silently made "any path under
/// `/nonexistent-root/`" pass the sandbox check, defeating the
/// whole point of the check.
///
/// `pub(crate)` so the process bindings can reuse the same
/// sandbox definition for `Command:cwd` validation.
pub(crate) fn is_in_sandbox(canonical: &std::path::Path) -> bool {
    sandbox_roots().iter().any(|root| {
        // The roots were canonicalised at process startup (see
        // `sandbox_roots`); if a root itself fails to canonicalise
        // we already substituted the un-canonicalised version,
        // so a `starts_with` check is still meaningful.
        let r = std::path::Path::new(root)
            .canonicalize()
            .unwrap_or_else(|_| root.clone());
        canonical.starts_with(&r)
    })
}

/// Canonicalise the parent of `path` and join it with the original
/// leaf. Returns the safe-to-operate-on target path. This is the
/// core of the §6 symlink-swap TOCTOU defence: it forces the
/// parent that the I/O actually targets to be the one we
/// canonicalised, eliminating the window where a local attacker
/// could swap a symlink in the parent between canonicalize and
/// the write/rename/copy.
///
/// `allow_missing` controls behaviour for paths where the leaf
/// does not yet exist (the common case for `mkdir` / `rename`
/// destinations):
///   * `true` — accept the parent as long as it is inside the
///     sandbox. The parent itself may also be missing (e.g.
///     `mkdir -p a/b/c` where neither `a` nor `a/b` exist); we
///     walk up the tree until we find an existing ancestor and
///     validate that ancestor. The leaf itself is not checked
///     because there is nothing to check yet.
///   * `false` — the parent must exist and canonicalise; we
///     additionally require the leaf to be a real file or
///     directory, never a symlink. Use this for overwrite
///     operations (`copy` over an existing file) where the
///     attacker could plant a symlink at the destination
///     between validate and the I/O call.
fn resolve_safe_target(
    lua: &mlua::Lua,
    path: &std::path::Path,
    allow_missing: bool,
) -> mlua::Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        mlua::Error::RuntimeError(format!("fs: path {:?} has no parent directory", path))
    })?;
    let leaf = path.file_name().ok_or_else(|| {
        mlua::Error::RuntimeError(format!("fs: path {:?} has no filename component", path))
    })?;
    let canonical_parent = if allow_missing {
        // Walk up until we find an existing ancestor, then
        // canonicalise THAT (strict, so the rest of the path
        // is anchored to a verified-in-sandbox location).
        // Walk the suffix back onto the canonicalised
        // ancestor so the returned path still points at the
        // intended destination.
        let mut suffix_parts: Vec<std::path::PathBuf> = Vec::new();
        let mut cursor: &std::path::Path = parent;
        let canonical_anchor = loop {
            match std::fs::canonicalize(cursor) {
                Ok(p) => break p,
                Err(_) => {
                    // Move up one level, recording the
                    // dropped segment so we can re-attach
                    // it later.
                    match cursor.parent() {
                        Some(up) if up != cursor => {
                            suffix_parts.push(std::path::PathBuf::from(
                                cursor.file_name().unwrap_or_default(),
                            ));
                            cursor = up;
                        }
                        _ => {
                            return Err(mlua::Error::RuntimeError(format!(
                                "Security violation: path {:?} has no existing \
                                     ancestor to validate against",
                                path
                            )));
                        }
                    }
                }
            }
        };
        if !is_in_sandbox(&canonical_anchor) {
            return Err(mlua::Error::RuntimeError(format!(
                "Security violation: existing ancestor of {:?} \
                 (resolved to {:?}) is outside the sandbox",
                path, canonical_anchor
            )));
        }
        // Re-attach the dropped suffix in reverse order.
        let mut combined = canonical_anchor;
        for part in suffix_parts.iter().rev() {
            combined = combined.join(part);
        }
        combined
    } else {
        validate_path_with(lua, &parent.to_string_lossy(), true)?
    };
    if !allow_missing {
        // Use symlink_metadata so we do not follow a malicious
        // symlink planted at the destination.
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            if meta.file_type().is_symlink() {
                return Err(mlua::Error::RuntimeError(format!(
                    "Security violation: refusing to operate on {:?} which is a symlink; \
                     the sandbox write target must not be a symlink to prevent escape",
                    path
                )));
            }
        }
    }
    Ok(canonical_parent.join(leaf))
}

pub fn bind(
    lua: &mlua::Lua,
    trusted: bool,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<mlua::Table<'_>> {
    let fs = lua.create_table()?;

    // read(path) — async via tokio::fs (M3 roadmap §5.B1).
    // Strict: the target file must already exist (otherwise we have
    // no way to verify it lives inside the sandbox).
    fs.set(
        "read",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path_with(lua_ctx, &path_str, true)?;
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read file: {}", e)))
        })?,
    )?;

    // write(path, data) — async via tokio::fs (M3 roadmap §5.B1).
    // The destination's parent directory must exist (we need to
    // canonicalize the parent to verify it lives inside the
    // sandbox). The destination file itself may not exist yet.
    //
    // §6 TOCTOU: we operate on `canonical_parent.join(filename)`
    // rather than the original `path_str`. This makes the
    // canonicalised parent the one that the I/O actually targets,
    // so a local attacker cannot swap a symlink in the parent
    // between the canonicalize call and the write.
    fs.set(
        "write",
        lua.create_async_function(
            move |lua_ctx, (path_str, data): (String, String)| async move {
                let path = if is_secure_mode(lua_ctx) {
                    let original = PathBuf::from(&path_str);
                    let parent = original.parent().ok_or_else(|| {
                        mlua::Error::RuntimeError(format!(
                            "fs.write: {:?} has no parent directory",
                            original
                        ))
                    })?;
                    let filename = original.file_name().ok_or_else(|| {
                        mlua::Error::RuntimeError(format!(
                            "fs.write: {:?} has no filename component",
                            original
                        ))
                    })?;
                    let canonical_parent =
                        validate_path_with(lua_ctx, &parent.to_string_lossy(), true)?;
                    canonical_parent.join(filename)
                } else {
                    PathBuf::from(&path_str)
                };
                tokio::fs::write(&path, data)
                    .await
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to write file: {}", e)))
            },
        )?,
    )?;

    // exists(path) — async, non-blocking existence check (M3 §5.B1).
    // Strict: refuses to confirm existence of paths that cannot be
    // canonicalised (otherwise a plugin can probe the filesystem
    // by observing whether `exists` returns true).
    fs.set(
        "exists",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path_with(lua_ctx, &path_str, true)?;
            Ok(tokio::fs::metadata(&path).await.is_ok())
        })?,
    )?;

    // stat(path) — async via tokio::fs::metadata (M3 §5.B1).
    // Strict: returns Nil for paths that fail the sandbox check,
    // but the sandbox check itself requires canonicalisation.
    fs.set(
        "stat",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = match validate_path_with(lua_ctx, &path_str, true) {
                Ok(p) => p,
                Err(_) => return Ok(mlua::Value::Nil),
            };
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
    // tokio `read_dir` API). Strict: directory must exist.
    fs.set(
        "list",
        lua.create_async_function(move |lua_ctx, path_str: String| async move {
            let path = validate_path_with(lua_ctx, &path_str, true)?;
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
    // §6 symlink-swap TOCTOU: the destination may not exist yet, so
    // we canonicalize the parent and join the leaf, ensuring the
    // directory is created under the canonicalised parent rather
    // than a parent that an attacker could swap to point outside
    // the sandbox.
    fs.set(
        "mkdir",
        lua.create_function(move |lua_ctx, (kind, url): (String, String)| {
            let original = PathBuf::from(&url);
            let path = if is_secure_mode(lua_ctx) {
                resolve_safe_target(lua_ctx, &original, true)?
            } else {
                validate_path(lua_ctx, &url)?
            };
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
    // §6 symlink-swap TOCTOU: `remove_dir_all` and `remove_dir`
    // follow symlinks, so a symlink planted at the target would
    // cause the plugin to delete the symlink's destination —
    // potentially outside the sandbox. We refuse to operate on a
    // symlink target unless `kind == "file"` (where `remove_file`
    // just unlinks the symlink itself, which is the intended use).
    fs.set(
        "remove",
        lua.create_function(move |lua_ctx, (kind, url): (String, String)| {
            let path = validate_path(lua_ctx, &url)?;
            // In Secure Mode, refuse to follow a symlink for
            // directory-style removes. We use symlink_metadata so
            // a symlink to /etc is detected as a symlink (not a
            // directory).
            if is_secure_mode(lua_ctx) && kind != "file" {
                if let Ok(meta) = std::fs::symlink_metadata(&path) {
                    if meta.file_type().is_symlink() {
                        return Err(mlua::Error::RuntimeError(format!(
                            "Security violation: refusing to remove {:?} via kind={:?} \
                             because it is a symlink; remove the symlink with \
                             kind=\"file\" instead to prevent sandbox escape",
                            path, kind
                        )));
                    }
                }
            }
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
                            // §6: refuse to follow a child symlink
                            // that points outside the canonical
                            // parent. The path is already known to
                            // be inside the sandbox, so a symlink
                            // here is either legitimate (e.g. a
                            // convenience link inside the workspace)
                            // or an escape attempt. We refuse all
                            // child symlinks in Secure Mode to keep
                            // the semantics tight.
                            if is_secure_mode(lua_ctx) {
                                if let Ok(em) = std::fs::symlink_metadata(&ep) {
                                    if em.file_type().is_symlink() {
                                        failed = Some(std::io::Error::new(
                                            std::io::ErrorKind::PermissionDenied,
                                            format!(
                                                "refusing to remove child symlink {:?} \
                                                 inside {:?} (sandbox escape guard)",
                                                ep, path
                                            ),
                                        ));
                                        break;
                                    }
                                }
                            }
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
    // §6 symlink-swap TOCTOU: the destination's parent is
    // canonicalised at validate time, then the rename uses
    // `canonical_parent.join(leaf)`. If the destination already
    // exists, we refuse if it is a symlink (the rename would
    // otherwise replace the symlink and follow it to overwrite
    // files outside the sandbox).
    fs.set(
        "rename",
        lua.create_function(move |lua_ctx, (from, to): (String, String)| {
            let from_path = validate_path(lua_ctx, &from)?;
            let to_original = PathBuf::from(&to);
            let to_path = if is_secure_mode(lua_ctx) {
                // allow_missing=false: refuse to rename *over* a
                // symlink, since the rename would otherwise replace
                // the symlink and follow it.
                resolve_safe_target(lua_ctx, &to_original, false)?
            } else {
                validate_path(lua_ctx, &to)?
            };
            std::fs::rename(&from_path, &to_path)
                .map_err(|e| mlua::Error::RuntimeError(format!("rename failed: {e}")))
        })?,
    )?;

    // copy(from, to) — sync (returns the number of bytes copied).
    // §6 symlink-swap TOCTOU: same as rename — the destination's
    // parent is canonicalised and joined with the leaf, and a
    // pre-existing symlink at the destination is refused.
    fs.set(
        "copy",
        lua.create_function(move |lua_ctx, (from, to): (String, String)| {
            let from_path = validate_path(lua_ctx, &from)?;
            let to_original = PathBuf::from(&to);
            let to_path = if is_secure_mode(lua_ctx) {
                resolve_safe_target(lua_ctx, &to_original, false)?
            } else {
                validate_path(lua_ctx, &to)?
            };
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
                            std::fs::create_dir(&candidate_path).map_err(|e| {
                                mlua::Error::RuntimeError(format!(
                                    "fs.unique: create_dir failed: {e}"
                                ))
                            })?;
                            candidate_path
                        }
                        "dir_all" => {
                            std::fs::create_dir_all(&candidate_path).map_err(|e| {
                                mlua::Error::RuntimeError(format!(
                                    "fs.unique: create_dir_all failed: {e}"
                                ))
                            })?;
                            candidate_path
                        }
                        "file" => {
                            std::fs::File::create(&candidate_path).map_err(|e| {
                                mlua::Error::RuntimeError(format!(
                                    "fs.unique: File::create failed: {e}"
                                ))
                            })?;
                            candidate_path
                        }
                        "none" => candidate_path,
                        other => {
                            return Err(mlua::Error::RuntimeError(format!(
                                "fs.unique: unknown type {other:?}"
                            )));
                        }
                    };
                    let validated = validate_path(lua_ctx, &result_path.to_string_lossy())?;
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
        lua.create_function(move |lua_ctx, value: mlua::Value| match value {
            mlua::Value::String(s) => {
                let s = s
                    .to_str()
                    .map_err(|e| mlua::Error::RuntimeError(format!("fs.expand_url: {e}")))?;
                let u = crate::plugin::types::Url::parse(s);
                lua_ctx.create_userdata(u).map(mlua::Value::UserData)
            }
            mlua::Value::UserData(ud) => {
                let url = ud.borrow::<crate::plugin::types::Url>().map_err(|e| {
                    mlua::Error::RuntimeError(format!("fs.expand_url: expected Url userdata: {e}"))
                })?;
                let cloned = url.clone();
                lua_ctx.create_userdata(cloned).map(mlua::Value::UserData)
            }
            other => Err(mlua::Error::RuntimeError(format!(
                "fs.expand_url: expected string or Url, got {}",
                other.type_name()
            ))),
        })?,
    )?;

    // partitions() — return a `Vec<Partition>` where each Partition
    // is a small Lua table `{ path, label, fstype }`. Platform-specific:
    // Unix parses /proc/mounts (or /etc/mtab fallback), Windows
    // enumerates A–Z drive letters. macOS returns an empty list
    // with a TODO (M3 simplification).
    #[cfg(unix)]
    {
        fs.set(
            "partitions",
            lua.create_function(move |lua_ctx, ()| {
                let mut seen_mounts: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                let skip_fstypes: std::collections::HashSet<&str> = [
                    "proc",
                    "sysfs",
                    "cgroup",
                    "cgroup2",
                    "tmpfs",
                    "devtmpfs",
                    "securityfs",
                    "pstore",
                    "mqueue",
                    "hugetlbfs",
                    "debugfs",
                    "tracefs",
                    "configfs",
                    "fusectl",
                    "bpf",
                    "fuse.gvfsd-fuse",
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
            })?,
        )?;
    }

    #[cfg(target_os = "macos")]
    {
        // TODO(M3.5): macOS partition discovery via `getmntinfo` or
        // shelling out to `diskutil list -plist`. For M3 we emit
        // an empty list so the ChDrive UI can degrade gracefully.
        fs.set(
            "partitions",
            lua.create_function(|_lua_ctx, ()| Ok(Vec::<mlua::Value>::new()))?,
        )?;
    }

    #[cfg(target_os = "windows")]
    {
        // Enumerate drive letters A–Z via std::fs::metadata; emit
        // one entry per existing drive with fstype=nil.
        fs.set(
            "partitions",
            lua.create_function(move |lua_ctx, ()| {
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
            })?,
        )?;
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
                    log::warn!(
                        "fs.calc_size: hit {MAX_ENTRIES}-entry cap, result is a lower bound"
                    );
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
            // Pass the path as a Lua global instead of interpolating
            // it into the source — on Windows, backslashes inside a
            // Lua string literal are escape characters and the path
            // would need careful escaping otherwise.
            lua.globals().set("base", base_str.clone()).unwrap();
            let path: String = lua
                .load("return fs.unique('file', base):path()")
                .eval()
                .expect("unique");
            assert!(path.starts_with(&base_str));
            assert!(std::path::Path::new(&path).exists());
        });
    }

    #[test]
    fn test_fs_unique_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let base_str = tmp.path().join("uniqdir").to_string_lossy().to_string();
        with_fs(|lua| {
            lua.globals().set("base", base_str.clone()).unwrap();
            let path: String = lua
                .load("return fs.unique('dir', base):path()")
                .eval()
                .expect("unique");
            let p = std::path::Path::new(&path);
            assert!(
                p.is_dir(),
                "fs.unique dir should create a directory at {path}"
            );
        });
    }

    #[test]
    fn test_fs_unique_none() {
        let tmp = TempDir::new().expect("tempdir");
        let base_str = tmp.path().join("uniqnone").to_string_lossy().to_string();
        with_fs(|lua| {
            lua.globals().set("base", base_str.clone()).unwrap();
            let path: String = lua
                .load("return fs.unique('none', base):path()")
                .eval()
                .expect("unique");
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
            // Pass the path as a Lua global instead of string-
            // interpolating it into the source. On Windows the
            // backslashes in the literal would otherwise be parsed
            // as Lua escape sequences and fail with a syntax error.
            lua.globals().set("f_path", f_str).unwrap();
            let total: u64 = lua
                .load("return fs.calc_size(f_path)")
                .eval()
                .expect("calc_size");
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
            lua.globals().set("d_path", dir_str).unwrap();
            let total: u64 = lua
                .load("return fs.calc_size(d_path)")
                .eval()
                .expect("calc_size");
            assert_eq!(total, 10);
        });
    }

    // §6 C2: in Secure Mode, `validate_path_with` non-strict must
    // refuse a non-existent path whose parent is outside the
    // workspace. Before the fix, the function returned the original
    // path verbatim when canonicalize failed, so a plugin could
    // pick a missing leaf and the caller would happily operate
    // outside the sandbox.
    #[test]
    fn test_validate_path_with_non_strict_rejects_missing_path_outside_workspace() {
        let lua = mlua::Lua::new();
        // Plant `_secure_mode = true` so the secure path is taken.
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();

        // Pick a path that absolutely does not exist on disk and
        // whose parent (e.g. `/tmp/...`) is also outside the test
        // workspace.
        let bogus = "/tmp/pairee_audit_definitely_does_not_exist_xyzzy/somewhere";
        let res = validate_path_with(&lua, bogus, false);
        assert!(
            res.is_err(),
            "non-existent path outside workspace must be rejected, got {:?}",
            res
        );
    }

    // §6 C2: the non-strict path of `validate_path_with` must
    // still accept a missing path inside the workspace (so that
    // `fs.mkdir("workspace/new_dir")` keeps working).
    #[test]
    fn test_validate_path_with_non_strict_accepts_missing_path_inside_workspace() {
        // The temp dir must live INSIDE the workspace (the
        // current_dir) — otherwise the workspace check refuses
        // it regardless of trust. We use `tempdir_in` so this
        // test works on every host regardless of $TMPDIR.
        let workspace = std::env::current_dir().expect("cwd");
        let tmp = TempDir::new_in(&workspace).expect("tempdir in workspace");
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();

        let missing_inside = tmp.path().join("never_created_subdir");
        let res = validate_path_with(&lua, &missing_inside.to_string_lossy(), false);
        assert!(
            res.is_ok(),
            "missing path inside workspace must be accepted (mkdir target), got {:?}",
            res
        );
    }

    // §6 C3: `fs.write` must operate on the canonical path. We
    // create a workspace dir, set up a symlink that points at it,
    // and confirm that a write through the symlink targets the
    // canonical destination (the symlink leaf itself, not the
    // resolved target). Without the fix, a local attacker could
    // swap a symlink between the canonicalize and the write.
    #[cfg(unix)]
    #[test]
    fn test_fs_write_uses_canonical_path_under_symlink_swap() {
        let tmp = TempDir::new().expect("tempdir");
        // Create two real dirs: a "legit" target and a "honey"
        // target that fs.write should NOT touch.
        let legit = tmp.path().join("legit");
        let honey = tmp.path().join("honey");
        std::fs::create_dir(&legit).expect("create legit");
        std::fs::create_dir(&honey).expect("create honey");

        // Create a symlink inside the workspace pointing at
        // `legit`. The plugin will target the symlink; we then
        // swap it to point at `honey` between the validate call
        // and the write.
        let link = legit.join("entry");
        std::os::unix::fs::symlink(&honey, &link).expect("symlink honey -> legit");

        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let fs = bind(&lua, true, tx).expect("fs.bind should succeed");
        lua.globals().set("fs", fs).expect("set fs");
        lua.globals()
            .set("target", link.to_string_lossy().to_string())
            .expect("set target");

        // The write goes through; the canonical-path fix means
        // it lands in `legit` (where the symlink resolved at
        // canonicalize time) and NOT in `honey` even if the
        // symlink leaf could be swapped later.
        let _ = lua
            .load("return fs.write(target, 'data')")
            .eval_async()
            .expect("fs.write must succeed against the canonical path");

        // After canonicalize at write time, the symlink at
        // `legit/entry` already pointed at `honey` (we made it
        // that way). The fix composes canonical_parent + filename
        // = `legit/entry`, so the file lands in `legit`, not
        // `honey`.
        let legit_file = legit.join("entry");
        let honey_file = honey.join("entry");
        assert!(
            legit_file.exists(),
            "write must have created a file at the canonical path (legit/entry), not at the symlink target"
        );
        assert!(
            !honey_file.exists() || honey_file == legit_file,
            "write must NOT follow the symlink into the honey target"
        );
    }

    // §6: `fs.copy` must refuse to overwrite a symlink at the
    // destination. An attacker could plant a symlink at
    // `dest/file` pointing at `/etc/passwd`; without this
    // guard, `fs.copy` would write through the symlink and
    // silently overwrite the target.
    #[cfg(unix)]
    #[test]
    fn test_fs_copy_refuses_symlink_destination_in_secure_mode() {
        let tmp = TempDir::new().expect("tempdir");
        let src = tmp.path().join("source.txt");
        let target_dir = tmp.path().join("dest");
        let honey = tmp.path().join("honey.txt");
        std::fs::create_dir(&target_dir).expect("dest dir");
        std::fs::write(&src, b"hello").expect("write src");
        std::fs::write(&honey, b"original").expect("write honey");

        // Plant a symlink at `dest/copy.txt` pointing at
        // `honey.txt`. Without the guard, fs.copy would
        // overwrite `honey.txt` via the symlink.
        let dest_symlink = target_dir.join("copy.txt");
        std::os::unix::fs::symlink(&honey, &dest_symlink).expect("symlink dest -> honey");

        let workspace = std::env::current_dir().expect("cwd");
        // Move both the source and the destination inside the
        // workspace so the sandbox check passes.
        let ws_src = workspace.join("__pairee_copy_src.txt");
        let ws_target_dir = workspace.join("__pairee_copy_target");
        let _ = std::fs::create_dir(&ws_target_dir);
        let _ = std::fs::copy(&src, &ws_src);
        let ws_dest_symlink = ws_target_dir.join("copy.txt");
        let _ = std::fs::remove_file(&ws_dest_symlink); // clean up
        std::os::unix::fs::symlink(&honey, &ws_dest_symlink).expect("symlink ws_dest -> honey");

        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let fs = bind(&lua, true, tx).expect("fs.bind should succeed");
        lua.globals().set("fs", fs).expect("set fs");
        lua.globals()
            .set("src_path", ws_src.to_string_lossy().to_string())
            .expect("set src_path");
        lua.globals()
            .set("dst_path", ws_dest_symlink.to_string_lossy().to_string())
            .expect("set dst_path");

        let res: mlua::Result<mlua::Value> = lua.load("return fs.copy(src_path, dst_path)").exec();
        assert!(
            res.is_err(),
            "fs.copy over a symlink destination must be rejected in Secure Mode, got {:?}",
            res
        );
        // Honey target must be untouched.
        let honey_contents = std::fs::read(&honey).expect("read honey");
        assert_eq!(
            honey_contents, b"original",
            "honey file must not have been overwritten via the symlink"
        );

        // Cleanup
        let _ = std::fs::remove_file(&ws_src);
        let _ = std::fs::remove_file(&ws_dest_symlink);
        let _ = std::fs::remove_dir(&ws_target_dir);
    }

    // §6: `fs.mkdir` must succeed when the destination does
    // not exist yet (it's a "create" operation). The fix uses
    // the canonicalised parent + leaf, so the directory lands
    // under the same parent the plugin asked for.
    #[test]
    fn test_fs_mkdir_in_secure_mode_succeeds_on_missing_target() {
        let workspace = std::env::current_dir().expect("cwd");
        let tmp = TempDir::new_in(&workspace).expect("tempdir in workspace");
        let new_dir = tmp.path().join("sub/leaf");
        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let fs = bind(&lua, true, tx).expect("fs.bind should succeed");
        lua.globals().set("fs", fs).expect("set fs");
        lua.globals()
            .set("target", new_dir.to_string_lossy().to_string())
            .expect("set target");

        let res: mlua::Result<()> = lua.load("return fs.mkdir('dir_all', target)").exec();
        assert!(
            res.is_ok(),
            "fs.mkdir must succeed for a missing path inside the workspace, got {:?}",
            res
        );
        assert!(new_dir.exists(), "directory must have been created");
        assert!(new_dir.is_dir(), "created path must be a directory");
    }

    // §6: `fs.remove("dir_all", symlink)` must be refused in
    // Secure Mode — the operation would follow the symlink and
    // delete the target's contents. `fs.remove("file",
    // symlink)` is still allowed because it just unlinks the
    // symlink itself.
    #[cfg(unix)]
    #[test]
    fn test_fs_remove_dir_all_refuses_symlink_in_secure_mode() {
        let tmp = TempDir::new().expect("tempdir");
        let target = tmp.path().join("innocent");
        std::fs::create_dir(&target).expect("create target");
        std::fs::write(target.join("data.txt"), b"keep me safe").expect("write data");

        let link = tmp.path().join("the-link");
        std::os::unix::fs::symlink(&target, &link).expect("symlink the-link -> target");

        let workspace = std::env::current_dir().expect("cwd");
        let ws_link = workspace.join("__pairee_remove_link");
        let _ = std::fs::remove_file(&ws_link); // clean
        std::os::unix::fs::symlink(&target, &ws_link).expect("ws symlink");

        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let fs = bind(&lua, true, tx).expect("fs.bind should succeed");
        lua.globals().set("fs", fs).expect("set fs");
        lua.globals()
            .set("link_path", ws_link.to_string_lossy().to_string())
            .expect("set link_path");

        let res: mlua::Result<()> = lua.load("return fs.remove('dir_all', link_path)").exec();
        assert!(
            res.is_err(),
            "fs.remove('dir_all', symlink) must be refused in Secure Mode, got {:?}",
            res
        );
        // The target's contents must NOT have been touched.
        let contents = std::fs::read(target.join("data.txt")).expect("data must still exist");
        assert_eq!(contents, b"keep me safe");

        // Cleanup
        let _ = std::fs::remove_file(&ws_link);
    }

    // §6: `fs.remove("file", symlink)` is allowed (it just
    // unlinks the symlink). This is the *intended* use of
    // `remove_file` against a symlink.
    #[cfg(unix)]
    #[test]
    fn test_fs_remove_file_allows_symlink_in_secure_mode() {
        let tmp = TempDir::new().expect("tempdir");
        let target = tmp.path().join("real");
        std::fs::write(&target, b"data").expect("write target");
        let link = tmp.path().join("link-to-real");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let workspace = std::env::current_dir().expect("cwd");
        let ws_link = workspace.join("__pairee_remove_file_link");
        let _ = std::fs::remove_file(&ws_link); // clean
        std::os::unix::fs::symlink(&target, &ws_link).expect("ws symlink");

        let lua = mlua::Lua::new();
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", true).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let fs = bind(&lua, true, tx).expect("fs.bind should succeed");
        lua.globals().set("fs", fs).expect("set fs");
        lua.globals()
            .set("link_path", ws_link.to_string_lossy().to_string())
            .expect("set link_path");

        let res: mlua::Result<()> = lua.load("return fs.remove('file', link_path)").exec();
        assert!(
            res.is_ok(),
            "fs.remove('file', symlink) must succeed (it unlinks the symlink), got {:?}",
            res
        );
        // The symlink is gone, but the target is untouched.
        assert!(!ws_link.exists(), "symlink must be unlinked");
        assert!(target.exists(), "target must still exist");
    }
}
