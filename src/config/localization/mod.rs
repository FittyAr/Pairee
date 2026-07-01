pub mod discovery;
pub mod loader;
pub mod tests;
pub mod translator;
pub mod types;

// Re-exports
pub use discovery::discover_languages;
pub use loader::{get_active_language_code, load_language};
pub use translator::t;
