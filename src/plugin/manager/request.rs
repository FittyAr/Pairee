//! Cross-thread request envelope used by plugins to talk to the main loop.
//!
//! The enum itself is the wire format; the helper structs
//! (`DialogPosition`, `WhichCandidate`, `NotifyPayload`,
//! `InputDialogResult`) carry the structured payloads that the new M0
//! APIs accept from Lua.
//!
//! **M1 note**: the dialog reply channels are now
//! `tokio::sync::mpsc::UnboundedSender<T>` instead of
//! `oneshot::Sender<T>` so the sender can be cloned into the
//! `PopupType` enum (the main loop's input handlers all clone the
//! active popup before mutating it). The receiver still gets exactly
//! one value because the channel is consumed by the
//! `oneshot_compat::recv_single` helper at the call site.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

pub enum PluginRequest {
    GetStateSnapshot(oneshot::Sender<super::snapshot::AppStateSnapshot>),
    /// Legacy notify path: renders `<title>: <msg>`. Kept for backwards
    /// compatibility with existing plugins; new plugins should prefer
    /// `NotifyStructured`.
    Notify {
        title: String,
        msg: String,
        level: String,
    },
    /// Structured notify payload used by `pairee.notify({title, content,
    /// level, timeout})`.
    NotifyStructured(NotifyPayload),
    Cd {
        path: String,
    },
    SetFocus {
        side: String,
    },
    /// Deprecated input stub. Real input dialogs flow through `InputDialog`.
    /// Kept only so old plugins do not crash before they migrate.
    Input {
        title: String,
        default: String,
        reply_tx: mpsc::UnboundedSender<String>,
    },
    /// Deprecated confirm stub. Real confirm dialogs flow through
    /// `ConfirmDialog`. Kept only so old plugins do not crash before
    /// they migrate.
    Confirm {
        title: String,
        msg: String,
        reply_tx: mpsc::UnboundedSender<bool>,
    },
    /// Real input dialog. `realtime` and `debounce_secs` are reserved
    /// for the M1.5 streaming variant — today the channel is used
    /// one-shot (the main loop sends one `InputDialogResult` on
    /// submit/cancel and drops the sender).
    InputDialog {
        title: String,
        default: String,
        position: Option<DialogPosition>,
        obscure: bool,
        realtime: bool,
        debounce_secs: f64,
        reply_tx: mpsc::UnboundedSender<InputDialogResult>,
    },
    /// Real confirm dialog. Returns the user's yes/no decision.
    ConfirmDialog {
        title: String,
        msg: String,
        position: Option<DialogPosition>,
        reply_tx: mpsc::UnboundedSender<bool>,
    },
    /// Key-prompt: waits for the user to press one of the candidate keys.
    /// `silent` hides the on-screen candidate list.
    WhichPrompt {
        candidates: Vec<WhichCandidate>,
        silent: bool,
        reply_tx: mpsc::UnboundedSender<Option<usize>>,
    },
    /// Generic action dispatch (introduced in M0). Any `Action` in
    /// `src/keybindings/actions.rs` is callable. The optional `reply_tx`
    /// receives a JSON value when the action supports result data.
    EmitAction {
        name: String,
        args: serde_json::Value,
        reply_tx: Option<oneshot::Sender<serde_json::Value>>,
    },
    /// Returns a stable cache URL for `(file, skip)` so previewers can
    /// generate and reuse the same cache file across invocations.
    FileCache {
        file_path: PathBuf,
        skip: usize,
        reply_tx: oneshot::Sender<Option<PathBuf>>,
    },
    SpawnCopyTask {
        from: PathBuf,
        to: PathBuf,
    },
    UpdatePluginWidget {
        path: PathBuf,
        widget: crate::app::state::types::PluginWidget,
    },
    /// Result of an asynchronous load of the installed-plugins list
    /// (triggered when opening the Plugin Manager). The receiver is the
    /// `(name, version, pinned, trusted, update_available)` tuple used by the
    /// `PluginMenu` popup's `installed` field.
    PluginMenuLoaded {
        installed: Vec<(String, String, bool, bool, Option<String>)>,
    },
    /// Result of an asynchronous scan of the dev plugins folder (and the two
    /// panel paths) for Option 0 "Select active development plugin".
    DevPluginScan {
        options: Vec<(String, String)>,
    },
    /// M2 placeholder: a plugin called `pairee.image.show(url, rect)`.
    /// The dispatcher in M3 will route this into the `QuickViewPanel`
    /// (or whichever preview surface is active). For M2 we accept the
    /// request and log it; the image is already decoded in the
    /// binding so the bytes can be carried over.
    ImagePreview {
        path: PathBuf,
        rect: ImageRect,
    },
}

/// Rectangular region on the terminal, in cells, used by
/// `pairee.image.show(url, rect)` and (in M3) the rest of the
/// `Renderable` surface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageRect {
    pub x: i32,
    pub y: i32,
    pub w: u16,
    pub h: u16,
}

/// Position hint for a dialog popup. Mirrors the public Lua shape that
/// `pairee.input` / `pairee.confirm` / `pairee.which` accept via their `pos`
/// field. Today only the origin and size are honoured; x/y offsets are
/// reserved for future use and stored for forward-compatibility.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogPosition {
    pub origin: String,
    pub x: i32,
    pub y: i32,
    pub w: u16,
    pub h: u16,
}

/// Candidate entry for `pairee.which`. `on` is a list of one or more key
/// strings (e.g. "a", "<C-c>") that the user can press to match this
/// candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhichCandidate {
    pub on: Vec<String>,
    pub desc: Option<String>,
}

/// Structured notify payload used by `pairee.notify`.
///
/// The legacy `PluginRequest::Notify { title, msg, level }` path remains
/// available for backwards compatibility; new code should prefer this
/// structured variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyPayload {
    pub title: String,
    pub content: String,
    pub level: Option<String>,
    pub timeout_secs: Option<f64>,
}

/// Result payload for a streaming `pairee.input` dialog.
///
/// - `value` is the text the user has entered (empty on cancel).
/// - `event` is an integer tag: 0 = unknown / channel closed, 1 = submitted
///   (Enter), 2 = cancelled (Esc), 3 = typed (realtime only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputDialogResult {
    pub value: String,
    pub event: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_position_default() {
        let p = DialogPosition::default();
        assert_eq!(p.origin, "");
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
        assert_eq!(p.w, 0);
        assert_eq!(p.h, 0);
    }

    #[test]
    fn test_which_candidate_serialization_roundtrip() {
        let c = WhichCandidate {
            on: vec!["a".to_string(), "<C-c>".to_string()],
            desc: Some("press a or Ctrl+C".to_string()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: WhichCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.on, c.on);
        assert_eq!(back.desc, c.desc);
    }

    #[test]
    fn test_input_dialog_result_serialization_roundtrip() {
        let r = InputDialogResult {
            value: "hello".to_string(),
            event: 1,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: InputDialogResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, r.value);
        assert_eq!(back.event, r.event);
    }

    #[test]
    fn test_notify_payload_serialization_roundtrip() {
        let p = NotifyPayload {
            title: "t".to_string(),
            content: "c".to_string(),
            level: Some("error".to_string()),
            timeout_secs: Some(0.0),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: NotifyPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, p.title);
        assert_eq!(back.content, p.content);
        assert_eq!(back.level, p.level);
    }
}
