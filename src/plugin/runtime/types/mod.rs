//! M2 typed-userdata wiring.
//!
//! Exposes the `Url` / `PathU` / `Cha` / `File` / `Error` types to
//! Lua, plus a tiny `Err(s, ...)` helper that plugins can call to
//! construct an ad-hoc `Error.custom` from a format string.

pub use crate::app::state::types::PluginWidget;
pub use crate::plugin::types::{Cha, ChaKind, ChaMode, Error, File, PathU, Url};

/// Registers every M2 typed userdata on the central `pairee` table:
/// `pairee.Url`, `pairee.Path`, `pairee.Cha`, `pairee.File`,
/// `pairee.Error`, and the global helper `Err(s, ...)`.
pub fn register(lua: &mlua::Lua, pairee: &mlua::Table<'_>) -> mlua::Result<()> {
    // `Url(string_or_url)` callable.
    let url = lua.create_table()?;
    url.set(
        "__call",
        lua.create_function(|lua, s: String| {
            let u = Url::parse(&s);
            lua.create_userdata(u).map(mlua::Value::UserData)
        })?,
    )?;
    // `Url.from(s)` for explicit table-style construction.
    url.set(
        "from",
        lua.create_function(|lua, s: String| {
            let u = Url::parse(&s);
            lua.create_userdata(u).map(mlua::Value::UserData)
        })?,
    )?;
    // Hide the metatable so plugins can't accidentally override Url.
    let url_mt = lua.create_table()?;
    url_mt.set("__metatable", mlua::Value::Boolean(false))?;
    url.set_metatable(Some(url_mt));
    pairee.set("Url", url)?;

    // `Path.os(string)` and `Path(string)` constructors.
    let path = lua.create_table()?;
    path.set(
        "os",
        lua.create_function(|lua, s: String| {
            let p = PathU::os(&s);
            lua.create_userdata(p).map(mlua::Value::UserData)
        })?,
    )?;
    path.set(
        "__call",
        lua.create_function(|lua, s: String| {
            let p = PathU::from_string(&s);
            lua.create_userdata(p).map(mlua::Value::UserData)
        })?,
    )?;
    let path_mt = lua.create_table()?;
    path_mt.set("__metatable", mlua::Value::Boolean(false))?;
    path.set_metatable(Some(path_mt));
    pairee.set("Path", path)?;

    // `Cha{...}` constructor from a Lua table.
    let cha = lua.create_table()?;
    cha.set(
        "__call",
        lua.create_function(|lua, opts: mlua::Table| {
            let mode_raw: u32 = opts.get::<_, u32>("mode").unwrap_or(0o644);
            let len: u64 = opts.get::<_, u64>("len").unwrap_or(0);
            let is_dir = opts.get::<_, bool>("is_dir").unwrap_or(false);
            let type_bit: u32 = if is_dir {
                ChaMode::T_DIR.0
            } else {
                ChaMode::T_FILE.0
            };
            let final_mode = ChaMode(mode_raw | type_bit);
            let cha = Cha {
                mode: final_mode,
                kind: ChaKind(0),
                len,
                atime: None,
                btime: None,
                mtime: None,
                uid: opts.get::<_, u32>("uid").ok(),
                gid: opts.get::<_, u32>("gid").ok(),
                nlink: 1,
            };
            lua.create_userdata(cha).map(mlua::Value::UserData)
        })?,
    )?;
    let cha_mt = lua.create_table()?;
    cha_mt.set("__metatable", mlua::Value::Boolean(false))?;
    cha.set_metatable(Some(cha_mt));
    pairee.set("Cha", cha)?;

    // `File{url=Url, cha=Cha}` constructor.
    let file = lua.create_table()?;
    file.set(
        "__call",
        lua.create_function(|lua, opts: mlua::Table| {
            // Two forms:
            //   File{url=Url, cha=Cha}
            //   File{path="/some/path"}  ← convenience
            if let Ok(url) = opts.get::<_, mlua::AnyUserData>("url") {
                let cha: Cha = opts
                    .get::<_, mlua::AnyUserData>("cha")
                    .ok()
                    .and_then(|u| u.borrow::<Cha>().ok().map(|c| c.clone()))
                    .unwrap_or_else(Cha::dummy);
                let url = url.borrow::<Url>().ok().map(|u| u.clone());
                match url {
                    Some(u) => {
                        let f = File {
                            url: u,
                            cha,
                            link_to: None,
                        };
                        lua.create_userdata(f).map(mlua::Value::UserData)
                    }
                    None => Ok(mlua::Value::Nil),
                }
            } else if let Ok(s) = opts.get::<_, mlua::String>("path") {
                let url = Url::parse(s.to_str().unwrap_or(""));
                let f = File::from_url(url);
                lua.create_userdata(f).map(mlua::Value::UserData)
            } else {
                Ok(mlua::Value::Nil)
            }
        })?,
    )?;
    let file_mt = lua.create_table()?;
    file_mt.set("__metatable", mlua::Value::Boolean(false))?;
    file.set_metatable(Some(file_mt));
    pairee.set("File", file)?;

    // `Error.custom` / `Error.fs`.
    crate::plugin::types::error::bind(lua, pairee)?;

    // `Err(s, ...)` global helper.
    let err_globals = lua.globals();
    let err_fn = lua.create_function(|lua, (fmt, args): (String, mlua::Variadic<mlua::Value>)| {
        // Format the string with the provided args using Lua's
        // `string.format` semantics. We do this from Rust by walking
        // the args and substituting them in order — `string.format`
        // is not exposed to us in mlua 0.9 without a Lua call, so
        // we hand-roll a simple "%s" substitution.
        let mut out = fmt.clone();
        for v in args.iter() {
            let piece = match v {
                mlua::Value::String(s) => s.to_str().map(|c| c.to_string()).unwrap_or_default(),
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                mlua::Value::Boolean(b) => b.to_string(),
                mlua::Value::Nil => "nil".to_string(),
                other => format!("{:?}", other),
            };
            if let Some(idx) = out.find("%s") {
                out.replace_range(idx..idx + 2, &piece);
            }
        }
        let e = Error::custom(out);
        lua.create_userdata(e).map(mlua::Value::UserData)
    })?;
    err_globals.set("Err", err_fn)?;

    let _ = crate::plugin::types::url::Scheme::Local; // keep re-export referenced
    Ok(())
}

