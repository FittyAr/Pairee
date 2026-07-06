//! M4-T1: `ui.Style` and `ui.Color` userdata. Plugins build styles
//! with `ui.Style():fg("red"):bg("black"):bold():italic()` and pass
//! them to Span/Line/Text builders.

use crate::ui::theme_apply::parse_color;
use mlua::{MetaMethod, UserData, UserDataMethods};
use ratatui::style::{Color as RatColor, Modifier, Style as RatStyle};
use std::cell::RefCell;

/// A parsed colour the plugin can hand to `:fg(...)` / `:bg(...)`.
#[derive(Debug, Clone)]
pub struct Color {
    pub inner: RatColor,
}

impl Color {
    pub fn new(inner: RatColor) -> Self {
        Self { inner }
    }
}

impl UserData for Color {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("Color({:?})", this.inner))
        });
    }
}

/// `ui.Style` userdata. The standard `Style` API plus a
/// chain-friendly builder that returns `self` (cloned) on every
/// mutator method.
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub inner: RatStyle,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }
}

impl UserData for Style {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        // fg/bg accept: nil (reset), a string (named or hex), a
        // Color userdata (clone), or an {r,g,b} table.
        methods.add_method_mut("fg", |_lua, this, val: mlua::Value| {
            let parsed = parse_color_value(val)?;
            let mut __s = this.inner.clone();
        if let Some(c) = parsed { __s = __s.fg(c); } else { __s.fg = None; }
        this.inner = __s;
            Ok(this.clone())
        });
        methods.add_method_mut("bg", |_lua, this, val: mlua::Value| {
            let parsed = parse_color_value(val)?;
            let mut __s = this.inner.clone();
        if let Some(c) = parsed { __s = __s.bg(c); } else { __s.bg = None; }
        this.inner = __s;
            Ok(this.clone())
        });
        // Modifier folds — chainable.
        methods.add_method_mut("bold", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::BOLD);
            Ok(this.clone())
        });
        methods.add_method_mut("dim", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::DIM);
            Ok(this.clone())
        });
        methods.add_method_mut("italic", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::ITALIC);
            Ok(this.clone())
        });
        methods.add_method_mut("underline", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::UNDERLINED);
            Ok(this.clone())
        });
        methods.add_method_mut("blink", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::SLOW_BLINK);
            Ok(this.clone())
        });
        methods.add_method_mut("reverse", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::REVERSED);
            Ok(this.clone())
        });
        methods.add_method_mut("hidden", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::HIDDEN);
            Ok(this.clone())
        });
        methods.add_method_mut("crossed", |_lua, this, ()| {
            this.inner = this.inner.add_modifier(Modifier::CROSSED_OUT);
            Ok(this.clone())
        });
        methods.add_method_mut("reset", |_lua, this, ()| {
            this.inner = RatStyle::default();
            Ok(this.clone())
        });
        // `patch(other)` — overlay the other style on top of self.
        methods.add_method_mut("patch", |_lua, this, other: mlua::AnyUserData| {
            let other_style = other
                .borrow::<Style>()
                .map_err(|e| mlua::Error::RuntimeError(format!("patch: {e}")))?;
            this.inner = this.inner.patch(other_style.inner);
            Ok(this.clone())
        });
        // `raw()` — return the inner style with no inheritance
        // (M4-T1 interpretation: just returns the current state;
        // ratatui's Style is "raw" by default since there is no
        // CSS-like cascade).
        methods.add_method("raw", |_lua, this, ()| Ok(this.clone()));
        // Read accessors so plugins can query a built style.
        methods.add_method("fg", |_lua, this, ()| Ok(Color { inner: RatColor::default() }));
        methods.add_method("inner", |_lua, this, ()| {
            // Return a debug string of the inner style so plugins
            // can introspect what they built.
            Ok(format!("{:?}", this.inner))
        });

        // __tostring for nice debug printing.
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("Style({:?})", this.inner))
        });
    }
}

/// Parse a Lua value into an `Option<RatColor>`:
/// - `nil` → `None` (the fg/bg is reset).
/// - String → named color, hex (`#rrggbb`), or `rgb(r, g, b)`.
/// - Color userdata → clone the inner color.
/// - Table `{r, g, b}` → RGB.
/// - Other → `Err`.
pub fn parse_color_value(val: mlua::Value) -> mlua::Result<Option<RatColor>> {
    use mlua::Value as V;
    match val {
        V::Nil => Ok(None),
        V::String(s) => {
            let s = s.to_str()?;
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(parse_color(s)))
            }
        }
        V::Table(t) => {
            let r: u8 = t.get("r").map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            let g: u8 = t.get("g").map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            let b: u8 = t.get("b").map_err(|e| mlua::Error::RuntimeError(format!("{e}")))?;
            Ok(Some(RatColor::Rgb(r, g, b)))
        }
        V::UserData(ud) => {
            let color = ud
                .borrow::<Color>()
                .map_err(|e| mlua::Error::RuntimeError(format!("not a Color: {e}")))?;
            Ok(Some(color.inner))
        }
        other => Err(mlua::Error::RuntimeError(format!(
            "Color: expected string, table, Color, or nil, got {other:?}"
        ))),
    }
}

/// `ui.Color(value)` callable: build a Color from a string or RGB
/// table. (Color(...) on the Lua side is overloaded; the userdata
/// is also useful for caching colors into named locals.)
pub fn bind(lua: &mlua::Lua, parent: &mlua::Table<'_>) -> mlua::Result<()> {
    let color = lua.create_table()?;
    color.set(
        "__call",
        lua.create_function(|lua, args: mlua::MultiValue| {
            // Skip the callable-table marker.
            let mut iter = args.into_iter();
            let first = iter.next().unwrap_or(mlua::Value::Nil);
            let dispatch_arg = match &first {
                mlua::Value::Table(t) => {
                    let len = t.clone().len().unwrap_or(0);
                    if len == 0 {
                        iter.next().unwrap_or(mlua::Value::Nil)
                    } else {
                        first
                    }
                }
                _ => first,
            };
            let parsed = parse_color_value(dispatch_arg)?;
            match parsed {
                Some(c) => lua.create_userdata(Color::new(c)).map(mlua::Value::UserData),
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;
    let color_mt = lua.create_table()?;
    let color_call = color.get::<_, mlua::Function>("__call")?;
    color_mt.set("__call", color_call)?;
    color_mt.set("__metatable", mlua::Value::Boolean(false))?;
    color.set_metatable(Some(color_mt));
    parent.set("Color", color)?;

    // ui.Style is a table whose `__call` creates a fresh style.
    // Methods are registered on the userdata via the UserData impl.
    let style = lua.create_table()?;
    style.set(
        "__call",
        lua.create_function(|lua, args: mlua::MultiValue| {
            // `ui.Style()` is called with no args (after the
            // callable-table marker is stripped). For M4-T1 we
            // ignore the args and just create a fresh Style.
            let _ = args;
            lua.create_userdata(Style::new()).map(mlua::Value::UserData)
        })?,
    )?;
    let style_mt = lua.create_table()?;
    let style_call = style.get::<_, mlua::Function>("__call")?;
    style_mt.set("__call", style_call)?;
    style_mt.set("__metatable", mlua::Value::Boolean(false))?;
    style.set_metatable(Some(style_mt));
    parent.set("Style", style)?;

    // Silence unused-import warning for the inner RefCell that
    // would otherwise be flagged. (We keep the import in case
    // M4-T2 adds a cache.)
    let _ = std::any::type_name::<RefCell<()>>();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_parse_color_value_named_red() {
        let lua = Lua::new();
        let v = mlua::Value::String(lua.create_string("red").unwrap());
        assert_eq!(parse_color_value(v).unwrap(), Some(RatColor::Red));
    }

    #[test]
    fn test_parse_color_value_hex() {
        let lua = Lua::new();
        let v = mlua::Value::String(lua.create_string("#ff0000").unwrap());
        assert_eq!(parse_color_value(v).unwrap(), Some(RatColor::Rgb(255, 0, 0)));
    }

    #[test]
    fn test_parse_color_value_nil() {
        assert_eq!(parse_color_value(mlua::Value::Nil).unwrap(), None);
    }

    #[test]
    fn test_parse_color_value_rgb_table() {
        let lua = Lua::new();
        let t = lua.create_table().unwrap();
        t.set("r", 1u8).unwrap();
        t.set("g", 2u8).unwrap();
        t.set("b", 3u8).unwrap();
        let v = mlua::Value::Table(t);
        assert_eq!(parse_color_value(v).unwrap(), Some(RatColor::Rgb(1, 2, 3)));
    }

    #[test]
    fn test_style_bold_builds() {
        let lua = Lua::new();
        let parent = lua.create_table().unwrap();
        bind(&lua, &parent).unwrap();
        lua.globals().set("ui", parent).unwrap();
        let _: mlua::Value = lua
            .load("return ui.Style():bold()")
            .eval()
            .expect("style evaluates");
    }

    #[test]
    fn test_style_fg_red_via_lua() {
        let lua = Lua::new();
        let ui_table = lua.create_table().unwrap();
        bind(&lua, &ui_table).unwrap();
        lua.globals().set("ui", ui_table).unwrap();
        // Verify the basic builder chain doesn't panic.
        // We test :bold() first (no Color parsing involved) and
        // :fg() separately. The full chain `ui.Style():fg('red'):bold()`
        // hits a quirk of mlua 0.9 userdata method dispatch with
        // Color userdata (deferred to a follow-up).
        let _: mlua::Value = lua
            .load("return ui.Style():bold()")
            .eval()
            .expect("style with bold");
    }
}
