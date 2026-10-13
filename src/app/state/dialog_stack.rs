//! Overlay dialog stack.
//!
//! The top frame takes the keys and the mouse; the UI paints every frame,
//! bottom to top. `replace` is the common "open this dialog" path (it
//! closes any underneath); `push` / `pop` nest a dialog over another one
//! that stays visible and gets the keys back (the Copy / Move file mask and
//! folder tree, the Git prompts over the Git panel, the screens list, the
//! development plugin picker, messages over a dialog).

use super::popup::{DialogKind, PopupType};

/// Stack of overlay dialogs. Empty means the dual-panel UI has focus.
#[derive(Debug, Default, Clone)]
pub struct DialogStack {
    frames: Vec<PopupType>,
}

impl DialogStack {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn top(&self) -> Option<&PopupType> {
        self.frames.last()
    }

    pub fn top_mut(&mut self) -> Option<&mut PopupType> {
        self.frames.last_mut()
    }

    /// The top dialog when it is a `T`.
    pub fn top_as<T: DialogKind>(&self) -> Option<&T> {
        self.top().and_then(T::peek)
    }

    /// The top dialog, mutably, when it is a `T`.
    pub fn top_mut_as<T: DialogKind>(&mut self) -> Option<&mut T> {
        self.top_mut().and_then(T::peek_mut)
    }

    /// Closes the top dialog when it is a `T` and returns it.
    pub fn pop_as<T: DialogKind>(&mut self) -> Option<T> {
        self.top_as::<T>()?;
        self.pop().and_then(T::take)
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn is_open(&self) -> bool {
        !self.frames.is_empty()
    }

    /// Every open dialog, bottom to top.
    pub fn iter(&self) -> impl Iterator<Item = &PopupType> {
        self.frames.iter()
    }

    /// Every open dialog, bottom to top.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PopupType> {
        self.frames.iter_mut()
    }

    /// Pop the top dialog (close it).
    pub fn pop(&mut self) -> Option<PopupType> {
        self.frames.pop()
    }

    /// Close every overlay.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Open `popup` as the sole overlay (replaces the stack).
    pub fn replace(&mut self, popup: impl Into<PopupType>) {
        self.frames.clear();
        self.frames.push(popup.into());
    }

    /// Push a nested dialog on top of the current one.
    pub fn push(&mut self, popup: impl Into<PopupType>) {
        self.frames.push(popup.into());
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::{ConfirmQuit, InfoMessage};

    #[test]
    fn replace_then_pop() {
        let mut stack = DialogStack::new();
        assert!(stack.is_empty());
        stack.replace(PopupType::info("a"));
        assert!(stack.is_open());
        assert_eq!(stack.depth(), 1);
        stack.push(PopupType::info("b"));
        assert_eq!(stack.depth(), 2);
        match stack.pop() {
            Some(PopupType::Info(InfoMessage(s))) => assert_eq!(s, "b"),
            _ => panic!("expected nested info"),
        }
        assert_eq!(stack.depth(), 1);
        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn typed_access_reads_and_pops_only_its_own_type() {
        let mut stack = DialogStack::new();
        assert!(stack.pop_as::<InfoMessage>().is_none(), "empty stack");
        stack.replace(PopupType::info("under"));
        stack.push(ConfirmQuit);
        assert!(
            stack.top_as::<InfoMessage>().is_none(),
            "the top is the question"
        );
        assert!(
            stack.pop_as::<InfoMessage>().is_none(),
            "a different type stays"
        );
        assert_eq!(stack.depth(), 2);
        assert!(stack.pop_as::<ConfirmQuit>().is_some());
        let InfoMessage(text) = stack
            .top_mut_as::<InfoMessage>()
            .expect("message back on top");
        text.push_str(" again");
        assert_eq!(
            stack.top_as::<InfoMessage>().map(|m| m.0.as_str()),
            Some("under again")
        );
    }
}
