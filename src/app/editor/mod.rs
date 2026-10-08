//! Built-in text editor (F4): buffer, undo history and file handling.
//!
//! Pairee does not launch external editors; every "edit" entry point opens
//! this editor through [`open::open_in_editor`]. This module has no
//! dependency on the terminal backend so it can be unit tested.

pub mod document;
pub mod history;
mod navigation;
pub mod open;
pub mod options;
mod state;

pub use options::EditorOptions;
pub use state::EditorState;
