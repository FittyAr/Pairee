pub fn bind(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    let ui = lua.create_table()?;

    ui.set(
        "Paragraph",
        lua.create_function(|lua_ctx, text: String| {
            let t = lua_ctx.create_table()?;
            t.set("type", "Paragraph")?;
            t.set("text", text)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "Gauge",
        lua.create_function(|lua_ctx, (ratio, label): (f64, String)| {
            let t = lua_ctx.create_table()?;
            t.set("type", "Gauge")?;
            t.set("ratio", ratio)?;
            t.set("label", label)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "List",
        lua.create_function(|lua_ctx, items: Vec<String>| {
            let t = lua_ctx.create_table()?;
            t.set("type", "List")?;
            t.set("items", items)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "Table",
        lua.create_function(
            |lua_ctx, (headers, rows): (Vec<String>, Vec<Vec<String>>)| {
                let t = lua_ctx.create_table()?;
                t.set("type", "Table")?;
                t.set("headers", headers)?;
                t.set("rows", rows)?;
                Ok(t)
            },
        )?,
    )?;

    ui.set(
        "Span",
        lua.create_function(|lua_ctx, (text, style): (String, String)| {
            let t = lua_ctx.create_table()?;
            t.set("type", "Span")?;
            t.set("text", text)?;
            t.set("style", style)?;
            Ok(t)
        })?,
    )?;

    ui.set(
        "Line",
        lua.create_function(|lua_ctx, spans: Vec<mlua::Table>| {
            let t = lua_ctx.create_table()?;
            t.set("type", "Line")?;
            t.set("spans", spans)?;
            Ok(t)
        })?,
    )?;

    Ok(ui)
}
