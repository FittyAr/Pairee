//! Overlay dialog stack (Phase C.3).
//!
//! The top frame is what the UI renders and what input handlers mutate.
//! `replace` is the common “open this dialog” path (clears any underneath).
//! `push` / `pop` are for nested prompts (filter on copy, git sub-dialogs).

use super::popup::PopupType;

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

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn is_open(&self) -> bool {
        !self.frames.is_empty()
    }

    /// Option-like aliases used at existing call sites.
    pub fn is_some(&self) -> bool {
        self.is_open()
    }

    pub fn is_none(&self) -> bool {
        self.is_empty()
    }

    #[allow(dead_code)]
    pub fn as_ref(&self) -> Option<&PopupType> {
        self.top()
    }

    #[allow(dead_code)]
    pub fn as_mut(&mut self) -> Option<&mut PopupType> {
        self.top_mut()
    }

    /// Every open dialog, bottom to top.
    #[cfg(test)]
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

    /// Alias of [`Self::pop`] for former `Option::take` call sites.
    pub fn take(&mut self) -> Option<PopupType> {
        self.pop()
    }

    /// Close every overlay.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Open `popup` as the sole overlay (replaces the stack).
    pub fn replace(&mut self, popup: PopupType) {
        self.frames.clear();
        self.frames.push(popup);
    }

    /// Push a nested dialog on top of the current one.
    #[allow(dead_code)]
    pub fn push(&mut self, popup: PopupType) {
        self.frames.push(popup);
    }

    /// Option-like set: `Some` replaces, `None` clears.
    pub fn set(&mut self, popup: Option<PopupType>) {
        match popup {
            Some(p) => self.replace(p),
            None => self.clear(),
        }
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Opens a sub-dialog that remembers the current top dialog (moved, not
    /// cloned) so it can be restored later: `build` receives the previous
    /// popup and returns the new one, which becomes the sole overlay. Does
    /// nothing when no dialog is open.
    pub fn open_over(&mut self, build: impl FnOnce(Box<PopupType>) -> PopupType) {
        if let Some(previous) = self.frames.pop() {
            self.frames.clear();
            self.frames.push(build(Box::new(previous)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_then_pop() {
        let mut stack = DialogStack::new();
        assert!(stack.is_none());
        stack.replace(PopupType::Info("a".into()));
        assert!(stack.is_some());
        assert_eq!(stack.depth(), 1);
        stack.push(PopupType::Info("b".into()));
        assert_eq!(stack.depth(), 2);
        match stack.pop() {
            Some(PopupType::Info(s)) => assert_eq!(s, "b"),
            _ => panic!("expected nested info"),
        }
        assert_eq!(stack.depth(), 1);
        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn open_over_moves_previous_into_new_dialog() {
        let mut stack = DialogStack::new();
        stack.open_over(|_| PopupType::Info("never".into()));
        assert!(stack.is_empty(), "no-op without an open dialog");

        stack.replace(PopupType::Info("panel".into()));
        stack.open_over(|prev| match *prev {
            PopupType::Info(s) => PopupType::Error(format!("over {s}")),
            _ => panic!("unexpected previous popup"),
        });
        assert_eq!(stack.depth(), 1);
        match stack.top() {
            Some(PopupType::Error(s)) => assert_eq!(s, "over panel"),
            other => panic!("unexpected top: {other:?}"),
        }
    }
}
