use crate::plugin::manager::PluginRequest;
use tokio::sync::mpsc;

pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let app = lua.create_table()?;
    let tx_clone = tx.clone();

    // Read active cwd from snapshot
    app.set(
        "cwd",
        lua.create_function(|lua_ctx, ()| {
            let globals = lua_ctx.globals();
            let pairee: mlua::Table = globals.get("pairee")?;
            let app_table: mlua::Table = pairee.get("app")?;
            if let Ok(snapshot_val) = app_table.get::<_, mlua::Value>("_current_snapshot") {
                if let mlua::Value::Table(ref t) = snapshot_val {
                    let active_panel: String =
                        t.get("active_panel").unwrap_or_else(|_| "left".to_string());
                    let resolved_cwd: String = if active_panel == "left" {
                        t.get("left_cwd").unwrap_or_default()
                    } else {
                        t.get("right_cwd").unwrap_or_default()
                    };
                    return Ok(resolved_cwd);
                }
            }
            Ok(String::new())
        })?,
    )?;

    // Navigate active panel to path
    let tx_cd = tx_clone.clone();
    app.set(
        "cd",
        lua.create_async_function(move |_, path: String| {
            let tx = tx_cd.clone();
            async move {
                let _ = tx.send(PluginRequest::Cd { path }).await;
                Ok(())
            }
        })?,
    )?;

    // Read focused panel from snapshot
    app.set(
        "focus",
        lua.create_function(|lua_ctx, ()| {
            let globals = lua_ctx.globals();
            let pairee: mlua::Table = globals.get("pairee")?;
            let app_table: mlua::Table = pairee.get("app")?;
            if let Ok(snapshot_val) = app_table.get::<_, mlua::Value>("_current_snapshot") {
                if let mlua::Value::Table(ref t) = snapshot_val {
                    let active_panel: String =
                        t.get("active_panel").unwrap_or_else(|_| "left".to_string());
                    return Ok(active_panel);
                }
            }
            Ok("left".to_string())
        })?,
    )?;

    // Set focus side
    let tx_focus = tx_clone.clone();
    app.set(
        "set_focus",
        lua.create_async_function(move |_, side: String| {
            let tx = tx_focus.clone();
            async move {
                let _ = tx.send(PluginRequest::SetFocus { side }).await;
                Ok(())
            }
        })?,
    )?;

    // Trigger popup notification
    let tx_notify = tx_clone.clone();
    app.set(
        "notify",
        lua.create_async_function(move |_, (title, msg, level): (String, String, String)| {
            let tx = tx_notify.clone();
            async move {
                let _ = tx.send(PluginRequest::Notify { title, msg, level }).await;
                Ok(())
            }
        })?,
    )?;

    // Blocking confirm dialog
    let tx_confirm = tx_clone.clone();
    app.set(
        "confirm",
        lua.create_async_function(move |_, (title, msg): (String, String)| {
            let tx = tx_confirm.clone();
            async move {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx
                    .send(PluginRequest::Confirm {
                        title,
                        msg,
                        reply_tx,
                    })
                    .await
                    .is_ok()
                {
                    Ok(reply_rx.await.unwrap_or(false))
                } else {
                    Ok(false)
                }
            }
        })?,
    )?;

    // Blocking input dialog
    let tx_input = tx_clone.clone();
    app.set(
        "input",
        lua.create_async_function(move |_, (title, default): (String, String)| {
            let tx = tx_input.clone();
            async move {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx
                    .send(PluginRequest::Input {
                        title,
                        default,
                        reply_tx,
                    })
                    .await
                    .is_ok()
                {
                    Ok(reply_rx.await.unwrap_or_default())
                } else {
                    Ok(String::new())
                }
            }
        })?,
    )?;

    // Get currently hovered file entry from snapshot
    app.set(
        "hovered",
        lua.create_function(|lua_ctx, ()| {
            let globals = lua_ctx.globals();
            let pairee: mlua::Table = globals.get("pairee")?;
            let app_table: mlua::Table = pairee.get("app")?;
            if let Ok(snapshot_val) = app_table.get::<_, mlua::Value>("_current_snapshot") {
                if let mlua::Value::Table(ref t) = snapshot_val {
                    let hovered: mlua::Value = t.get("hovered_file").unwrap_or(mlua::Value::Nil);
                    return Ok(hovered);
                }
            }
            Ok(mlua::Value::Nil)
        })?,
    )?;

    Ok(app)
}
