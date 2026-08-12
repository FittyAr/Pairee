pub mod discovery;
pub mod loader;
pub mod translator;
pub mod types;

#[cfg(test)]
mod tests;

// Re-exports
pub use discovery::discover_languages;
pub use loader::{get_active_language_code, load_language};
pub use translator::t;
