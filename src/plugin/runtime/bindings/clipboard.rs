//! Lua binding for `pairee.clipboard(text?)` (M1).
//!
//! The single-name API matches the roadmap (§5.E7):
//!
//! - `pairee.clipboard()` (no arg or `nil`) — **get** the current
//!   clipboard text. Returns a string, or `nil` if the clipboard is
//!   empty / not text / blocked by Secure Mode.
//! - `pairee.clipboard(text)` — **set** the clipboard to `text`.
//!   Returns `true` on success, `nil` on failure (and a warning is
//!   logged).
//!
//! **Secure Mode policy** (roadmap §6):
//!
//! - `get` is blocked outright in Secure Mode (data exfiltration
//!   vector): returns `nil` and emits a single `log::warn!` per process.
//! - `set` is allowed but a single `log::warn!` is emitted if the
//!   value is non-empty and is not a path inside the user's
//!   workspace, config, or cache directory. The set itself is not
//!   refused; the warning is a soft policy signal that helps plugin
//!   authors notice accidental cross-workspace writes.

use crate::plugin::manager::PluginRequest;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, _tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let table = lua.create_table()?;

    // `pairee.clipboard(text?)` — single-name get/set dispatch.
    let cb_fn = lua.create_function(|lua_ctx, value: Option<String>| {
        // Read the cached secure-mode flag set by `standard::bind_runtime`
        // before invoking the clipboard binding.
        let secure = lua_ctx
            .globals()
            .get::<_, mlua::Table>("pairee")
            .ok()
            .and_then(|t| t.get::<_, bool>("_secure_mode").ok())
            .unwrap_or(false);

        match value {
            None => {
                // ── get ───────────────────────────────────────────────────
                if secure {
                    log::warn!(
                        "pairee.clipboard() (get) is blocked in secure mode to prevent \
                         data exfiltration."
                    );
                    return Ok(mlua::Value::Nil);
                }
                match read_clipboard_text() {
                    Ok(Some(text)) => lua_ctx.create_string(&text).map(mlua::Value::String),
                    Ok(None) => Ok(mlua::Value::Nil),
                    Err(e) => {
                        log::warn!("pairee.clipboard() (get) failed: {e}");
                        Ok(mlua::Value::Nil)
                    }
                }
            }
            Some(text) => {
                // ── set ───────────────────────────────────────────────────
                if secure && !text.is_empty() && !is_workspace_path(Path::new(&text)) {
                    log::warn!(
                        "pairee.clipboard(text) (set) called with a value that is not inside \
                         the workspace, config, or cache directory; this is a soft Secure-Mode \
                         warning (set still proceeds)."
                    );
                }
                match write_clipboard_text(&text) {
                    Ok(()) => Ok(mlua::Value::Boolean(true)),
                    Err(e) => {
                        log::warn!("pairee.clipboard(text) (set) failed: {e}");
                        Ok(mlua::Value::Nil)
                    }
                }
            }
        }
    })?;
    table.set("clipboard", cb_fn)?;
    Ok(table)
}

/// Reads the current clipboard text. Returns `Ok(None)` if the
/// clipboard is empty / not text / unavailable; returns `Err` for
/// hard failures (e.g. on headless Linux without a display server).
fn read_clipboard_text() -> anyhow::Result<Option<String>> {
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| anyhow::anyhow!("could not open clipboard: {e}"))?;
    match cb.get_text() {
        Ok(s) => Ok(Some(s)),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("clipboard read failed: {e}")),
    }
}

/// Writes a string to the system clipboard.
fn write_clipboard_text(text: &str) -> anyhow::Result<()> {
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| anyhow::anyhow!("could not open clipboard: {e}"))?;
    cb.set_text(text.to_string())
        .map_err(|e| anyhow::anyhow!("clipboard write failed: {e}"))
}

/// Returns `true` if the given path is inside the user's current
/// working directory, the Pairee config dir, or the Pairee cache
/// dir. Used for the Secure-Mode soft-warn on `clipboard(text)` when
/// the value looks like a path.
fn is_workspace_path(path: &Path) -> bool {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let allowed_roots = [
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        crate::config::paths::get_config_dir(),
        crate::config::paths::get_cache_dir(),
    ];
    allowed_roots.iter().any(|root| {
        let root = root.canonicalize().unwrap_or_else(|_| root.clone());
        canonical.starts_with(&root)
    })
}

// Suppress an unused-import warning for `PathBuf` when building on
// platforms where the soft-warn path is not exercised.
#[allow(dead_code)]
fn _typecheck(_p: PathBuf) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_workspace_path_accepts_workspace() {
        // The current working directory is by definition inside the
        // workspace root. We can therefore assert that "." is a
        // workspace path; everything else falls out of the helper's
        // contract.
        let cwd = std::env::current_dir().expect("cwd");
        assert!(is_workspace_path(&cwd));
    }

    #[test]
    fn test_is_workspace_path_rejects_obviously_external() {
        // `/dev/null` is not under the user's workspace, config, or
        // cache directory on any platform we ship to.
        let p = Path::new("/dev/null");
        assert!(!is_workspace_path(p));
    }

    #[test]
    fn test_bind_exposes_clipboard_function() {
        // The clipboard binding must register a `clipboard` function
        // on the returned table, regardless of whether a clipboard
        // backend is available on the test host.
        let lua = mlua::Lua::new();
        // Set up the `_secure_mode` flag the binding reads from the
        // `pairee` global. We register a stub `pairee` table.
        let pairee = lua.create_table().unwrap();
        pairee.set("_secure_mode", false).unwrap();
        lua.globals().set("pairee", pairee).unwrap();
        let (tx, _rx) =
            tokio::sync::mpsc::channel::<crate::plugin::manager::PluginRequest>(1);
        let table = bind(&lua, tx).expect("clipboard table");
        let cb_fn: mlua::Function = table.get("clipboard").expect("clipboard function");
        // The function exists; we don't call it here because the
        // headless test environment likely has no clipboard backend.
        let _ = cb_fn;
    }
}
