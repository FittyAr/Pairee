//! Lua binding for `pairee.emit(action, args)`.
//!
//! `pairee.emit` fires an action as if its key were pressed: any keymap id
//! (`"copy"`, `"go_to_tab_2"`, `"plugin.<name>.<command>"`) is queued for the
//! main loop. `cd` and `set_focus` (alias `focus`) also take arguments
//! (a path, a side). The call returns once the request is enqueued.
//!
//! Every request carries the [`PluginOrigin`] captured here in Rust, so the
//! main loop can apply the trust policy (untrusted plugins may only fire
//! the non-destructive actions of `manager::emit_policy`).

use crate::plugin::manager::{PluginOrigin, PluginRequest};
use crate::plugin::runtime::lua_json;
use tokio::sync::mpsc;

pub fn bind(
    lua: &mlua::Lua,
    tx: mpsc::Sender<PluginRequest>,
    origin: PluginOrigin,
) -> mlua::Result<mlua::Function<'_>> {
    lua.create_async_function(move |lua, (name, args): (String, Option<mlua::Value>)| {
        let tx = tx.clone();
        let origin = origin.clone();
        // Refused (sparse, cyclic, too large) arguments raise a Lua error
        // instead of being silently replaced.
        let args = lua_json::args_to_json(lua, args);
        async move { send(&tx, origin, name, args?).await }
    })
}

/// Queues `name` with `args` for the main loop on behalf of `origin`.
pub async fn send(
    tx: &mpsc::Sender<PluginRequest>,
    origin: PluginOrigin,
    name: String,
    args: serde_json::Value,
) -> mlua::Result<()> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let request = PluginRequest::EmitAction {
        origin,
        name: name.clone(),
        args,
        reply_tx: Some(reply_tx),
    };
    if tx.send(request).await.is_err() {
        tracing::error!("pairee.emit('{name}') could not enqueue; main loop not running");
        return Ok(());
    }
    // Fire-and-forget: the main loop runs the action later.
    let _ = reply_rx.await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> PluginOrigin {
        PluginOrigin {
            name: "demo".into(),
            trusted: false,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn emit_carries_origin_and_args() {
        let lua = mlua::Lua::new();
        let (tx, mut rx) = mpsc::channel(4);
        lua.globals()
            .set("emit", bind(&lua, tx, origin()).unwrap())
            .unwrap();
        let task = tokio::spawn(async move {
            match rx.recv().await {
                Some(PluginRequest::EmitAction {
                    origin,
                    name,
                    args,
                    reply_tx,
                }) => {
                    if let Some(r) = reply_tx {
                        let _ = r.send(serde_json::Value::Null);
                    }
                    (origin, name, args)
                }
                _ => panic!("expected EmitAction"),
            }
        });
        lua.load(r#"emit("cd", { path = "/tmp" })"#)
            .exec_async()
            .await
            .unwrap();
        let (got_origin, name, args) = task.await.unwrap();
        assert_eq!(got_origin, origin());
        assert_eq!(name, "cd");
        assert_eq!(args, serde_json::json!({ "path": "/tmp" }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sparse_args_raise_instead_of_allocating() {
        let lua = mlua::Lua::new();
        let (tx, _rx) = mpsc::channel(4);
        lua.globals()
            .set("emit", bind(&lua, tx, origin()).unwrap())
            .unwrap();
        let err = lua
            .load(r#"emit("x", { [2^60] = 1 })"#)
            .exec_async()
            .await
            .unwrap_err();
        assert!(err.to_string().contains("dense"), "{err}");
    }
}
