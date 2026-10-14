//! The UI side of [`crate::credential_prompt`]: requests from background
//! jobs wait here and are shown one at a time as a [`CredentialPrompt`]
//! pushed over the dialog that is open, except one the user is typing in
//! (keys meant for it would land in the prompt, and Enter would send them
//! as the secret): the request waits until that dialog closes. Enter
//! answers with the typed secret, Esc with `None`; a prompt that
//! disappears without an answer (another dialog replaced the stack) is
//! shown again, so a request is never dropped behind the user's back and a
//! job never waits for a dialog that is gone.

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
    fn answer(&mut self, id: u64, secret: Option<crate::credential_prompt::Secret>) {
        if self.shown.as_ref().is_some_and(|p| p.id == id)
            && let Some(pending) = self.shown.take()
        {
            pending.answer(secret);
        }
    }
}

/// Takes new requests, queues again a shown one whose prompt was closed by
/// something else and shows the next. Called every event-loop pass.
pub fn process(state: &mut AppState) {
    let requests = &mut state.credential_requests;
    while let Some(pending) = requests.inbox.try_next() {
        requests.waiting.push_back(pending);
    }
    if let Some(id) = requests.shown.as_ref().map(|p| p.id)
        && !prompt_open(state, id)
    {
        let requests = &mut state.credential_requests;
        if let Some(lost) = requests.shown.take() {
            requests.waiting.push_front(lost);
        }
    }
    if state.credential_requests.shown.is_some() || typing_in_top_dialog(state) {
        return;
    }
    let requests = &mut state.credential_requests;
    let Some(next) = requests.waiting.pop_front() else {
        return;
    };
    let prompt = CredentialPrompt {
        id: next.id,
        kind: next.request.kind,
        subject: next.request.subject.clone(),
        retry: next.request.retry,
        input: TextField::default(),
    };
    requests.shown = Some(next);
    state.dialogs.push(prompt);
    state.mark_ui_dirty();
}

/// Whether the dialog on top has a text field taking the typed keys.
fn typing_in_top_dialog(state: &mut AppState) -> bool {
    state
        .dialogs
        .top_mut()
        .is_some_and(|popup| popup.dialog_mut().focused_field_mut().is_some())
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
    let secret = submit.then(|| crate::credential_prompt::Secret::new(prompt.input.into_text()));
    state.credential_requests.answer(prompt.id, secret);
    state.mark_ui_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::{ConfirmQuit, RenamePrompt};
    use std::path::PathBuf;

    /// A state with one request (id 1) from a job thread waiting to be shown.
    fn state_with_waiting_request() -> (AppState, std::thread::JoinHandle<()>) {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let asker = state.credential_requests.asker();
        let job = std::thread::spawn(move || {
            let _ = asker.ask(credential_prompt::CredentialRequest::password("me@host"));
        });
        let pending = loop {
            if let Some(p) = state.credential_requests.inbox.try_next() {
                break p;
            }
            std::thread::yield_now();
        };
        state.credential_requests.waiting.push_back(pending);
        (state, job)
    }

    #[test]
    fn a_request_waits_while_the_user_types_in_a_dialog() {
        let (mut state, job) = state_with_waiting_request();
        state.dialogs.replace(RenamePrompt {
            input: "a".into(),
            original: "a".into(),
            src_path: PathBuf::from("a"),
            parent_dir: PathBuf::from("."),
            cursor_idx: 0,
        });
        process(&mut state);
        assert!(state.credential_requests.shown.is_none());
        assert_eq!(state.dialogs.depth(), 1);
        state.dialogs.clear();
        process(&mut state);
        assert!(prompt_open(&state, 1));
        finish_prompt(&mut state, false);
        job.join().unwrap();
    }

    #[test]
    fn a_prompt_closed_by_another_dialog_is_shown_again() {
        let (mut state, job) = state_with_waiting_request();
        process(&mut state);
        assert!(prompt_open(&state, 1));
        state.dialogs.replace(ConfirmQuit);
        process(&mut state);
        assert!(state.credential_requests.shown.is_some(), "not cancelled");
        assert!(prompt_open(&state, 1));
        finish_prompt(&mut state, false);
        job.join().unwrap();
    }
}
