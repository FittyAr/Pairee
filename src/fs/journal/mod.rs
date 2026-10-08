//! Operation journal for undo/redo (no UI).
//!
//! Patterns: **Command** ([`FsCommand`], with [`FsCommand::inverse`]) and
//! **Memento** ([`Journal`] keeps what each operation did, bounded and in
//! memory). Flow: a finished operation is recorded ([`record`] for Transfer
//! Engine jobs) → undo takes the entry, [`check`]s the inverse against the
//! filesystem and runs what still applies through the regular paths
//! (Transfer Engine, multi-rename executor, mkdir/link primitives) → what
//! that run actually did goes to the redo stack, and vice versa.

pub mod check;
pub mod command;
mod history;
mod record;
pub(crate) mod trash;

#[cfg(test)]
mod tests;

pub use check::{Skipped, check};
pub use command::{FileBatch, FsCommand};
pub use history::{Direction, JobOrigin, Journal};
pub use record::from_transfer;
pub use trash::TrashIndex;
