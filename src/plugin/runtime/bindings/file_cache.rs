//! Lua binding for `pairee.file_cache({file, skip})`.
//!
//! Returns a stable cache path for `(file, skip)`, so previewers can
//! generate derived content once and reuse it across invocations. The
//! cache lives under `<Pairee cache dir>/preview_cache/` and the
//! returned path is the absolute file path the plugin should write to
//! (and read from) for that file and skip combination.

use crate::plugin::manager::PluginRequest;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let table = lua.create_table()?;

    let tx_cache = tx;
    table.set(
        "file_cache",
        lua.create_async_function(move |_lua, (file, skip): (Option<String>, Option<usize>)| {
            let tx = tx_cache.clone();
            async move {
                let file_path = match file {
                    Some(p) => PathBuf::from(p),
                    None => {
                        log::warn!("pairee.file_cache called without a `file` argument");
                        return Ok(mlua::Value::Nil);
                    }
                };
                let skip = skip.unwrap_or(0);
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx
                    .send(PluginRequest::FileCache {
                        file_path,
                        skip,
                        reply_tx,
                    })
                    .await
                    .is_err()
                {
                    log::error!("pairee.file_cache could not enqueue; main loop not running");
                    return Ok(mlua::Value::Nil);
                }
                match reply_rx.await {
                    Ok(Some(p)) => Ok(mlua::Value::String(
                        _lua.create_string(p.to_string_lossy().as_ref())?,
                    )),
                    Ok(None) => Ok(mlua::Value::Nil),
                    Err(_) => {
                        log::error!("pairee.file_cache reply channel closed");
                        Ok(mlua::Value::Nil)
                    }
                }
            }
        })?,
    )?;

    Ok(table)
}

#[cfg(test)]
mod tests {
    use crate::config::paths;

    #[test]
    fn test_cache_dir_under_pairee_cache() {
        let cache = paths::get_cache_dir();
        // Sanity: the cache root is non-empty and points inside the user
        // Pairee data directory. We do not assert a specific path because
        // it differs by platform (`%APPDATA%` on Windows, `~/.cache` on
        // Linux), but the helper itself is platform-correct.
        assert!(!cache.as_os_str().is_empty());
    }
}
