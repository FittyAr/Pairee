//! M4-T2: `ui.Gauge` userdata — a progress bar.

use mlua::{MetaMethod, UserData, UserDataMethods};

/// A `ui.Gauge():ratio(r):label(span)` userdata.
#[derive(Debug, Clone)]
pub struct Gauge {
    pub ratio: f64,
    pub label: String,
}

impl Gauge {
    pub fn new() -> Self {
        Self { ratio: 0.0, label: String::new() }
    }
}

impl Default for Gauge { fn default() -> Self { Self::new() } }

impl UserData for Gauge {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("ratio", |_lua, this, r: f64| {
            this.ratio = r.clamp(0.0, 1.0);
            Ok(this.clone())
        });
        methods.add_method_mut("percent", |_lua, this, p: f64| {
            this.ratio = (p.clamp(0.0, 100.0)) / 100.0;
            Ok(this.clone())
        });
        methods.add_method_mut("label", |_lua, this, s: String| {
            this.label = s;
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("Gauge({}%)", (this.ratio * 100.0) as u16))
        });
    }
}

/// `ui.Gauge()` callable.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let gauge = lua.create_table()?;
    gauge.set("__call", lua.create_function(|lua_ctx, _: mlua::MultiValue| {
        lua_ctx.create_userdata(Gauge::new()).map(mlua::Value::UserData)
    })?)?;
    let mt = lua.create_table()?;
    mt.set("__call", gauge.get::<_, mlua::Function>("__call")?)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    gauge.set_metatable(Some(mt));
    parent.set("Gauge", gauge)
}
