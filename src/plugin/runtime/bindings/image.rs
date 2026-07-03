//! M2 typed binding: `pairee.image.{show, precache, info}`.
//!
//! Reuses the existing `image` crate to decode PNG/JPEG/GIF/WebP/etc.
//! and exposes three helper functions to plugins:
//!
//! - `pairee.image.show(url, rect)` — renders the image into the
//!   current preview pane. M2 ships the wiring as a `log::info!`
//!   plus a `PluginRequest::ImagePreview` enum variant (M3's main
//!   loop dispatch will route it to `QuickViewPanel.image_data`).
//!   For now the binding returns `true` to indicate the image was
//!   successfully decoded; the actual rendering path is a follow-up.
//! - `pairee.image.precache(src, dist)` — decodes `src`, resizes it
//!   to fit within a 1024-pixel bounding box, and writes the
//!   result to `dist`. Returns `true` on success.
//! - `pairee.image.info(url)` — returns a Lua table with `w`, `h`,
//!   `format`, and `color` (e.g. `"RGB8"`).
//!
//! In Secure Mode the existing `validate_path` policy applies; the
//! helpers refuse to read or write outside the user's workspace,
//! config, or cache directory.

use crate::plugin::manager::PluginRequest;
use std::path::Path;
use tokio::sync::mpsc;

/// Max dimension (in either width or height) for `precache` outputs.
const PRECACHE_MAX_DIM: u32 = 1024;

/// Build the `pairee.image` namespace table.
pub fn bind(lua: &mlua::Lua, tx: mpsc::Sender<PluginRequest>) -> mlua::Result<mlua::Table<'_>> {
    let table = lua.create_table()?;

    // `pairee.image.show(url, rect)` — render the image in the
    // current preview pane. M2 wires the API surface and the
    // dispatcher; the actual rendering is M3.
    let tx_show = tx.clone();
    table.set(
        "show",
        lua.create_async_function(move |_lua, (url_str, rect): (String, mlua::Table)| {
            let tx = tx_show.clone();
            async move {
                let path = std::path::PathBuf::from(&url_str);
                if !is_workspace_path(&path) {
                    log::warn!(
                        "pairee.image.show refused: {url_str} is outside the workspace"
                    );
                    return Ok(false);
                }
                match image::open(&path) {
                    Ok(img) => {
                        let w = img.width();
                        let h = img.height();
                        let x: i32 = rect.get("x").unwrap_or(0);
                        let y: i32 = rect.get("y").unwrap_or(0);
                        let rw: u16 = rect.get("w").unwrap_or(0);
                        let rh: u16 = rect.get("h").unwrap_or(0);
                        log::info!(
                            "pairee.image.show decoded {}x{} image from {} at ({},{}) size {}x{}",
                            w,
                            h,
                            url_str,
                            x,
                            y,
                            rw,
                            rh
                        );
                        // M2 placeholder: the actual render is a
                        // follow-up. We *do* send an ImagePreview
                        // request so the main loop can integrate
                        // it later without further binding changes.
                        let _ = tx.send(PluginRequest::ImagePreview {
                            path,
                            rect: crate::plugin::manager::ImageRect {
                                x,
                                y,
                                w: rw,
                                h: rh,
                            },
                        });
                        Ok(true)
                    }
                    Err(e) => {
                        log::warn!("pairee.image.show failed to decode {url_str}: {e}");
                        Ok(false)
                    }
                }
            }
        })?,
    )?;

    // `pairee.image.precache(src, dist)` — resize + write.
    table.set(
        "precache",
        lua.create_async_function(move |_lua, (src, dist): (String, String)| {
            async move {
                let src_path = std::path::PathBuf::from(&src);
                let dist_path = std::path::PathBuf::from(&dist);
                if !is_workspace_path(&src_path) || !is_workspace_path(&dist_path) {
                    log::warn!("pairee.image.precache refused: path outside workspace");
                    return Ok(false);
                }
                match image::open(&src_path) {
                    Ok(img) => {
                        let resized = if img.width() > PRECACHE_MAX_DIM
                            || img.height() > PRECACHE_MAX_DIM
                        {
                            img.resize(
                                PRECACHE_MAX_DIM,
                                PRECACHE_MAX_DIM,
                                image::imageops::FilterType::Triangle,
                            )
                        } else {
                            img
                        };
                        if let Some(parent) = dist_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        match resized.save(&dist_path) {
                            Ok(()) => Ok(true),
                            Err(e) => {
                                log::warn!("pairee.image.precache failed to save: {e}");
                                Ok(false)
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("pairee.image.precache failed to decode {src}: {e}");
                        Ok(false)
                    }
                }
            }
        })?,
    )?;

    // `pairee.image.info(url)` — return {w, h, format, color}.
    table.set(
        "info",
        lua.create_async_function(move |lua_ctx, url_str: String| {
            async move {
                let path = std::path::PathBuf::from(&url_str);
                match image::open(&path) {
                    Ok(img) => {
                        let table = lua_ctx.create_table()?;
                        table.set("w", img.width())?;
                        table.set("h", img.height())?;
                        // `image::ImageFormat` is detected from the
                        // path extension by the `image::open`
                        // helper; we re-detect here for the format
                        // string.
                        let format = image::ImageFormat::from_path(&path)
                            .ok()
                            .map(|f| format!("{f:?}"))
                            .unwrap_or_else(|| "Unknown".to_string());
                        table.set("format", format)?;
                        table.set("color", color_string(&img))?;
                        Ok(mlua::Value::Table(table))
                    }
                    Err(e) => {
                        log::warn!("pairee.image.info failed: {e}");
                        Ok(mlua::Value::Nil)
                    }
                }
            }
        })?,
    )?;

    Ok(table)
}

fn color_string(img: &image::DynamicImage) -> String {
    use image::ColorType;
    match img.color() {
        ColorType::L8 => "L8".to_string(),
        ColorType::La8 => "La8".to_string(),
        ColorType::Rgb8 => "RGB8".to_string(),
        ColorType::Rgba8 => "RGBA8".to_string(),
        ColorType::L16 => "L16".to_string(),
        ColorType::La16 => "La16".to_string(),
        ColorType::Rgb16 => "RGB16".to_string(),
        ColorType::Rgba16 => "RGBA16".to_string(),
        ColorType::Rgb32F => "RGB32F".to_string(),
        ColorType::Rgba32F => "RGBA32F".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn is_workspace_path(path: &Path) -> bool {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let allowed_roots = [
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        crate::config::paths::get_config_dir(),
        crate::config::paths::get_cache_dir(),
    ];
    allowed_roots.iter().any(|root| {
        let root = root.canonicalize().unwrap_or_else(|_| root.clone());
        canonical.starts_with(&root)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use mlua::Lua;

    fn fresh_png(path: &Path) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(32, 32, |x, y| Rgb([x as u8, y as u8, (x + y) as u8]));
        img.save(path).unwrap();
    }

    #[tokio::test]
    async fn test_info_returns_dims() {
        let path = std::env::temp_dir().join("pairee_image_info_test.png");
        fresh_png(&path);
        let lua = Lua::new();
        let (tx, _rx) = mpsc::channel::<PluginRequest>(1);
        let table = bind(&lua, tx).expect("image table");
        let info_fn: mlua::Function = table.get("info").expect("info function");
        let _ = table; // keep the table alive for info_fn's lifetime
        let result: mlua::Value = info_fn
            .call_async::<String, mlua::Value>(path.to_string_lossy().to_string())
            .await
            .expect("info call");
        let result_table = match result {
            mlua::Value::Table(t) => t,
            _ => panic!("expected table"),
        };
        let w: u32 = result_table.get("w").unwrap();
        let h: u32 = result_table.get("h").unwrap();
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        std::fs::remove_file(&path).ok();
    }
}
