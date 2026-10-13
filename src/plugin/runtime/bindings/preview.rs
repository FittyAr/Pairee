//! `pairee.preview_code(opts)` and `pairee.preview_widget(opts, widget)`.
//!
//! * `preview_code({file, skip?, lines?})` returns a paragraph widget with
//!   the file's text from line `skip` (0-based) on, `lines` lines at most
//!   (500 by default, 5000 at most), behind a line-number gutter, tabs
//!   expanded; `nil, Error` for binary or unreadable files. It reads at
//!   most the quick view's text cap. There is no syntax highlighting.
//! * `preview_widget({file?, skip?}, widget)` shows `widget` in the quick
//!   view now (for previews that arrive after `peek` returned, e.g. from a
//!   child process); `file` defaults to the file being previewed. The host
//!   only applies it while the quick view still shows that file. Returns
//!   `true` or `false, Error`.
//!
//! Sandbox: `preview_code` reads the file being previewed or what the read
//! jail allows, like `pairee.image.show`; the widget it returns has no
//! getters, so the plugin cannot read the text back.

use super::fs::FsPolicy;
use super::image::showable;
use super::ui::convert::renderable_arg;
use crate::app::state::popup::PluginPreview;
use crate::app::state::popup::plugin_widget::{WLine, WParagraph, WSpan, WStyle, WText, WWrap};
use crate::app::state::quick_view::load::QUICK_VIEW_TEXT_BYTES;
use crate::plugin::manager::PluginRequest;
use crate::plugin::runtime::types::LuaError;
use crate::plugin::worker::PreviewTarget;
use mlua::{Lua, MultiValue, Table, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Lines `preview_code` returns by default and at most.
pub const CODE_LINES: usize = 500;
pub const MAX_CODE_LINES: usize = 5000;
const TAB: &str = "    ";

/// The start of `path` as text, or an error for binary content.
fn read_text(path: &Path) -> Result<String, LuaError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(QUICK_VIEW_TEXT_BYTES)
        .read_to_end(&mut bytes)?;
    if bytes.iter().take(8192).any(|&b| b == 0) {
        return Err(LuaError {
            kind: Some("InvalidData".into()),
            code: None,
            message: format!("pairee.preview_code: {} is not text", path.display()),
        });
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Lines `skip..skip + count` of `text` behind a dim line-number gutter.
pub fn code_text(text: &str, skip: usize, count: usize) -> WText {
    let total = text.lines().count();
    let width = total.max(1).to_string().len();
    let gutter = WStyle {
        fg: Some("darkgray".into()),
        ..WStyle::default()
    };
    let lines = text
        .lines()
        .enumerate()
        .skip(skip)
        .take(count)
        .map(|(i, line)| {
            let content: String = line
                .replace('\t', TAB)
                .chars()
                .filter(|c| !c.is_control())
                .collect();
            WLine {
                spans: vec![
                    WSpan {
                        content: format!("{:>width$} │ ", i + 1),
                        style: gutter.clone(),
                    },
                    WSpan {
                        content,
                        style: WStyle::default(),
                    },
                ],
                ..WLine::default()
            }
        })
        .collect();
    WText::lines(lines)
}

fn preview_code(lua: &Lua, policy: &FsPolicy, opts: &Table) -> Result<WParagraph, LuaError> {
    let file: Value = opts
        .get("file")
        .map_err(|e| LuaError::custom(e.to_string()))?;
    let path = showable(lua, policy, &file, true)?;
    let skip: usize = opts
        .get::<Option<usize>>("skip")
        .ok()
        .flatten()
        .unwrap_or(0);
    let count = opts
        .get::<Option<usize>>("lines")
        .ok()
        .flatten()
        .unwrap_or(CODE_LINES)
        .min(MAX_CODE_LINES);
    let text = super::fs::blocking(|| read_text(&path))?;
    Ok(WParagraph {
        text: code_text(&text, skip, count),
        wrap: WWrap::No,
        scroll: 0,
        block: None,
        style: WStyle::default(),
    })
}

/// The quick-view update `preview_widget` sends.
fn widget_update(
    lua: &Lua,
    plugin: &str,
    opts: &Table,
    widget: &Value,
) -> mlua::Result<PluginRequest> {
    let file: Value = opts.get("file")?;
    let path: PathBuf = if file.is_nil() {
        lua.app_data_ref::<PreviewTarget>()
            .map(|t| t.0.clone())
            .ok_or_else(|| {
                mlua::Error::RuntimeError(
                    "pairee.preview_widget: no file is being previewed; pass opts.file".into(),
                )
            })?
    } else {
        crate::plugin::runtime::types::path::path_arg(&file)?
    };
    let mut widget = renderable_arg(widget)?;
    crate::plugin::worker::decode_images(&mut widget);
    let preview = PluginPreview {
        widget,
        plugin: plugin.to_string(),
        skip: opts.get::<Option<usize>>("skip")?.unwrap_or(0),
        seekable: false,
    };
    Ok(PluginRequest::UpdatePluginWidget {
        path,
        preview: Box::new(preview),
    })
}

/// Adds `preview_code` and `preview_widget` to `pairee`.
pub fn bind(
    lua: &Lua,
    pairee: &Table,
    (plugin, policy): (&str, Arc<FsPolicy>),
    tx: mpsc::Sender<PluginRequest>,
) -> mlua::Result<()> {
    pairee.set(
        "preview_code",
        lua.create_function(move |lua, opts: Table| {
            LuaError::pair(lua, preview_code(lua, &policy, &opts))
        })?,
    )?;
    let plugin = plugin.to_string();
    pairee.set(
        "preview_widget",
        lua.create_async_function(move |lua, (opts, widget): (Table, Value)| {
            let tx = tx.clone();
            let request = widget_update(&lua, &plugin, &opts, &widget);
            async move {
                let sent = match request {
                    Ok(req) => tx
                        .send(req)
                        .await
                        .map_err(|_| LuaError::custom("pairee.preview_widget: Pairee is closing")),
                    Err(e) => Err(LuaError::custom(e.to_string())),
                };
                Ok::<_, mlua::Error>(match sent {
                    Ok(()) => MultiValue::from_vec(vec![Value::Boolean(true), Value::Nil]),
                    Err(err) => MultiValue::from_vec(vec![
                        Value::Boolean(false),
                        Value::UserData(lua.create_userdata(err)?),
                    ]),
                })
            }
        })?,
    )
}

#[cfg(test)]
mod tests;
