pub mod actions;
#[allow(clippy::module_inception)]
pub mod app;
pub mod auto_refresh;
pub mod config_rows;
pub mod context;
pub mod disk_usage;
pub mod editor;
pub mod form;
pub mod git_local;
pub mod git_ops;
pub mod git_panel_load;
pub mod input;
pub mod input_popup;
pub mod jobs;
pub mod list_nav;
pub mod menu_handler;
pub mod screen_input;
pub mod session;
pub mod state;
pub mod sync;
pub mod sys_helpers;
pub mod text_input;

pub use app::run;
pub use context::AppContext;
pub use state::AppState;
