//! `pairee.Command` builder, `Child`, `Output`, and `Status`.

mod child;
mod command;
mod output;
mod stdio;

pub use command::{LuaCommand, bind};
pub use stdio::StdioKind;
