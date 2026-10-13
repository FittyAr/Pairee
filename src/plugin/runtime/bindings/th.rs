//! `pairee.th`: the active theme as `ui.Style` values, so plugin widgets
//! can match the user's colors (`ui.Span("x"):style(pairee.th.header_fg)`).
//!
//! `th.name` is the theme name; every color of the theme file is a style
//! under its own key (`panel_fg`, `panel_bg`, `selection_bg`,
//! `popup_border`, ...): keys ending in `_bg` set the background, the others
//! the foreground. The values are those of the theme the plugins were
//! loaded with. Each read of a key gives a new style, so changing it does
//! not change what the next read returns.

use crate::app::state::popup::plugin_widget::WStyle;
use crate::plugin::load_options::HostSnapshot;
use mlua::{Lua, Table, Value};

fn style_of(key: &str, color: &str) -> WStyle {
    let color = Some(color.to_string());
    if key.ends_with("_bg") {
        WStyle {
            bg: color,
            ..WStyle::default()
        }
    } else {
        WStyle {
            fg: color,
            ..WStyle::default()
        }
    }
}

/// Builds `pairee.th` from the host snapshot.
pub fn bind(lua: &Lua, host: &HostSnapshot) -> mlua::Result<Table> {
    let th = lua.create_table()?;
    th.set("name", host.theme_name.as_str())?;
    let colors: std::collections::HashMap<String, String> =
        host.theme_colors.iter().cloned().collect();
    let meta = lua.create_table()?;
    meta.set(
        "__index",
        lua.create_function(
            move |lua, (_, key): (Table, String)| match colors.get(&key) {
                Some(color) => Ok(Value::UserData(lua.create_userdata(style_of(&key, color))?)),
                None => Ok(Value::Nil),
            },
        )?,
    )?;
    th.set_metatable(Some(meta))?;
    Ok(th)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colors_become_styles() {
        let lua = Lua::new();
        let host = HostSnapshot {
            theme_name: "slate".into(),
            theme_colors: vec![
                ("panel_fg".into(), "White".into()),
                ("panel_bg".into(), "#000010".into()),
            ],
            ..HostSnapshot::default()
        };
        lua.globals().set("th", bind(&lua, &host).unwrap()).unwrap();
        let name: String = lua.load("return th.name").eval().unwrap();
        assert_eq!(name, "slate");
        let fg: mlua::AnyUserData = lua.load("return th.panel_fg").eval().unwrap();
        assert_eq!(fg.borrow::<WStyle>().unwrap().fg.as_deref(), Some("White"));
        let bg: mlua::AnyUserData = lua.load("return th.panel_bg").eval().unwrap();
        assert_eq!(
            bg.borrow::<WStyle>().unwrap().bg.as_deref(),
            Some("#000010")
        );
        let missing: Value = lua.load("return th.nope").eval().unwrap();
        assert!(missing.is_nil());
        let fresh: bool = lua
            .load("th.panel_fg:bold() return th.panel_fg ~= nil")
            .eval()
            .unwrap();
        assert!(fresh);
    }
}
