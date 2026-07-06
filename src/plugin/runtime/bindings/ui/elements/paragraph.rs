//! M4-T2: `ui.Paragraph` userdata — wraps a `ui.Text` + paragraph-level Style.

use super::super::style::Style;
use super::text::Text;
use mlua::{MetaMethod, UserData, UserDataMethods};

/// A `ui.Paragraph(text)` userdata. The paragraph holds a `Text`
/// (which itself holds Lines of Spans) plus a paragraph-level
/// Style that propagates to every line when rendered.
#[derive(Debug, Clone)]
pub struct Paragraph {
    pub text: Text,
    pub style: Style,
    pub alignment: Option<String>,
    pub wrap: Option<String>,
}

impl Paragraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Text::from_string(text),
            style: Style::new(),
            alignment: None,
            wrap: None,
        }
    }

    pub fn from_text(text: Text) -> Self {
        Self {
            text,
            style: Style::new(),
            alignment: None,
            wrap: None,
        }
    }
}

impl UserData for Paragraph {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("style", |_lua, this, s: mlua::AnyUserData| {
            let s = s
                .borrow::<Style>()
                .map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            this.style = Style { inner: s.inner };
            Ok(this.clone())
        });
        methods.add_method_mut("fg", |_lua, this, val: mlua::Value| {
            let parsed = super::super::style::parse_color_value(val)?;
            let mut s = this.style.inner.clone();
            if let Some(c) = parsed { s = s.fg(c); } else { s.fg = None; }
            this.style.inner = s;
            Ok(this.clone())
        });
        methods.add_method_mut("bg", |_lua, this, val: mlua::Value| {
            let parsed = super::super::style::parse_color_value(val)?;
            let mut s = this.style.inner.clone();
            if let Some(c) = parsed { s = s.bg(c); } else { s.bg = None; }
            this.style.inner = s;
            Ok(this.clone())
        });
        methods.add_method_mut("bold", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::BOLD);
            Ok(this.clone())
        });
        methods.add_method_mut("italic", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::ITALIC);
            Ok(this.clone())
        });
        methods.add_method_mut("underline", |_lua, this, ()| {
            this.style.inner = this.style.inner.add_modifier(ratatui::style::Modifier::UNDERLINED);
            Ok(this.clone())
        });
        methods.add_method_mut("align", |_lua, this, align: String| {
            this.alignment = Some(align);
            Ok(this.clone())
        });
        methods.add_method_mut("wrap", |_lua, this, wrap: String| {
            this.wrap = Some(wrap);
            Ok(this.clone())
        });
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!(
                "Paragraph(lines={}, style={:?})",
                this.text.lines.len(),
                this.style.inner
            ))
        });
    }
}

/// `ui.Paragraph(text)` callable.
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let para = lua.create_table()?;
    para.set(
        "__call",
        lua.create_function(|lua_ctx, args: mlua::MultiValue| {
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let text = match first {
                mlua::Value::String(s) => Some(s.to_str().ok().map(|c| c.to_string()).unwrap_or_default()),
                _ => None,
            };
            let para = match text {
                Some(t) => Paragraph::new(t),
                None => Paragraph::new(""),
            };
            lua_ctx.create_userdata(para).map(mlua::Value::UserData)
        })?,
    )?;
    let mt = lua.create_table()?;
    let inner_call = para.get::<_, mlua::Function>("__call")?;
    mt.set("__call", inner_call)?;
    mt.set("__metatable", mlua::Value::Boolean(false))?;
    para.set_metatable(Some(mt));
    parent.set("Paragraph", para)
}
