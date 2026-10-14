//! Reloading the user's syntax definitions (`<config>/syntax/*.toml`): on
//! request (the `reload_syntax_definitions` action) or when a definition
//! file changes, noticed by a folder monitor of the auto-refresh
//! machinery. Open editors and viewers are highlighted again; an invalid
//! file is reported and keeps the language it defined before.

use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::config::settings::Settings;
use crate::fs::watch::{ChangeSink, CoalesceTiming, Coalescer, DirChange, DirMonitor, MonitorPlan};
use crate::syntax::SyntaxLimits;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// How often a missing definitions folder is checked for.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// The monitor of the definitions folder.
pub struct SyntaxWatch {
    monitor: Option<DirMonitor>,
    /// The folder existed when the monitor started (a missing one is
    /// polled until it appears, then watched).
    existed: bool,
    coalescer: Coalescer,
    tx: Sender<DirChange>,
    rx: Receiver<DirChange>,
}

impl Default for SyntaxWatch {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            monitor: None,
            existed: false,
            coalescer: Coalescer::new(CoalesceTiming::default()),
            tx,
            rx,
        }
    }
}

impl SyntaxWatch {
    /// Monitors `dir`: started on the first call, and again once a folder
    /// that did not exist appears (to watch it instead of polling).
    fn arm(&mut self, dir: &Path) {
        if self.monitor.is_some() && (self.existed || !dir.is_dir()) {
            return;
        }
        self.existed = dir.is_dir();
        let plan = MonitorPlan {
            prefer_poll: false,
            poll_interval: POLL_INTERVAL,
            recursive: false,
        };
        let sink = ChangeSink::new(self.tx.clone(), crate::app::jobs::wake_event_loop);
        self.monitor = Some(DirMonitor::start(dir.to_path_buf(), plan, sink));
    }

    /// Whether a definition file changed and the changes settled by `now`.
    fn take_due(&mut self, now: Instant) -> bool {
        while let Ok(change) = self.rx.try_recv() {
            if touches_definitions(&change) {
                self.coalescer.record(change, now);
            }
        }
        !self.coalescer.take_due(now).is_empty()
    }
}

/// Whether `change` may concern a definition file (editors' swap and
/// backup files do not).
fn touches_definitions(change: &DirChange) -> bool {
    change.entries.is_empty()
        || change.entries.iter().any(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
        })
}

impl AppState {
    /// Watches the definitions folder and reloads it once a change settles.
    /// Called once per event-loop pass.
    pub fn poll_syntax_definitions(&mut self, settings: &Settings) {
        self.syntax_watch
            .arm(&crate::config::paths::get_syntax_dir());
        if self.syntax_watch.take_due(Instant::now()) {
            reload_syntax_definitions(self, settings, false);
        }
    }
}

/// Reads the user's definitions again and highlights the open editors and
/// viewers with them. Invalid files are reported; `announce` also confirms
/// a reload without errors (the explicit action).
pub fn reload_syntax_definitions(state: &mut AppState, settings: &Settings, announce: bool) {
    let errors = crate::config::load_syntax_definitions();
    state
        .screens
        .apply_syntax_limits(&SyntaxLimits::from_settings(settings));
    if !errors.is_empty() {
        state.dialogs.push(PopupType::error(errors.join("\n\n")));
    } else if announce {
        state
            .dialogs
            .push(PopupType::info(t("syntax_definitions_reloaded")));
    }
    state.mark_ui_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Screen;
    use crate::ui::viewer::ViewerState;
    use std::path::PathBuf;

    fn highlighted(state: &AppState) -> Option<String> {
        match state.screens.last() {
            Some(Screen::Viewer(vw)) => vw.syntax.borrow().as_ref().map(|c| c.language().into()),
            _ => None,
        }
    }

    #[test]
    fn reloads_rehighlight_open_screens_and_report_errors() {
        let root = tempfile::tempdir().unwrap();
        let _redirect = crate::config::paths::test_root::redirect(root.path());
        let dir = crate::config::paths::get_syntax_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("reload-notes.toml");
        let settings = Settings {
            syntax_highlighting: true,
            ..Settings::default()
        };
        let mut state = AppState::new(".".into(), ".".into());
        let doc = vec!["TODO".to_string()];
        state.push_screen(Screen::Viewer(ViewerState::from_text(
            PathBuf::from("a.reloadnotes"),
            doc,
        )));
        std::fs::write(
            &file,
            "name = \"ReloadNotes\"\nextensions = [\"reloadnotes\"]",
        )
        .unwrap();
        reload_syntax_definitions(&mut state, &settings, true);
        assert_eq!(highlighted(&state).as_deref(), Some("ReloadNotes"));
        assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
        state.dialogs.clear();

        std::fs::write(&file, "name = ").unwrap();
        reload_syntax_definitions(&mut state, &settings, true);
        assert!(
            matches!(state.dialogs.top(), Some(PopupType::Error(_))),
            "{:?}",
            state.dialogs.top()
        );
        assert_eq!(highlighted(&state).as_deref(), Some("ReloadNotes"), "kept");

        std::fs::remove_file(&file).unwrap();
        reload_syntax_definitions(&mut state, &settings, false);
        assert_eq!(highlighted(&state), None, "gone with its file");
    }

    #[test]
    fn only_definition_files_count_as_changes() {
        let change = |entries: &[&str]| DirChange {
            entries: entries.iter().map(PathBuf::from).collect(),
            ..DirChange::whole(PathBuf::from("syntax"))
        };
        assert!(touches_definitions(&change(&[])));
        assert!(touches_definitions(&change(&[
            "syntax/a.TOML",
            "syntax/a.swp"
        ])));
        assert!(!touches_definitions(&change(&[
            "syntax/a.toml~",
            "syntax/.a.swp"
        ])));
    }

    #[test]
    fn a_changed_definition_file_is_noticed() {
        let dir = tempfile::tempdir().unwrap();
        let mut watch = SyntaxWatch::default();
        watch.arm(dir.path());
        assert!(watch.existed);
        std::fs::write(dir.path().join("a.toml"), "name = \"A\"").unwrap();
        // Noticed by the watch, or (if it started after the write) by the
        // monitor's check of the folder time when it starts.
        let deadline = Instant::now() + Duration::from_secs(20);
        while !watch.take_due(Instant::now()) {
            assert!(Instant::now() < deadline, "no change noticed");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
