//! `pairee.state`: small key/value storage a plugin keeps between calls
//! and between sessions (counters, caches, "last seen" values).
//!
//! ```lua
//! pairee.state.get(key)        --> value or nil
//! pairee.state.set(key, value) --> true, nil | false, Error  (nil deletes)
//! pairee.state.all()           --> table (a copy)
//! pairee.state.clear()         --> true, nil | false, Error
//! ```
//!
//! Values are anything the shared bounded Lua ⇄ JSON converter accepts
//! (scalars, dense lists, string-keyed maps). The store lives in
//! `state.json` inside the plugin's private data directory; the path is
//! fixed in Rust, so the plugin cannot point it anywhere else. It is loaded
//! on first use (a file larger than [`MAX_STATE_BYTES`] or that does not
//! parse is ignored), and every change is written at once through
//! `config::write_atomic` (temporary file + rename, `0600` on Unix), so a
//! crash never leaves a half-written store. A change that would make the
//! serialized store larger than [`MAX_STATE_BYTES`] is refused and leaves
//! the store as it was.

use crate::plugin::runtime::lua_json;
use crate::plugin::runtime::types::LuaError;
use mlua::{Lua, MultiValue, Table, Value};
use serde_json::{Map, Value as Json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Largest serialized store, in bytes.
pub const MAX_STATE_BYTES: usize = 256 * 1024;
/// Longest key, in bytes.
pub const MAX_KEY_BYTES: usize = 256;
/// File name of the store inside the plugin data directory.
pub const STATE_FILE: &str = "state.json";

/// The plugin's store: loaded lazily, written through on every change.
struct Store {
    path: PathBuf,
    values: Option<Map<String, Json>>,
}

impl Store {
    fn values(&mut self) -> &mut Map<String, Json> {
        let path = &self.path;
        self.values.get_or_insert_with(|| load(path))
    }

    /// Applies `change` to a copy and keeps it only if it could be saved.
    fn update(&mut self, change: impl FnOnce(&mut Map<String, Json>)) -> Result<(), LuaError> {
        let mut next = self.values().clone();
        change(&mut next);
        let bytes = serde_json::to_vec(&next).map_err(|e| LuaError::custom(e.to_string()))?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(LuaError {
                kind: Some("StorageFull".into()),
                code: None,
                message: format!("pairee.state: the store would exceed {MAX_STATE_BYTES} bytes"),
            });
        }
        crate::config::write_atomic(&self.path, &bytes)
            .map_err(|e| LuaError::custom(format!("pairee.state: {e:#}")))?;
        self.values = Some(next);
        Ok(())
    }
}

/// The saved store, or an empty one when it is missing, too large or
/// not a JSON object.
fn load(path: &Path) -> Map<String, Json> {
    let too_big = std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_STATE_BYTES as u64);
    if too_big {
        tracing::warn!("Ignoring plugin state {path:?}: larger than {MAX_STATE_BYTES} bytes");
        return Map::new();
    }
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Json>(&bytes).ok())
        .and_then(|json| match json {
            Json::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

fn check_key(key: &str) -> Result<(), LuaError> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES {
        return Err(LuaError {
            kind: Some("InvalidInput".into()),
            code: None,
            message: format!("pairee.state: keys must be 1 to {MAX_KEY_BYTES} bytes long"),
        });
    }
    Ok(())
}

/// `true, nil` or `false, Error`.
fn outcome(lua: &Lua, result: Result<(), LuaError>) -> mlua::Result<MultiValue> {
    Ok(match result {
        Ok(()) => MultiValue::from_vec(vec![Value::Boolean(true), Value::Nil]),
        Err(err) => MultiValue::from_vec(vec![
            Value::Boolean(false),
            Value::UserData(lua.create_userdata(err)?),
        ]),
    })
}

fn lock(store: &Mutex<Store>) -> mlua::Result<std::sync::MutexGuard<'_, Store>> {
    store
        .lock()
        .map_err(|_| mlua::Error::RuntimeError("pairee.state: store poisoned".into()))
}

/// Binds `pairee.state` on `<data_dir>/state.json`.
pub fn bind(lua: &Lua, data_dir: &Path) -> mlua::Result<Table> {
    let store = Arc::new(Mutex::new(Store {
        path: data_dir.join(STATE_FILE),
        values: None,
    }));
    let table = lua.create_table()?;

    let s = Arc::clone(&store);
    table.set(
        "get",
        lua.create_function(move |lua, key: String| {
            let value = lock(&s)?.values().get(&key).cloned();
            value.map_or(Ok(Value::Nil), |v| lua_json::from_json(lua, &v))
        })?,
    )?;

    let s = Arc::clone(&store);
    table.set(
        "set",
        lua.create_function(move |lua, (key, value): (String, Value)| {
            let result = check_key(&key).and_then(|()| {
                let json = match value {
                    Value::Nil => None,
                    other => Some(
                        lua_json::to_json(lua, other)
                            .map_err(|e| LuaError::custom(format!("pairee.state: {e}")))?,
                    ),
                };
                lock(&s)
                    .map_err(|e| LuaError::custom(e.to_string()))?
                    .update(|map| match json {
                        Some(json) => {
                            map.insert(key, json);
                        }
                        None => {
                            map.remove(&key);
                        }
                    })
            });
            outcome(lua, result)
        })?,
    )?;

    let s = Arc::clone(&store);
    table.set(
        "all",
        lua.create_function(move |lua, ()| {
            let all = Json::Object(lock(&s)?.values().clone());
            lua_json::from_json(lua, &all)
        })?,
    )?;

    let s = store;
    table.set(
        "clear",
        lua.create_function(move |lua, ()| {
            let result = lock(&s)
                .map_err(|e| LuaError::custom(e.to_string()))
                .and_then(|mut store| store.update(Map::clear));
            outcome(lua, result)
        })?,
    )?;
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua_with_state(dir: &Path) -> Lua {
        let lua = Lua::new();
        lua.globals()
            .set("state", bind(&lua, dir).unwrap())
            .unwrap();
        lua
    }

    #[test]
    fn values_persist_across_runtimes() {
        let dir = tempfile::tempdir().unwrap();
        let lua = lua_with_state(dir.path());
        let ok: bool = lua
            .load("return state.set('count', 3) and state.set('list', {1, 2, 'x'})")
            .eval()
            .unwrap();
        assert!(ok);
        let lua = lua_with_state(dir.path());
        let (count, third): (i64, String) = lua
            .load("return state.get('count'), state.get('list')[3]")
            .eval()
            .unwrap();
        assert_eq!((count, third.as_str()), (3, "x"));
        let missing: Value = lua.load("return state.get('nope')").eval().unwrap();
        assert!(missing.is_nil());
    }

    #[test]
    fn nil_deletes_and_clear_empties() {
        let dir = tempfile::tempdir().unwrap();
        let lua = lua_with_state(dir.path());
        let n: i64 = lua
            .load(
                r#"
                state.set('a', 1) state.set('b', 2) state.set('a', nil)
                local n = 0 for _ in pairs(state.all()) do n = n + 1 end
                return n
                "#,
            )
            .eval()
            .unwrap();
        assert_eq!(n, 1);
        let empty: bool = lua
            .load("state.clear() return next(state.all()) == nil")
            .eval()
            .unwrap();
        assert!(empty);
    }

    #[test]
    fn oversized_changes_and_bad_keys_are_refused_without_effect() {
        let dir = tempfile::tempdir().unwrap();
        let lua = lua_with_state(dir.path());
        let (ok, kind, kept): (bool, String, i64) = lua
            .load(
                r#"
                state.set('keep', 7)
                local ok, err = state.set('big', string.rep('x', 300 * 1024))
                return ok, err.kind, state.get('keep')
                "#,
            )
            .eval()
            .unwrap();
        assert!(!ok);
        assert_eq!((kind.as_str(), kept), ("StorageFull", 7));
        let big: Value = lua.load("return state.get('big')").eval().unwrap();
        assert!(big.is_nil());
        let (ok, kind): (bool, String) = lua
            .load("local ok, e = state.set('', 1) return ok, e.kind")
            .eval()
            .unwrap();
        assert!(!ok);
        assert_eq!(kind, "InvalidInput");
        let (ok, sparse): (bool, String) = lua
            .load("local ok, e = state.set('s', {[2^40] = 1}) return ok, tostring(e)")
            .eval()
            .unwrap();
        assert!(!ok && sparse.contains("pairee.state"));
    }

    #[test]
    fn a_corrupt_or_huge_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(STATE_FILE), b"not json").unwrap();
        let lua = lua_with_state(dir.path());
        let ok: bool = lua
            .load("return next(state.all()) == nil and state.set('a', 1)")
            .eval()
            .unwrap();
        assert!(ok);
        std::fs::write(dir.path().join(STATE_FILE), vec![b' '; MAX_STATE_BYTES + 1]).unwrap();
        let lua = lua_with_state(dir.path());
        let empty: bool = lua.load("return next(state.all()) == nil").eval().unwrap();
        assert!(empty);
    }
}
