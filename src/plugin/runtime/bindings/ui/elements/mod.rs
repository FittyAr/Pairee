//! `ui/elements` subdirectory re-exports. Each widget module
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

pub use cell::Cell;
pub use gauge::Gauge;
pub use line::Line;
pub use list::List;
pub use paragraph::Paragraph;
pub use span::Span;
pub use table::{Row, Table};
pub use text::Text;
