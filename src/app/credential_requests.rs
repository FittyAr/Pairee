//! The UI side of [`crate::credential_prompt`]: requests from background
//! jobs wait here and are shown one at a time as a [`CredentialPrompt`]
//! pushed over whatever dialog is open. Enter answers with the typed
//! secret, Esc with `None`; a prompt that disappears without an answer
//! (another dialog replaced the stack) cancels its request, so a job never
//! waits for a dialog that is gone.

use crate::app::state::AppState;
use crate::app::state::popup::CredentialPrompt;
use crate::app::text_input::TextField;
use crate::credential_prompt::{self, Asker, Inbox, Pending};
use std::collections::VecDeque;

pub struct CredentialRequests {
    asker: Asker,
    inbox: Inbox,
    waiting: VecDeque<Pending>,
    /// The request whose prompt is open.
    shown: Option<Pending>,
}

impl Default for CredentialRequests {
    fn default() -> Self {
        let (asker, inbox) = credential_prompt::channel(crate::app::jobs::wake_event_loop);
        Self {
            // The state lives on the UI thread, which answers: a job run
            // inline there (no runtime) must not wait for itself.
            asker: asker.answered_on_this_thread(),
            inbox,
            waiting: VecDeque::new(),
            shown: None,
        }
    }
}

impl std::fmt::Debug for CredentialRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialRequests")
            .field("waiting", &self.waiting.len())
            .field("shown", &self.shown.as_ref().map(|p| p.id))
            .finish()
    }
}

impl CredentialRequests {
    /// The asker background jobs use to reach this state's prompts.
    pub fn asker(&self) -> Asker {
        self.asker.clone()
    }

    /// Answers the shown request `id` (`None`: cancelled).
    fn answer(&mut self, id: u64, secret: Option<String>) {
        if self.shown.as_ref().is_some_and(|p| p.id == id)
            && let Some(pending) = self.shown.take()
        {
            pending.answer(secret);
        }
    }
}

/// Takes new requests, cancels a shown one whose prompt was closed by
/// something else and shows the next. Called every event-loop pass.
pub fn process(state: &mut AppState) {
    let requests = &mut state.credential_requests;
    while let Some(pending) = requests.inbox.try_next() {
        requests.waiting.push_back(pending);
    }
    if let Some(id) = requests.shown.as_ref().map(|p| p.id)
        && !prompt_open(state, id)
    {
        state.credential_requests.answer(id, None);
    }
    let requests = &mut state.credential_requests;
    if requests.shown.is_some() {
        return;
    }
    let Some(next) = requests.waiting.pop_front() else {
        return;
    };
    let prompt = CredentialPrompt {
        id: next.id,
        kind: next.request.kind,
        subject: next.request.subject.clone(),
        input: TextField::default(),
    };
    requests.shown = Some(next);
    state.dialogs.push(prompt);
    state.mark_ui_dirty();
}

fn prompt_open(state: &AppState, id: u64) -> bool {
    state
        .dialogs
        .iter()
        .any(|d| matches!(d, crate::app::state::PopupType::CredentialPrompt(p) if p.id == id))
}

/// Closes the prompt on top and sends its answer (`secret`, or `None` when
/// cancelled) to the job that asked.
pub fn finish_prompt(state: &mut AppState, submit: bool) {
    let Some(prompt) = state.dialogs.pop_as::<CredentialPrompt>() else {
        return;
    };
    let secret = submit.then(|| prompt.input.into_text());
    state.credential_requests.answer(prompt.id, secret);
    state.mark_ui_dirty();
}
