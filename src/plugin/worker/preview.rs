//! Previewer calls: `peek`, `preload` and `seek`.
//!
//! Each gets a `job` table: `file` (a `File` userdata of the previewed
//! file, with `mime`, size and times), `area` (a `ui.Rect` at `0, 0` of
//! the preview size, with the older `width` / `height` fields too; the
//! rectangle to split and place widgets in), `skip` (lines scrolled) and,
//! for `seek`,
//! `units` (lines to scroll, negative is up).
//!
//! * `peek(job)` returns the widget to show (or `nil` to leave the
//!   built-in preview).
//! * `preload(job)` prepares a preview ahead of time (a thumbnail, a cache
//!   file) and returns `complete[, err]`. A file version (path + mtime)
//!   whose preload reported `true` is not preloaded again.
//! * `seek(job)` returns the new `skip`; the host then peeks again with it.
//!
//! While one of these runs, the previewed file is recorded as the
//! [`PreviewTarget`] of the Lua state: the preview helpers that only hand
//! the file back to the host (`pairee.image.show`, `pairee.preview_code`)
//! accept it even when it lies outside the plugin's `pairee.fs` jail.

use super::Worker;
use crate::app::state::popup::plugin_widget::Renderable;
use crate::plugin::registry::PreviewJob;
use crate::plugin::runtime::bindings::ui::to_renderable;
use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, MultiValue, Table, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// File versions whose preload completed, kept until there are too many.
#[derive(Debug, Default)]
pub struct PreloadCache(HashSet<(PathBuf, Option<u64>)>);

/// Most completed preloads remembered per plugin.
const PRELOAD_CACHE_LIMIT: usize = 4096;

impl PreloadCache {
    fn contains(&self, key: &(PathBuf, Option<u64>)) -> bool {
        self.0.contains(key)
    }

    fn insert(&mut self, key: (PathBuf, Option<u64>)) {
        if self.0.len() >= PRELOAD_CACHE_LIMIT {
            self.0.clear();
        }
        self.0.insert(key);
    }
}

/// The file a previewer method is working on (Lua app data).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewTarget(pub PathBuf);

impl PreviewTarget {
    /// Whether `path` is the file being previewed right now in `lua`.
    pub fn is_current(lua: &Lua, path: &Path) -> bool {
        lua.app_data_ref::<PreviewTarget>()
            .is_some_and(|target| target.0 == path)
    }
}

/// The `job` table of a previewer call.
fn job_table(
    lua: &Lua,
    job: &PreviewJob,
    file: LuaFile,
    units: Option<i64>,
) -> mlua::Result<Table> {
    let area = crate::app::state::popup::plugin_widget::WRect {
        x: 0,
        y: 0,
        w: job.area_width,
        h: job.area_height,
    };
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("area", area)?;
    table.set("skip", job.skip)?;
    if let Some(units) = units {
        table.set("units", units)?;
    }
    Ok(table)
}

/// Decodes the images of `widget` (the plugin only named the files; the
/// host reads at most the quick view's size cap of each).
pub fn decode_images(widget: &mut Renderable) {
    for image in widget.images_mut() {
        image.image = load_image(&image.path);
    }
}

fn load_image(path: &Path) -> Option<Arc<image::DynamicImage>> {
    use crate::app::state::quick_view::load::QUICK_VIEW_MAX_BYTES;
    use crate::fs::vfs::{LocalVfs, Vfs};
    crate::plugin::runtime::bindings::fs::blocking(|| {
        let size = std::fs::metadata(path).ok()?.len();
        if size > QUICK_VIEW_MAX_BYTES {
            return None;
        }
        let bytes = LocalVfs.read_prefix(path, QUICK_VIEW_MAX_BYTES).ok()?;
        crate::ui::viewer::decode_image(path, bytes).map(Arc::new)
    })
}

impl Worker {
    /// Calls previewer `method` on `job` with the file recorded as the
    /// preview target for the duration of the call; `finish` converts the
    /// results under the watchdog (see `call_method`).
    async fn preview_call<T>(
        &self,
        method: &str,
        job: &PreviewJob,
        units: Option<i64>,
        finish: impl FnOnce(MultiValue) -> mlua::Result<T>,
    ) -> mlua::Result<Option<T>> {
        let file = LuaFile::from_path(&job.file_path);
        let table = job_table(&self.lua, job, file, units)?;
        self.lua.set_app_data(PreviewTarget(job.file_path.clone()));
        let result = self.call_method(method, table, finish).await;
        self.lua.remove_app_data::<PreviewTarget>();
        result
    }

    pub(crate) async fn peek(&self, job: PreviewJob) -> Option<Renderable> {
        let result = self
            .preview_call("peek", &job, None, |values| {
                to_renderable(&values.into_iter().next().unwrap_or(Value::Nil))
            })
            .await;
        let mut widget = self.report("peek", result).flatten().flatten()?;
        decode_images(&mut widget);
        Some(widget)
    }

    pub(super) async fn preload(&mut self, job: PreviewJob) -> bool {
        let key = (
            job.file_path.clone(),
            LuaFile::from_path(&job.file_path).mtime,
        );
        if self.preloaded.contains(&key) {
            return true;
        }
        let result = self
            .preview_call("preload", &job, None, |values| {
                let mut values = values.into_iter();
                let complete = matches!(values.next(), Some(Value::Boolean(true)));
                let err = match values.next().filter(|v| !v.is_nil()) {
                    Some(err) => Some(err.to_string()?),
                    None => None,
                };
                Ok((complete, err))
            })
            .await;
        let Some(Some((complete, err))) = self.report("preload", result) else {
            return false;
        };
        if let Some(err) = err {
            tracing::warn!(
                "Plugin {} preload of {:?}: {err}",
                self.name,
                job.file_path,
            );
        }
        if complete {
            self.preloaded.insert(key);
        }
        complete
    }

    pub(super) async fn seek(&self, job: PreviewJob, units: i64) -> Option<usize> {
        let result = self
            .preview_call("seek", &job, Some(units), |values| {
                Ok(match values.into_iter().next() {
                    Some(Value::Integer(skip)) => Some(skip.max(0) as usize),
                    Some(Value::Number(skip)) if skip.is_finite() => Some(skip.max(0.0) as usize),
                    _ => None,
                })
            })
            .await;
        self.report("seek", result)??
    }
}
