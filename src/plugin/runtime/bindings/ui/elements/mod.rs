//! `ui/elements` subdirectory. Each widget module
//! follows the same pattern: a `pub struct Widget { ... }` + a
//! `pub fn bind(lua, parent) -> mlua::Result<()>` that registers
//! the `__call` metatable on `parent` under the widget name.

pub mod cell;
pub mod gauge;
pub mod line;
pub mod list;
pub mod paragraph;
pub mod span;
pub mod table;
pub mod text;
