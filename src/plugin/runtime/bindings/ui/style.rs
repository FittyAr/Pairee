//! `ui.Style` and the style methods every widget shares:
//! `:style(s)`, `:fg(c)`, `:bg(c)`, `:bold()`, `:dim()`, `:italic()`,
//! `:underline()`, `:blink()`, `:reverse()`, `:hidden()`, `:crossed()`,
//! `:reset()`. Each returns the widget itself, so calls chain:
//! `ui.Span("x"):fg("red"):bold()`.
//!
//! Colors are the names and `#rrggbb` strings of theme files; a `Style`
//! argument may also be given as a plain color string (foreground).

use crate::app::state::popup::plugin_widget::{
    WBorder, WGauge, WLine, WList, WParagraph, WRow, WSpan, WStyle, WTable, WText, modifier,
};
use mlua::{AnyUserData, UserData, UserDataMethods, Value};

/// A widget with a style the shared methods change.
pub trait Styled {
    fn style_mut(&mut self) -> &mut WStyle;
}

macro_rules! styled {
    ($($ty:ty),*) => {$(
        impl Styled for $ty {
            fn style_mut(&mut self) -> &mut WStyle {
                &mut self.style
            }
        }
    )*};
}
styled!(
    WSpan, WLine, WText, WParagraph, WList, WRow, WTable, WGauge, WBorder
);

impl Styled for WStyle {
    fn style_mut(&mut self) -> &mut WStyle {
        self
    }
}

/// A `Style` userdata, a color string (foreground) or `nil` (no style).
pub fn style_arg(value: &Value) -> mlua::Result<WStyle> {
    match value {
        Value::Nil => Ok(WStyle::default()),
        Value::String(s) => Ok(WStyle {
            fg: Some(s.to_string_lossy()),
            ..WStyle::default()
        }),
        Value::UserData(ud) => Ok(ud.borrow::<WStyle>()?.clone()),
        other => Err(mlua::Error::RuntimeError(format!(
            "expected a ui.Style or a color, got {}",
            other.type_name()
        ))),
    }
}

/// Modifier methods and their bits.
const MODIFIERS: &[(&str, u16)] = &[
    ("bold", modifier::BOLD),
    ("dim", modifier::DIM),
    ("italic", modifier::ITALIC),
    ("underline", modifier::UNDERLINED),
    ("blink", modifier::SLOW_BLINK),
    ("reverse", modifier::REVERSED),
    ("hidden", modifier::HIDDEN),
    ("crossed", modifier::CROSSED_OUT),
];

/// Adds the shared style methods to widget type `T`.
pub fn add_style_methods<T: Styled + 'static, M: UserDataMethods<T>>(methods: &mut M) {
    methods.add_function("style", |_, (ud, style): (AnyUserData, Value)| {
        let style = style_arg(&style)?;
        {
            let mut this = ud.borrow_mut::<T>()?;
            let current = this.style_mut();
            *current = current.patch(&style);
        }
        Ok(ud)
    });
    methods.add_function("fg", |_, (ud, color): (AnyUserData, String)| {
        ud.borrow_mut::<T>()?.style_mut().fg = Some(color);
        Ok(ud)
    });
    methods.add_function("bg", |_, (ud, color): (AnyUserData, String)| {
        ud.borrow_mut::<T>()?.style_mut().bg = Some(color);
        Ok(ud)
    });
    for &(name, bit) in MODIFIERS {
        methods.add_function(name, move |_, ud: AnyUserData| {
            ud.borrow_mut::<T>()?.style_mut().with(bit);
            Ok(ud)
        });
    }
    methods.add_function("reset", |_, ud: AnyUserData| {
        ud.borrow_mut::<T>()?.style_mut().reset();
        Ok(ud)
    });
}

impl UserData for WStyle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        methods.add_function("patch", |_, (ud, other): (AnyUserData, Value)| {
            let other = style_arg(&other)?;
            {
                let mut this = ud.borrow_mut::<WStyle>()?;
                *this = this.patch(&other);
            }
            Ok(ud)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_methods_chain_and_patch() {
        let lua = mlua::Lua::new();
        let new_style = lua.create_function(|_, ()| Ok(WStyle::default())).unwrap();
        lua.globals().set("Style", new_style).unwrap();
        let style: WStyle = lua
            .load(
                r##"local s = Style():fg("red"):bold():italic()
                   return s:patch(Style():bg("#102030"))"##,
            )
            .eval::<AnyUserData>()
            .unwrap()
            .borrow::<WStyle>()
            .unwrap()
            .clone();
        assert_eq!(style.fg.as_deref(), Some("red"));
        assert_eq!(style.bg.as_deref(), Some("#102030"));
        assert_eq!(style.add, modifier::BOLD | modifier::ITALIC);
        assert!(style_arg(&Value::Boolean(true)).is_err());
    }
}
