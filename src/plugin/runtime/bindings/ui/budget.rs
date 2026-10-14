//! Bounds on what a plugin can make the host build from Lua values.
//!
//! Converting a plugin value into widgets allocates in Rust, outside the
//! Lua memory cap, so every conversion is bounded while it runs (not only
//! once the result is complete):
//!
//! * nested lists may be at most [`MAX_NESTING`] deep, so a table that
//!   contains itself fails instead of overflowing the stack;
//! * the items collected into one widget are charged to a [`Budget`] of
//!   the `Renderable` size limits as they are added, so the same table or
//!   userdata listed many times cannot multiply into gigabytes;
//! * a `Bar` / `Fill` symbol (drawn once per cell) is at most
//!   [`MAX_SYMBOL_BYTES`] and a constraint list at most
//!   [`MAX_CONSTRAINTS`] long.

use crate::app::state::popup::plugin_widget::{
    MAX_RENDERABLE_BYTES, MAX_RENDERABLE_NODES, Renderable, WLine, WText,
};

/// Deepest nesting of lists inside one widget argument.
pub const MAX_NESTING: usize = 16;

/// Longest `Bar` / `Fill` symbol: one cell holds one grapheme.
pub const MAX_SYMBOL_BYTES: usize = 16;

/// Most constraints of a `Layout` or columns of a `Table`.
pub const MAX_CONSTRAINTS: usize = 256;

fn too_large() -> mlua::Error {
    mlua::Error::RuntimeError("ui widget too large to show".into())
}

/// Nodes and text bytes collected so far for one widget.
#[derive(Debug, Default)]
pub struct Budget {
    nodes: usize,
    bytes: usize,
}

impl Budget {
    /// Adds `nodes` and `bytes`; an error once the limits are exceeded.
    pub fn charge(&mut self, nodes: usize, bytes: usize) -> mlua::Result<()> {
        self.nodes = self.nodes.saturating_add(nodes);
        self.bytes = self.bytes.saturating_add(bytes);
        if self.nodes > MAX_RENDERABLE_NODES || self.bytes > MAX_RENDERABLE_BYTES {
            return Err(too_large());
        }
        Ok(())
    }

    pub fn text(&mut self, text: &WText) -> mlua::Result<()> {
        self.charge(text.spans(), text.bytes())
    }

    pub fn line(&mut self, line: &WLine) -> mlua::Result<()> {
        let bytes = line.spans.iter().map(|s| s.content.len()).sum();
        self.charge(line.spans.len(), bytes)
    }

    pub fn widget(&mut self, widget: &Renderable) -> mlua::Result<()> {
        let (nodes, bytes) = widget.weight();
        self.charge(nodes, bytes)
    }
}

/// The nesting level below `depth`, or an error past [`MAX_NESTING`].
pub fn nested(depth: usize) -> mlua::Result<usize> {
    if depth >= MAX_NESTING {
        return Err(mlua::Error::RuntimeError(format!(
            "ui: lists nested deeper than {MAX_NESTING} levels"
        )));
    }
    Ok(depth + 1)
}

/// `symbol` when it fits in one cell's [`MAX_SYMBOL_BYTES`].
pub fn symbol_arg(symbol: String) -> mlua::Result<String> {
    if symbol.len() > MAX_SYMBOL_BYTES {
        return Err(mlua::Error::RuntimeError(format!(
            "ui: a symbol is drawn in every cell and may have at most {MAX_SYMBOL_BYTES} bytes"
        )));
    }
    Ok(symbol)
}

/// `len` items when they stay within [`MAX_CONSTRAINTS`].
pub fn constraint_count(len: usize) -> mlua::Result<()> {
    if len > MAX_CONSTRAINTS {
        return Err(mlua::Error::RuntimeError(format!(
            "ui: at most {MAX_CONSTRAINTS} constraints"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_fails_once_the_limits_are_passed() {
        let mut budget = Budget::default();
        budget.charge(MAX_RENDERABLE_NODES, 0).unwrap();
        assert!(budget.charge(1, 0).is_err());
        let mut budget = Budget::default();
        assert!(budget.charge(0, MAX_RENDERABLE_BYTES + 1).is_err());
        assert!(budget.charge(usize::MAX, usize::MAX).is_err());
    }

    #[test]
    fn nesting_symbols_and_constraints_are_bounded() {
        assert_eq!(nested(0).unwrap(), 1);
        assert!(nested(MAX_NESTING).is_err());
        assert!(symbol_arg("█".into()).is_ok());
        assert!(symbol_arg("x".repeat(MAX_SYMBOL_BYTES + 1)).is_err());
        assert!(constraint_count(MAX_CONSTRAINTS).is_ok());
        assert!(constraint_count(MAX_CONSTRAINTS + 1).is_err());
    }
}
