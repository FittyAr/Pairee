//! `pairee.image` (G12): images in plugin previews, drawn by the quick
//! view's own image renderer.
//!
//! * `image.show(path)` → an image widget for `peek` to return (or to put
//!   in a `ui.Layout`), `nil, Error` when the file may not be shown. The
//!   host decodes the file after `peek` returns.
//! * `image.info(path)` → `{w, h, format, color}`, or `nil, Error`.
//! * `image.precache(src, dst, {max_width?, max_height?})` → `true` or
//!   `false, Error`: writes a PNG of `src` scaled to fit (1024×1024 by
//!   default, aspect kept) to `dst`, e.g. a cache file in the plugin's data
//!   directory, for `preload` to prepare thumbnails.
//!
//! Sandbox: `show` and `info` accept the file being previewed (the
//! `peek` / `preload` / `seek` job's file, which the user is looking at
//! anyway; the plugin only names it and gets an opaque widget or the
//! dimensions back) and any file the plugin's `pairee.fs` read jail
//! allows. `precache` decodes pixels the plugin could then read from
//! `dst`, so its source must pass the read jail (never the preview
//! exception) and `dst` the write jail. Files larger than the quick view's
//! size cap are refused, and decoding is bounded by `image` limits.

use super::fs::{Access, FsPolicy, Target};
use crate::app::state::popup::plugin_widget::PluginImage;
use crate::app::state::quick_view::load::QUICK_VIEW_MAX_BYTES;
use crate::plugin::runtime::types::LuaError;
use crate::plugin::runtime::types::path::path_arg;
use crate::plugin::worker::PreviewTarget;
use image::{ImageDecoder, ImageReader, Limits};
use mlua::{Lua, MultiValue, Table, UserData, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default bounds of `precache`.
pub const PRECACHE_MAX_SIDE: u32 = 1024;
/// Largest image (in pixels per side) the plugin helpers decode.
const MAX_SIDE: u32 = 16_384;
/// Most memory one decode may allocate.
const MAX_ALLOC: u64 = 256 * 1024 * 1024;

/// An image widget is opaque to Lua.
impl UserData for PluginImage {}

fn denied(message: String) -> LuaError {
    LuaError {
        kind: Some("PermissionDenied".into()),
        code: None,
        message,
    }
}

/// The path of `value` if the plugin may show it: the previewed file or a
/// path its read jail allows (`preview_ok = false` skips the former).
fn showable(
    lua: &Lua,
    policy: &FsPolicy,
    value: &Value,
    preview_ok: bool,
) -> Result<PathBuf, LuaError> {
    let path = path_arg(value).map_err(|e| LuaError::custom(e.to_string()))?;
    if preview_ok && PreviewTarget::is_current(lua, &path) {
        return Ok(path);
    }
    policy
        .check(&path.to_string_lossy(), Access::Read)
        .map(|target| target.path().to_path_buf())
        .map_err(denied)
}

/// A reader for `path` with format detection and decode limits, after the
/// size check.
fn reader(path: &Path) -> Result<ImageReader<std::io::BufReader<std::fs::File>>, LuaError> {
    let size = std::fs::metadata(path)?.len();
    if size > QUICK_VIEW_MAX_BYTES {
        return Err(LuaError {
            kind: Some("FileTooLarge".into()),
            code: None,
            message: format!(
                "pairee.image: {} is larger than {QUICK_VIEW_MAX_BYTES} bytes",
                path.display()
            ),
        });
    }
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    Ok(reader)
}

fn image_error(e: image::ImageError) -> LuaError {
    LuaError {
        kind: Some("InvalidData".into()),
        code: None,
        message: format!("pairee.image: {e}"),
    }
}

fn info(lua: &Lua, path: &Path) -> Result<Table, LuaError> {
    let reader = reader(path)?;
    let format = reader.format();
    let decoder = reader.into_decoder().map_err(image_error)?;
    let (w, h) = decoder.dimensions();
    let table = lua
        .create_table()
        .map_err(|e| LuaError::custom(e.to_string()))?;
    let fill = || -> mlua::Result<()> {
        table.set("w", w)?;
        table.set("h", h)?;
        table.set("format", format.map(|f| format!("{f:?}").to_lowercase()))?;
        table.set(
            "color",
            format!("{:?}", decoder.color_type()).to_lowercase(),
        )?;
        Ok(())
    };
    fill().map_err(|e| LuaError::custom(e.to_string()))?;
    Ok(table)
}

fn precache(src: &Path, dst: &Target, (max_w, max_h): (u32, u32)) -> Result<bool, LuaError> {
    let image = reader(src)?.decode().map_err(image_error)?;
    let thumb = image.thumbnail(max_w.max(1), max_h.max(1));
    let mut png = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(image_error)?;
    dst.write(png.into_inner())?;
    Ok(true)
}

/// `false, Error` / `true, nil` for `precache`.
fn outcome(lua: &Lua, result: Result<bool, LuaError>) -> mlua::Result<MultiValue> {
    match result {
        Ok(done) => LuaError::pair(lua, Ok::<_, LuaError>(done)),
        Err(err) => Ok(MultiValue::from_vec(vec![
            Value::Boolean(false),
            Value::UserData(lua.create_userdata(err)?),
        ])),
    }
}

/// Builds `pairee.image` for a plugin with `policy`.
pub fn bind(lua: &Lua, policy: Arc<FsPolicy>) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    let p = Arc::clone(&policy);
    table.set(
        "show",
        lua.create_function(move |lua, value: Value| {
            let result =
                showable(lua, &p, &value, true).map(|path| PluginImage { path, image: None });
            LuaError::pair(lua, result)
        })?,
    )?;
    let p = Arc::clone(&policy);
    table.set(
        "info",
        lua.create_function(move |lua, value: Value| {
            let result = showable(lua, &p, &value, true)
                .and_then(|path| super::fs::blocking(|| info(lua, &path)));
            LuaError::pair(lua, result)
        })?,
    )?;
    table.set(
        "precache",
        lua.create_function(
            move |lua, (src, dst, opts): (Value, Value, Option<Table>)| {
                let bound = |key: &str| -> mlua::Result<u32> {
                    Ok(match &opts {
                        Some(o) => o.get::<Option<u32>>(key)?.unwrap_or(PRECACHE_MAX_SIDE),
                        None => PRECACHE_MAX_SIDE,
                    })
                };
                let bounds = (bound("max_width")?, bound("max_height")?);
                let result = showable(lua, &policy, &src, false).and_then(|src| {
                    let dst = path_arg(&dst).map_err(|e| LuaError::custom(e.to_string()))?;
                    let dst = policy
                        .check(&dst.to_string_lossy(), Access::Write)
                        .map_err(denied)?;
                    super::fs::blocking(|| precache(&src, &dst, bounds))
                });
                outcome(lua, result)
            },
        )?,
    )?;
    Ok(table)
}

#[cfg(test)]
mod tests;
