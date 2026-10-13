use crate::plugin::load_options::PluginLoadOptions;
use crate::plugin::loader::PluginManifest;
use crate::plugin::manager::{PluginOrigin, PluginRequest};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Builds the `pairee` global for the plugin in `plugin_dir`.
///
/// Everything the host decided (`options`: trust, Secure Mode, user
/// settings) is captured by the Rust closures here; the Lua copies
/// (`pairee._secure_mode`, `pairee.settings`) are informational only.
pub fn bind_runtime(
    lua: &mlua::Lua,
    plugin_dir: &Path,
    options: &PluginLoadOptions,
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    let globals = lua.globals();
    let pairee = lua.create_table()?;
    let trusted = options.trusted;

    // 1. Secure Mode comes from the host; nothing in Rust reads the Lua copy.
    pairee.set("_secure_mode", options.secure_mode)?;
    pairee.set("_lua_api_version", super::api_version::LUA_API_VERSION)?;
    let manifest = PluginManifest::read_from_dir(plugin_dir);
    let declared_commands = manifest
        .as_ref()
        .map(|m| m.permissions.commands.clone())
        .unwrap_or_default();
    let command_policy = Arc::new(crate::plugin::command_policy::CommandPolicy::new(
        trusted,
        options.secure_mode,
        &declared_commands,
    ));
    let fs_policy = Arc::new(super::bindings::fs::FsPolicy::new(
        plugin_dir,
        trusted,
        options.secure_mode,
    ));
    let origin = PluginOrigin {
        name: options.name.clone(),
        trusted,
    };

    // 2. Bind submodules
    use super::bindings as b;
    pairee.set("app", b::app::bind(lua, tx.clone(), origin.clone())?)?;
    pairee.set("emit", b::emit::bind(lua, tx.clone(), origin)?)?;
    pairee.set("file_cache", b::file_cache::bind(lua, tx.clone())?)?;
    // Dialogs write their entries directly on the central `pairee`
    // table so that the top-level `pairee.confirm` and `pairee.input`
    // are discoverable alongside the existing `pairee.app.*` stubs.
    b::dialogs::bind(lua, &pairee, tx.clone())?;
    pairee.set(
        "fs",
        b::fs::bind(
            lua,
            Arc::clone(&fs_policy),
            command_policy.clone(),
            tx.clone(),
        )?,
    )?;
    pairee.set(
        "Command",
        b::process::bind(lua, command_policy, Arc::clone(&fs_policy))?,
    )?;
    pairee.set("ui", b::ui::bind(lua)?)?;
    pairee.set("ps", b::ps::bind(lua, tx.clone())?)?;
    pairee.set("log", b::log::bind(lua)?)?;
    pairee.set("keymap", b::keymap::bind(lua)?)?;
    pairee.set("notify", b::notify_ext::bind(lua, tx.clone())?)?;
    pairee.set("sync", b::sync::bind(lua, tx.clone())?)?;
    pairee.set("utils", b::utils_ext::bind(lua)?)?;
    pairee.set(
        "clipboard",
        b::clipboard::bind(lua, trusted && !options.secure_mode)?,
    )?;
    pairee.set("Error", super::types::error::bind(lua)?)?;
    pairee.set("Path", super::types::path::bind(lua)?)?;
    pairee.set("Url", super::types::url::bind(lua)?)?;
    pairee.set("Cha", super::types::cha::bind(lua)?)?;
    pairee.set("state", b::state::bind(lua, &fs_policy.data_dir)?)?;
    pairee.set("which", b::which::bind(lua, tx.clone())?)?;
    b::cx::bind_empty(lua, &pairee)?;

    // 3. Settings (schema defaults + the user's values from the host)
    let settings = super::settings::build(lua, manifest.as_ref(), &options.settings)?;
    pairee.set("settings", settings)?;

    // 4. i18n translation helper: pairee.t
    let t = super::i18n::bind(lua, plugin_dir, manifest.as_ref())?;
    pairee.set("t", t)?;

    b::preview::bind(
        lua,
        &pairee,
        (
            &options.name,
            Arc::clone(&fs_policy),
            crate::syntax::SyntaxLimits {
                enabled: options.host.syntax_highlighting,
                max_bytes: options.host.syntax_max_bytes,
            },
        ),
        tx.clone(),
    )?;

    // 5. Namespaces built on first use.
    lazy_namespaces(lua, &pairee, options, plugin_dir, &fs_policy)?;

    globals.set("pairee", pairee)?;
    Ok(())
}

/// `pairee.image`, `pairee.rt` and `pairee.th`, built on first access.
fn lazy_namespaces(
    lua: &mlua::Lua,
    pairee: &mlua::Table,
    options: &PluginLoadOptions,
    plugin_dir: &Path,
    fs_policy: &Arc<super::bindings::fs::FsPolicy>,
) -> mlua::Result<()> {
    use super::bindings as b;
    use super::composer::builder;
    use mlua::Value;
    let image_policy = Arc::clone(fs_policy);
    let (rt_options, rt_dir, rt_data) = (
        options.clone(),
        plugin_dir.to_path_buf(),
        fs_policy.data_dir.clone(),
    );
    let host = options.host.clone();
    super::composer::install(
        lua,
        pairee,
        vec![
            (
                "image",
                builder(move |lua| {
                    b::image::bind(lua, Arc::clone(&image_policy)).map(Value::Table)
                }),
            ),
            (
                "rt",
                builder(move |lua| {
                    b::rt::bind(lua, &rt_options, &rt_dir, &rt_data).map(Value::Table)
                }),
            ),
            (
                "th",
                builder(move |lua| b::th::bind(lua, &host).map(Value::Table)),
            ),
        ],
    )
}
