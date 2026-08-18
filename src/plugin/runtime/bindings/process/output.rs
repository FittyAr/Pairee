//! `Output` and `Status` userdata returned by `Command` / `Child`.

use mlua::{UserData, UserDataFields};

#[derive(Clone, Debug)]
pub struct LuaStatus {
    pub success: bool,
    pub code: Option<i32>,
}

impl LuaStatus {
    pub fn from_exit(status: std::process::ExitStatus) -> Self {
        Self {
            success: status.success(),
            code: status.code(),
        }
    }
}

impl UserData for LuaStatus {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("success", |_, this| Ok(this.success));
        fields.add_field_method_get("code", |_, this| Ok(this.code));
    }
}

#[derive(Clone, Debug)]
pub struct LuaOutput {
    pub status: LuaStatus,
    pub stdout: String,
    pub stderr: String,
}

impl UserData for LuaOutput {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("stdout", |_, this| Ok(this.stdout.clone()));
        fields.add_field_method_get("stderr", |_, this| Ok(this.stderr.clone()));
        fields.add_field_method_get("status", |_, this| Ok(this.status.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn output_fields_roundtrip_in_lua() {
        let lua = Lua::new();
        lua.globals()
            .set(
                "out",
                LuaOutput {
                    status: LuaStatus {
                        success: true,
                        code: Some(0),
                    },
                    stdout: "ok".into(),
                    stderr: String::new(),
                },
            )
            .unwrap();
        let stdout: String = lua.load("return out.stdout").eval().unwrap();
        let ok: bool = lua.load("return out.status.success").eval().unwrap();
        assert_eq!(stdout, "ok");
        assert!(ok);
    }
}
