//! `Output` and `Status` userdata returned by `Command:output()`,
//! `Command:status()`, and `Child:wait()`/`wait_with_output()`.

use mlua::{UserData, UserDataMethods};
use std::process::ExitStatus;

/// The M3 `Status` userdata.
#[derive(Debug, Clone, Copy)]
pub struct Status {
    pub success: bool,
    pub code: Option<i32>,
}

impl Status {
    pub fn from_exit(s: ExitStatus) -> Self {
        Self {
            success: s.success(),
            code: s.code(),
        }
    }
}

impl UserData for Status {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("success", |_lua, this, ()| Ok(this.success));
        methods.add_method("code", |_lua, this, ()| Ok(this.code));
    }
}

/// The M3 `Output` userdata.
#[derive(Debug, Clone)]
pub struct Output {
    pub status: Status,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn from_tokio(out: std::process::Output) -> Self {
        Self {
            status: Status::from_exit(out.status),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        }
    }
}

impl UserData for Output {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("status", |_lua, this, ()| {
            let ud = _lua.create_userdata(this.status)?;
            Ok(ud)
        });
        methods.add_method("stdout", |_lua, this, ()| {
            Ok(mlua::Value::String(_lua.create_string(&this.stdout)?))
        });
        methods.add_method("stderr", |_lua, this, ()| {
            Ok(mlua::Value::String(_lua.create_string(&this.stderr)?))
        });
    }
}
