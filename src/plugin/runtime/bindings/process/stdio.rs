//! `Stdio` enum + `Command.NULL`/`PIPED`/`INHERIT` constants.

use mlua::UserData;

/// The M3 process standard-IO configuration. The three values
/// mirror `tokio::process::Stdio`:
/// - `NULL` — redirect to `/dev/null` (or `NUL` on Windows).
/// - `PIPED` — capture into a pipe the plugin can read from.
/// - `INHERIT` — pass through to the parent process's terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stdio {
    Null,
    Piped,
    Inherit,
}

impl Stdio {
    pub fn to_tokio(self) -> tokio::process::Stdio {
        match self {
            Stdio::Null => tokio::process::Stdio::null(),
            Stdio::Piped => tokio::process::Stdio::piped(),
            Stdio::Inherit => tokio::process::Stdio::inherit(),
        }
    }
}

impl UserData for Stdio {}

/// Register the `Stdio` constants under the `Command` table so
/// plugins can write `Command.NULL`, `Command.PIPED`,
/// `Command.INHERIT`.
pub fn register(lua: &mlua::Lua, command_table: &mlua::Table<'_>) -> mlua::Result<()> {
    command_table.set("NULL", Stdio::Null)?;
    command_table.set("PIPED", Stdio::Piped)?;
    command_table.set("INHERIT", Stdio::Inherit)?;
    let _ = lua; // (kept for future per-VM state)
    Ok(())
}
