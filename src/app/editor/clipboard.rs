//! Clipboard used by the built-in editor (`Ctrl+C` / `Ctrl+X` / `Ctrl+V`).
//!
//! Text goes to the system clipboard through a [`ClipboardBackend`]
//! (strategy). An internal buffer always keeps the last copied text so copy
//! and paste keep working inside Pairee when there is no system clipboard
//! (SSH sessions, headless Linux, a clipboard manager refusing access).

/// Access to a system clipboard.
pub trait ClipboardBackend: Send {
    fn set_text(&mut self, text: &str) -> Result<(), String>;
    fn get_text(&mut self) -> Result<String, String>;
}

/// The operating-system clipboard (via `arboard`).
pub struct SystemClipboard;

impl ClipboardBackend for SystemClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), String> {
        crate::app::sys_helpers::clipboard::set_text(text)
    }

    fn get_text(&mut self) -> Result<String, String> {
        crate::app::sys_helpers::clipboard::get_text()
    }
}

/// No system clipboard at all (tests, or when it must not be touched).
pub struct NoSystemClipboard;

impl ClipboardBackend for NoSystemClipboard {
    fn set_text(&mut self, _text: &str) -> Result<(), String> {
        Err(String::new())
    }

    fn get_text(&mut self) -> Result<String, String> {
        Err(String::new())
    }
}

/// System clipboard with an internal fallback buffer.
pub struct EditorClipboard {
    backend: Box<dyn ClipboardBackend>,
    internal: Option<String>,
}

impl Default for EditorClipboard {
    fn default() -> Self {
        Self::new(Box::new(SystemClipboard))
    }
}

impl EditorClipboard {
    pub fn new(backend: Box<dyn ClipboardBackend>) -> Self {
        Self {
            backend,
            internal: None,
        }
    }

    /// Clipboard that never touches the system one.
    pub fn internal_only() -> Self {
        Self::new(Box::new(NoSystemClipboard))
    }

    /// Stores `text`; returns `false` when only the internal buffer has it.
    pub fn copy(&mut self, text: String) -> bool {
        let system = self.backend.set_text(&text).is_ok();
        self.internal = Some(text);
        system
    }

    /// Text to paste: the system clipboard when readable and not empty,
    /// otherwise the last text copied inside Pairee.
    pub fn paste(&mut self) -> Option<String> {
        match self.backend.get_text() {
            Ok(text) if !text.is_empty() => Some(text),
            _ => self.internal.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Shared = std::sync::Arc<std::sync::Mutex<Option<String>>>;

    /// A system clipboard that works, shared with the test.
    struct Fake(Shared);

    impl ClipboardBackend for Fake {
        fn set_text(&mut self, text: &str) -> Result<(), String> {
            *self.0.lock().unwrap() = Some(text.to_string());
            Ok(())
        }

        fn get_text(&mut self) -> Result<String, String> {
            self.0.lock().unwrap().clone().ok_or_else(String::new)
        }
    }

    #[test]
    fn falls_back_to_internal_buffer() {
        let mut cb = EditorClipboard::internal_only();
        assert_eq!(cb.paste(), None);
        assert!(!cb.copy("abc".into()));
        assert_eq!(cb.paste().as_deref(), Some("abc"));
    }

    #[test]
    fn prefers_system_clipboard() {
        let shared = Shared::default();
        let mut cb = EditorClipboard::new(Box::new(Fake(shared.clone())));
        assert!(cb.copy("mine".into()));
        assert_eq!(shared.lock().unwrap().as_deref(), Some("mine"));
        *shared.lock().unwrap() = Some("from another app".into());
        assert_eq!(cb.paste().as_deref(), Some("from another app"));
    }
}
