//! `pairee.utils.{uid, gid, user_name, group_name, host_name}`.
//!
//! Each returns the value or `nil` when the platform has no such thing
//! (numeric ids and the primary group on Windows), so plugins can stay
//! portable without checking `target_family` first. Available to every
//! plugin: the user's own account and machine names are low-sensitivity
//! information that the plugin could already read from paths it is given.

use crate::fs::identity;
use mlua::{Lua, Table};

/// Adds the identity functions to the `pairee.utils` table.
pub fn bind_into(lua: &Lua, utils: &Table) -> mlua::Result<()> {
    utils.set("uid", lua.create_function(|_, ()| Ok(identity::uid()))?)?;
    utils.set("gid", lua.create_function(|_, ()| Ok(identity::gid()))?)?;
    utils.set(
        "user_name",
        lua.create_function(|_, ()| Ok(identity::user_name()))?,
    )?;
    utils.set(
        "group_name",
        lua.create_function(|_, ()| Ok(identity::group_name()))?,
    )?;
    utils.set(
        "host_name",
        lua.create_function(|_, ()| Ok(identity::host_name()))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_functions_follow_the_platform() {
        let lua = Lua::new();
        let utils = lua.create_table().unwrap();
        bind_into(&lua, &utils).unwrap();
        lua.globals().set("u", utils).unwrap();
        let (uid_type, gid_type, host): (String, String, String) = lua
            .load("return type(u.uid()), type(u.gid()), u.host_name()")
            .eval()
            .unwrap();
        let expected = if cfg!(unix) { "number" } else { "nil" };
        assert_eq!((uid_type.as_str(), gid_type.as_str()), (expected, expected));
        assert!(!host.is_empty());
        let user: mlua::Value = lua.load("return u.user_name()").eval().unwrap();
        assert!(user.is_nil() || user.is_string());
    }
}
