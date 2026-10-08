//! End-to-end test harness for the TUI (test builds only).
//!
//! A [`Harness`] runs the real application state and context over a private
//! temporary sandbox: panels start in `<sandbox>/work/left` and
//! `<sandbox>/work/right`, and every per-user directory (config, cache,
//! session, hotlist) is redirected to `<sandbox>/home` for the harness
//! thread, so a test never touches the real user configuration.
//!
//! Input goes through the same dispatcher as the event loop (popups,
//! screens, CLI, keymap), background jobs run on a private Tokio runtime and
//! are drained by [`Harness::settle`] / [`Harness::wait_until`], and frames
//! are painted on a `TestBackend` for assertions on the screen text.

mod drive;
mod keys;
mod screen;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::AppConfig;
use crate::config::paths::test_root;
use crate::config::settings::Settings;
use crate::terminal::TerminalBackend;
use ratatui::{Terminal, backend::TestBackend};
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Options a scenario may change before the harness starts.
pub struct Builder {
    keymap: String,
    size: (u16, u16),
    settings: Settings,
    sandbox: Option<TempDir>,
    cli_folders: bool,
}

impl Builder {
    /// Starts like a plain `pairee` (no command-line folders), so the
    /// restored session alone decides what the panels show.
    pub fn without_cli_folders(mut self) -> Self {
        self.cli_folders = false;
        self
    }

    /// Keymap preset (`norton`, `vscode`, `neovim`).
    pub fn keymap(mut self, preset: &str) -> Self {
        self.keymap = preset.to_string();
        self
    }

    /// Terminal size of the rendered frames.
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.size = (width, height);
        self
    }

    /// Changes the settings the harness starts with.
    pub fn settings(mut self, change: impl FnOnce(&mut Settings)) -> Self {
        change(&mut self.settings);
        self
    }

    pub fn build(self) -> Harness {
        Harness::start(self)
    }
}

/// A running application over a temporary sandbox.
pub struct Harness {
    pub state: AppState,
    pub ctx: AppContext,
    rt: tokio::runtime::Runtime,
    backend: ManuallyDrop<TerminalBackend>,
    screen: Terminal<TestBackend>,
    keymap: String,
    // Field order matters: the redirect is restored before the sandbox goes.
    _redirect: test_root::Redirect,
    sandbox: TempDir,
}

impl Harness {
    /// Norton keymap, 100x30 terminal, default settings.
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn builder() -> Builder {
        let settings = Settings {
            onboarding_completed: true,
            auto_update_check: false,
            plugins_enabled: false,
            ..Settings::default()
        };
        Builder {
            keymap: "norton".into(),
            size: (100, 30),
            settings,
            sandbox: None,
            cli_folders: true,
        }
    }

    fn start(options: Builder) -> Self {
        let sandbox = options
            .sandbox
            .unwrap_or_else(|| tempfile::tempdir().expect("sandbox"));
        let redirect = test_root::redirect(&sandbox.path().join("home"));
        let (left, right) = (
            sandbox.path().join("work/left"),
            sandbox.path().join("work/right"),
        );
        for dir in [&left, &right, &crate::config::paths::get_config_dir()] {
            std::fs::create_dir_all(dir).expect("sandbox dirs");
        }
        let mut config = AppConfig {
            settings: options.settings,
            ..AppConfig::default()
        };
        config.keybindings.preset = options.keymap.clone();
        let mut ctx = AppContext::new(config);
        let cli_folders = if options.cli_folders {
            vec![left, right]
        } else {
            Vec::new()
        };
        let mut state = crate::run::initial_state(&ctx.config.settings, &cli_folders);
        crate::app::app::prepare_first_frame(&mut state, &mut ctx);
        // Editor copy / paste must not replace the user's clipboard.
        state.editor_clipboard = crate::app::editor::EditorClipboard::internal_only();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime");
        let (width, height) = options.size;
        let mut harness = Self {
            state,
            ctx,
            rt,
            backend: ManuallyDrop::new(TerminalBackend::headless().expect("headless backend")),
            screen: Terminal::new(TestBackend::new(width, height)).expect("test terminal"),
            keymap: options.keymap,
            _redirect: redirect,
            sandbox,
        };
        harness.settle();
        harness
    }

    /// Quits like `F10` would (session, history, setup saved to the
    /// sandbox) and starts a new harness on the same sandbox.
    pub fn restart(self, change: impl FnOnce(Builder) -> Builder) -> Self {
        let mut this = self;
        this.quit();
        let size = this.screen.backend().buffer().area;
        let mut builder = Self::builder()
            .keymap(&this.keymap)
            .size(size.width, size.height);
        builder.settings = this.ctx.config.settings.clone();
        let mut builder = change(builder);
        let Harness {
            sandbox, _redirect, ..
        } = this;
        drop(_redirect);
        builder.sandbox = Some(sandbox);
        builder.build()
    }

    /// Persists what outlives the run (as the event loop does on quit) and
    /// returns the folder a shell wrapper would change to.
    pub fn quit(&mut self) -> Option<PathBuf> {
        self.state.should_quit = true;
        crate::app::session::persist_on_exit(&self.state, &mut self.ctx)
    }

    /// The sandbox root.
    pub fn root(&self) -> &Path {
        self.sandbox.path()
    }

    /// Initial folder of the left panel.
    pub fn left(&self) -> PathBuf {
        self.root().join("work/left")
    }

    /// Initial folder of the right panel.
    pub fn right(&self) -> PathBuf {
        self.root().join("work/right")
    }

    /// The redirected configuration directory.
    pub fn config_dir(&self) -> PathBuf {
        crate::config::paths::get_config_dir()
    }

    /// Writes `contents` to `path` (relative to the sandbox root), creating
    /// the parent folders.
    pub fn write(&self, path: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let full = self.root().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("parent dirs");
        }
        std::fs::write(&full, contents).expect("write fixture");
        full
    }

    /// Creates the folder `path` (relative to the sandbox root).
    pub fn mkdir(&self, path: &str) -> PathBuf {
        let full = self.root().join(path);
        std::fs::create_dir_all(&full).expect("mkdir fixture");
        full
    }

    /// Reads `path` (relative to the sandbox root) as bytes.
    pub fn read(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.root().join(path))
            .unwrap_or_else(|e| panic!("read {path}: {e}\n{}", self.screen_text()))
    }

    /// True when `path` (relative to the sandbox root) exists.
    pub fn exists(&self, path: &str) -> bool {
        self.root().join(path).symlink_metadata().is_ok()
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}
