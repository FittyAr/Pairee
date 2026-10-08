pub mod actions;
#[allow(clippy::module_inception)]
pub mod app;
pub mod context;
pub mod git_ops;
pub mod input;
pub mod input_popup;
pub mod jobs;
pub mod list_nav;
pub mod menu_handler;
pub mod screen_input;
pub mod state;
pub mod sys_helpers;
pub mod text_input;

pub use app::run;
pub use context::AppContext;
pub use state::AppState;
