//! Native change notifications through `notify` (recursive for a branch
//! view, which lists every file below its folder).

use super::monitor::{ChangeSink, DirChange};
use super::strategy::ChangeStrategy;
use notify::event::{EventKind, MetadataKind, ModifyKind};
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::Receiver;

pub struct WatchStrategy {
    pub recursive: bool,
}

impl ChangeStrategy for WatchStrategy {
    fn run(&self, dir: &Path, stop: &Receiver<()>, sink: &ChangeSink) -> Result<(), String> {
        let owned = dir.to_path_buf();
        let events = sink.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(event) if is_change(&event.kind) => events.report(DirChange {
                    dir: owned.clone(),
                    entries: event.paths,
                    origin: Default::default(),
                }),
                Ok(_) => {}
                // Lost events (queue overflow, …): reread the whole folder.
                Err(_) => events.report(DirChange::whole(owned.clone())),
            }
        })
        .map_err(|e| e.to_string())?;
        let mode = if self.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher.watch(dir, mode).map_err(|e| e.to_string())?;
        sink.armed(dir);
        // Blocks until the monitor is dropped; the watcher stops with it.
        let _ = stop.recv();
        Ok(())
    }
}

/// Reads (and access-time updates) do not change a listing.
pub(super) fn is_change(kind: &EventKind) -> bool {
    !matches!(
        kind,
        EventKind::Access(_) | EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime))
    )
}
