//! Keybinding subsystem: actions, presets, the resolver, and the
//! source-attribution types that tell the dispatcher where a
//! binding came from.
//!
//! See [`resolver`] for the entry point (`KeybindingResolver::new`)
//! and [`source`] for the priority table that decides what wins
//! when two sources register the same key.

pub mod actions;
pub mod preset;
pub mod resolver;
pub mod source;

pub use actions::Action;
pub use resolver::KeybindingResolver;
#[allow(unused_imports)]
pub use source::{BindingSource, ConflictPolicy, RegisterOutcome, ResolvedBinding};
