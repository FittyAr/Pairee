use crate::plugin::manager::PluginRequest;
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let app = lua.create_table()?;

    // Read active cwd from snapshot
    app.set(
        "cwd",
        lua.create_function(|lua_ctx, ()| {
            let Some(t) = snapshot_table(lua_ctx)? else {
                return Ok(String::new());
            };
            let cwd_key = if active_panel(&t) == "left" {
                "left_cwd"
            } else {
                "right_cwd"
            };
            Ok(t.get::<_, String>(cwd_key).unwrap_or_default())
        })?,
    )?;

    // Read focused panel from snapshot
    app.set(
        "focus",
        lua.create_function(|lua_ctx, ()| {
            Ok(snapshot_table(lua_ctx)?
                .map(|t| active_panel(&t))
                .unwrap_or_else(|| "left".to_string()))
        })?,
    )?;

    // Get currently hovered file entry from snapshot
    app.set(
        "hovered",
        lua.create_function(|lua_ctx, ()| {
            Ok(snapshot_table(lua_ctx)?
                .map(|t| t.get("hovered_file").unwrap_or(mlua::Value::Nil))
                .unwrap_or(mlua::Value::Nil))
        })?,
    )?;

    // Navigate active panel to path, set focus side, popup notification
    app.set(
        "cd",
        fire_and_forget(lua, &tx, |path: String| PluginRequest::Cd { path })?,
    )?;
    app.set(
        "set_focus",
        fire_and_forget(lua, &tx, |side: String| PluginRequest::SetFocus { side })?,
    )?;
    app.set(
        "notify",
        fire_and_forget(lua, &tx, |(title, msg, level): (String, String, String)| {
            PluginRequest::Notify { title, msg, level }
        })?,
    )?;

    bind_legacy_dialogs(lua, &app, &tx)?;
    Ok(app)
}

/// Blocking confirm/input dialogs with the legacy signatures; they route
/// through the deprecated stub dispatcher in the main loop. New code calls
/// the top-level `pairee.confirm` / `pairee.input` registered in
/// `standard.rs` with the structured opts table.
fn bind_legacy_dialogs<'lua>(
    lua: &'lua mlua::Lua,
    app: &mlua::Table<'lua>,
    tx: &mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    let tx_confirm = tx.clone();
    app.set(
        "confirm",
        lua.create_async_function(move |_, (title, msg): (String, String)| {
            let tx = tx_confirm.clone();
            async move {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                let request = PluginRequest::Confirm {
                    title,
                    msg,
                    reply_tx,
                };
                if tx.send(request).await.is_ok() {
                    Ok(reply_rx.await.unwrap_or(false))
                } else {
                    Ok(false)
                }
            }
        })?,
    )?;

    let tx_input = tx.clone();
    app.set(
        "input",
        lua.create_async_function(move |_, (title, default): (String, String)| {
            let tx = tx_input.clone();
            async move {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                let request = PluginRequest::Input {
                    title,
                    default,
                    reply_tx,
                };
                if tx.send(request).await.is_ok() {
                    Ok(reply_rx.await.unwrap_or_default())
                } else {
                    Ok(String::new())
                }
            }
        })?,
    )?;
    Ok(())
}

/// Async Lua function that sends the request built from its arguments and returns nothing.
fn fire_and_forget<'lua, A>(
    lua: &'lua mlua::Lua,
    tx: &mpsc::Sender<PluginRequest>,
    make: fn(A) -> PluginRequest,
) -> mlua::Result<mlua::Function<'lua>>
where
    A: mlua::FromLuaMulti<'lua> + 'static,
{
    let tx = tx.clone();
    lua.create_async_function(move |_, args: A| {
        let tx = tx.clone();
        let request = make(args);
        async move {
            let _ = tx.send(request).await;
            Ok(())
        }
    })
}

/// `pairee.app._current_snapshot`, when a snapshot was taken.
fn snapshot_table(lua: &mlua::Lua) -> mlua::Result<Option<mlua::Table<'_>>> {
    let pairee: mlua::Table = lua.globals().get("pairee")?;
    let app_table: mlua::Table = pairee.get("app")?;
    Ok(match app_table.get::<_, mlua::Value>("_current_snapshot") {
        Ok(mlua::Value::Table(t)) => Some(t),
        _ => None,
    })
}

fn active_panel(snapshot: &mlua::Table) -> String {
    snapshot
        .get("active_panel")
        .unwrap_or_else(|_| "left".to_string())
}
