//! Previewer calls: `peek`, `preload` and `seek`.
//!
//! Each gets a `job` table: `file` (a `File` userdata of the previewed
//! file, with `mime`, size and times), `area` (`x`, `y`, `w`, `h`, and the
//! older `width` / `height`), `skip` (lines scrolled) and, for `seek`,
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
use crate::app::state::types::PluginWidget;
use crate::plugin::registry::PreviewJob;
use crate::plugin::runtime::types::LuaFile;
use mlua::{Lua, LuaSerdeExt, MultiValue, Table, Value};
use std::collections::HashSet;
use std::path::PathBuf;

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

/// The `job` table of a previewer call.
fn job_table(
    lua: &Lua,
    job: &PreviewJob,
    file: LuaFile,
    units: Option<i64>,
) -> mlua::Result<Table> {
    let area = lua.create_table()?;
    area.set("x", 0)?;
    area.set("y", 0)?;
    area.set("w", job.area_width)?;
    area.set("h", job.area_height)?;
    area.set("width", job.area_width)?;
    area.set("height", job.area_height)?;
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("area", area)?;
    table.set("skip", job.skip)?;
    if let Some(units) = units {
        table.set("units", units)?;
    }
    Ok(table)
}

/// Converts what `peek` returned into a widget (`None` for `nil`).
fn to_widget(lua: &Lua, value: Value) -> mlua::Result<Option<PluginWidget>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

impl Worker {
    /// Calls previewer `method` on `job` with the file recorded as the
    /// preview target for the duration of the call.
    async fn preview_call(
        &self,
        method: &str,
        job: &PreviewJob,
        units: Option<i64>,
    ) -> mlua::Result<Option<MultiValue>> {
        let file = LuaFile::from_path(&job.file_path);
        let table = job_table(&self.lua, job, file, units)?;
        self.lua.set_app_data(PreviewTarget(job.file_path.clone()));
        let result = self.call_method(method, table).await;
        self.lua.remove_app_data::<PreviewTarget>();
        result
    }

    pub(super) async fn peek(&self, job: PreviewJob) -> Option<PluginWidget> {
        let result = async {
            let Some(values) = self.preview_call("peek", &job, None).await? else {
                return Ok(None);
            };
            to_widget(&self.lua, values.into_iter().next().unwrap_or(Value::Nil))
        }
        .await;
        self.report("peek", result).flatten()
    }

    pub(super) async fn preload(&mut self, job: PreviewJob) -> bool {
        let key = (
            job.file_path.clone(),
            LuaFile::from_path(&job.file_path).mtime,
        );
        if self.preloaded.contains(&key) {
            return true;
        }
        let result = self.preview_call("preload", &job, None).await;
        let Some(Some(values)) = self.report("preload", result) else {
            return false;
        };
        let mut values = values.into_iter();
        let complete = matches!(values.next(), Some(Value::Boolean(true)));
        if let Some(err) = values.next().filter(|v| !v.is_nil()) {
            tracing::warn!(
                "Plugin {} preload of {:?}: {}",
                self.name,
                job.file_path,
                err.to_string().unwrap_or_default()
            );
        }
        if complete {
            self.preloaded.insert(key);
        }
        complete
    }

    pub(super) async fn seek(&self, job: PreviewJob, units: i64) -> Option<usize> {
        let result = self.preview_call("seek", &job, Some(units)).await;
        let values = self.report("seek", result)??;
        match values.into_iter().next() {
            Some(Value::Integer(skip)) => Some(skip.max(0) as usize),
            Some(Value::Number(skip)) if skip.is_finite() => Some(skip.max(0.0) as usize),
            _ => None,
        }
    }
}
