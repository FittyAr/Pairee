//! `pairee.clipboard(text?)`: the system clipboard.
//!
//! * `pairee.clipboard()` returns `text, nil` or `nil, Error`.
//! * `pairee.clipboard(text)` returns `true, nil` or `false, Error`.
//!
//! Policy (decided in Rust when the runtime is bound, never read from Lua):
//!
//! * **Reading** is a data-exfiltration path (passwords and tokens end up on
//!   the clipboard), so only a trusted plugin outside Secure Mode may read;
//!   untrusted plugins and every plugin in Secure Mode get
//!   `nil, Error{kind = "PermissionDenied"}`.
//! * **Writing** is allowed to every plugin, but the text is capped at
//!   [`MAX_CLIPBOARD_BYTES`] and control characters other than tab and line
//!   feed (escape sequences, carriage returns, NUL) are removed, so a plugin
//!   cannot plant terminal escape sequences for the user to paste.

use crate::plugin::runtime::types::LuaError;
use mlua::{Lua, MultiValue, Value};
use std::sync::Arc;

/// Longest text a plugin may put on the clipboard.
pub const MAX_CLIPBOARD_BYTES: usize = 1024 * 1024;

/// Where the text goes (the OS clipboard, or a fake in tests).
pub trait ClipboardBackend: Send + Sync {
    fn get(&self) -> Result<String, String>;
    fn set(&self, text: &str) -> Result<(), String>;
}

/// The OS clipboard (the same helper Copy Path uses).
struct SystemClipboard;

impl ClipboardBackend for SystemClipboard {
    fn get(&self) -> Result<String, String> {
        crate::app::sys_helpers::clipboard::get_text()
    }
    fn set(&self, text: &str) -> Result<(), String> {
        crate::app::sys_helpers::clipboard::set_text(text)
    }
}

/// `text` without control characters other than `\t` and `\n`.
fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\t' | '\n'))
        .collect()
}

/// Binds `pairee.clipboard` on the OS clipboard; `can_read` is
/// `trusted && !secure_mode`.
pub fn bind(lua: &Lua, can_read: bool) -> mlua::Result<mlua::Function> {
    bind_with(lua, can_read, Arc::new(SystemClipboard))
}

fn bind_with(
    lua: &Lua,
    can_read: bool,
    backend: Arc<dyn ClipboardBackend>,
) -> mlua::Result<mlua::Function> {
    lua.create_function(
        move |lua, text: Option<mlua::LuaString>| -> mlua::Result<MultiValue> {
            match text {
                None => LuaError::pair(lua, read(can_read, backend.as_ref())),
                Some(text) => {
                    let result = write(&text.to_string_lossy(), backend.as_ref());
                    match result {
                        Ok(()) => LuaError::pair(lua, Ok::<_, LuaError>(true)),
                        Err(err) => Ok(MultiValue::from_vec(vec![
                            Value::Boolean(false),
                            Value::UserData(lua.create_userdata(err)?),
                        ])),
                    }
                }
            }
        },
    )
}

fn read(can_read: bool, backend: &dyn ClipboardBackend) -> Result<String, LuaError> {
    if !can_read {
        return Err(LuaError {
            kind: Some("PermissionDenied".into()),
            code: None,
            message:
                "pairee.clipboard: reading the clipboard needs a trusted plugin outside Secure Mode"
                    .into(),
        });
    }
    backend.get().map_err(LuaError::custom)
}

fn write(text: &str, backend: &dyn ClipboardBackend) -> Result<(), LuaError> {
    if text.len() > MAX_CLIPBOARD_BYTES {
        return Err(LuaError {
            kind: Some("InvalidInput".into()),
            code: None,
            message: format!("pairee.clipboard: text larger than {MAX_CLIPBOARD_BYTES} bytes"),
        });
    }
    backend.set(&sanitize(text)).map_err(LuaError::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake(Mutex<String>);

    impl ClipboardBackend for Fake {
        fn get(&self) -> Result<String, String> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn set(&self, text: &str) -> Result<(), String> {
            *self.0.lock().unwrap() = text.to_string();
            Ok(())
        }
    }

    fn lua_with(can_read: bool, fake: Arc<Fake>) -> Lua {
        let lua = Lua::new();
        let f = bind_with(&lua, can_read, fake).unwrap();
        lua.globals().set("clipboard", f).unwrap();
        lua
    }

    #[test]
    fn trusted_plugins_read_back_what_they_wrote() {
        let fake = Arc::new(Fake::default());
        let lua = lua_with(true, fake);
        let (ok, text): (bool, String) = lua
            .load("local ok = clipboard('hi there') return ok, clipboard()")
            .eval()
            .unwrap();
        assert!(ok);
        assert_eq!(text, "hi there");
    }

    #[test]
    fn reading_is_refused_without_read_rights_but_writing_works() {
        let fake = Arc::new(Fake::default());
        *fake.0.lock().unwrap() = "secret".into();
        let lua = lua_with(false, Arc::clone(&fake));
        let (value, kind, ok): (Value, String, bool) = lua
            .load("local v, e = clipboard() return v, e.kind, clipboard('x')")
            .eval()
            .unwrap();
        assert!(value.is_nil());
        assert_eq!(kind, "PermissionDenied");
        assert!(ok);
        assert_eq!(*fake.0.lock().unwrap(), "x");
    }

    #[test]
    fn written_text_loses_control_characters_and_is_capped() {
        let fake = Arc::new(Fake::default());
        let lua = lua_with(false, Arc::clone(&fake));
        lua.load("clipboard('a\\27[31mb\\r\\n\\tc\\0')")
            .exec()
            .unwrap();
        assert_eq!(*fake.0.lock().unwrap(), "a[31mb\n\tc");
        let (ok, kind): (bool, String) = lua
            .load("local ok, e = clipboard(string.rep('x', 1024 * 1024 + 1)) return ok, e.kind")
            .eval()
            .unwrap();
        assert!(!ok);
        assert_eq!(kind, "InvalidInput");
    }
}
