//! Internal viewer (F3): background open, background search and encoding
//! selection for the viewer screens.

use super::AppState;
use super::types::Screen;
use crate::app::jobs::JobSlot;
use crate::config::localization::t;
use crate::config::settings::Settings;
use crate::fs::text::{EncodingPolicy, encoding_by_name};
use crate::ui::viewer::{ViewerMode, ViewerState};
use std::path::{Path, PathBuf};

/// Outcome of a background viewer search.
#[derive(Debug)]
pub struct SearchHit {
    path: PathBuf,
    query: String,
    line: Option<u64>,
}

/// Background work of the viewer screens.
#[derive(Debug, Default)]
pub struct ViewerJobs {
    /// Opening the file (encoding detection, image decoding).
    load: JobSlot<ViewerState>,
    /// The running search, with its progress in percent.
    search: JobSlot<SearchHit, u8>,
    /// Status values last painted, to repaint only when they change.
    seen: (Option<u8>, u64, Option<u8>),
}

impl ViewerJobs {
    /// Progress of the running search, if any.
    pub fn search_progress(&self) -> Option<u8> {
        self.search
            .is_running()
            .then(|| self.search.progress().unwrap_or(0))
    }
}

/// Encoding choice for newly opened files, from the viewer settings.
pub fn encoding_policy(settings: &Settings) -> EncodingPolicy {
    if settings.viewer_autodetect_codepage {
        return EncodingPolicy::Detect;
    }
    EncodingPolicy::Fixed(
        encoding_by_name(&settings.viewer_default_codepage).unwrap_or(encoding_rs::UTF_8),
    )
}

impl AppState {
    /// Opens the internal viewer on `path`. The file is opened by a
    /// background job; a "Loading…" viewer is shown meanwhile.
    pub fn open_viewer(&mut self, path: PathBuf, settings: &Settings, hex: bool) {
        self.push_screen(Screen::Viewer(ViewerState::loading(path.clone())));
        let allow_image = settings.image_preview_enabled;
        let policy = encoding_policy(settings);
        self.viewer.load.start(move |_| {
            let mut viewer = ViewerState::load_with_images(path, allow_image, policy);
            if hex {
                viewer.mode = ViewerMode::Hex;
            }
            viewer
        });
        self.poll_viewer();
    }

    /// The viewer on the active screen.
    pub fn active_viewer_mut(&mut self) -> Option<&mut ViewerState> {
        match self.screens.get_mut(self.active_screen_idx) {
            Some(Screen::Viewer(vw)) => Some(vw),
            _ => None,
        }
    }

    /// First viewer screen showing `path` (`loading` placeholders only when asked).
    fn viewer_for(&mut self, path: &Path, loading: bool) -> Option<&mut ViewerState> {
        self.screens.iter_mut().find_map(|s| match s {
            Screen::Viewer(v) if v.path == path && (!loading || v.loading) => Some(v),
            _ => None,
        })
    }

    /// Applies finished viewer jobs and repaints when the status line
    /// (indexing or search progress) changed. Returns `true` on a change.
    pub fn poll_viewer(&mut self) -> bool {
        let mut changed = false;
        if let Some(loaded) = self.viewer.load.poll()
            && let Some(slot) = self.viewer_for(&loaded.path.clone(), true)
        {
            *slot = loaded;
            changed = true;
        }
        if let Some(hit) = self.viewer.search.poll() {
            apply_search_hit(self, hit);
            changed = true;
        }
        let search = self.viewer.search_progress();
        let seen = match self.screens.get(self.active_screen_idx) {
            Some(Screen::Viewer(vw)) => (vw.doc.index_progress(), vw.doc.line_count(), search),
            _ => (None, 0, search),
        };
        if seen != self.viewer.seen {
            self.viewer.seen = seen;
            changed = true;
        }
        if changed {
            self.mark_ui_dirty();
        }
        changed
    }

    /// Searches the active viewer for `query` in the background, from the
    /// current line (the line after it when repeating the last search),
    /// wrapping to the top.
    pub fn search_viewer(&mut self, query: String, case_sensitive: bool) {
        if query.is_empty() {
            return;
        }
        let Some(vw) = self.active_viewer_mut() else {
            return;
        };
        let is_repeat = vw.last_search.as_ref() == Some(&query);
        let from = if is_repeat { vw.scroll + 1 } else { vw.scroll };
        vw.last_search = Some(query.clone());
        vw.last_case_sensitive = case_sensitive;
        vw.notice = None;
        if vw.mode != ViewerMode::Text {
            return;
        }
        let Some(job) = vw.doc.search(from as u64, &query, case_sensitive) else {
            vw.notice = Some(t("viewer_search_wait"));
            return;
        };
        let path = vw.path.clone();
        self.viewer.search.start(move |ctx| {
            let line = job.run(&|| ctx.is_cancelled(), &|pct| ctx.report(pct));
            SearchHit { path, query, line }
        });
        self.poll_viewer();
    }

    /// Repeats the active viewer's last search (F3).
    pub fn repeat_viewer_search(&mut self) {
        let Some(vw) = self.active_viewer_mut() else {
            return;
        };
        if let Some(query) = vw.last_search.clone() {
            let case_sensitive = vw.last_case_sensitive;
            self.search_viewer(query, case_sensitive);
        }
    }

    /// Stops a running viewer search. Returns `false` when none was running.
    pub fn cancel_viewer_search(&mut self) -> bool {
        if !self.viewer.search.is_running() {
            return false;
        }
        self.viewer.search.cancel();
        self.mark_ui_dirty();
        true
    }
}

fn apply_search_hit(state: &mut AppState, hit: SearchHit) {
    let Some(vw) = state.viewer_for(&hit.path, false) else {
        return;
    };
    match hit.line {
        Some(line) => vw.scroll = line as usize,
        None => vw.notice = Some(t("viewer_search_not_found").replacen("{}", &hit.query, 1)),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::{AppState, Screen};
    use crate::config::settings::Settings;

    fn open(text: &str, hex: bool) -> AppState {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.txt");
        std::fs::write(&path, text).unwrap();
        let mut state = AppState::new(dir.path().into(), dir.path().into());
        state.open_viewer(path, &Settings::default(), hex);
        state
    }

    #[test]
    fn open_viewer_replaces_loading_placeholder() {
        let mut state = open("hello", true);
        let vw = state.active_viewer_mut().expect("viewer screen");
        assert!(!vw.loading);
        assert_eq!(vw.doc.lines(0, 5), ["hello"]);
        assert_eq!(vw.mode, crate::ui::viewer::ViewerMode::Hex);
        assert!(matches!(state.screens.last(), Some(Screen::Viewer(_))));
    }

    #[test]
    fn search_scrolls_repeats_and_reports_not_found() {
        let mut state = open("alpha\nbeta\ngamma\nbeta again\n", false);
        state.search_viewer("BETA".into(), false);
        assert_eq!(state.active_viewer_mut().unwrap().scroll, 1);
        state.repeat_viewer_search();
        assert_eq!(state.active_viewer_mut().unwrap().scroll, 3);
        state.repeat_viewer_search();
        assert_eq!(state.active_viewer_mut().unwrap().scroll, 1, "wraps");
        state.search_viewer("zeta".into(), false);
        let vw = state.active_viewer_mut().unwrap();
        assert_eq!(vw.scroll, 1);
        assert!(vw.notice.as_deref().unwrap_or_default().contains("zeta"));
        assert!(!state.cancel_viewer_search());
    }
}
