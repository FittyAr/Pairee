pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    let log_table = lua.create_table()?;

    log_table.set(
        "info",
        lua.create_function(|_, msg: String| {
            log::info!("{}", msg);
            Ok(())
        })?,
    )?;

    log_table.set(
        "warn",
        lua.create_function(|_, msg: String| {
            log::warn!("{}", msg);
            Ok(())
        })?,
    )?;

    log_table.set(
        "error",
        lua.create_function(|_, msg: String| {
            log::error!("{}", msg);
            Ok(())
        })?,
    )?;

    log_table.set(
        "debug",
        lua.create_function(|_, msg: String| {
            log::debug!("{}", msg);
            Ok(())
        })?,
    )?;

    Ok(log_table)
}
