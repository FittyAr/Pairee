//! Credential requests: a background network job (an SSH/SFTP login, a
//! Git fetch with an encrypted key, an FTP / WebDAV / S3 connection
//! without a stored password) asks the user for a secret and waits for the
//! answer, while the UI thread keeps running.
//!
//! The job calls [`Asker::ask`] on the asker of the state that started it:
//! the request travels over a channel to the UI, which shows a prompt
//! dialog (see `app::credential_requests`) and sends the answer back; Esc,
//! closing the dialog or quitting answers `None` and the job gives up.
//! Nothing here knows about the UI, and there is no process-wide asker:
//! jobs without one (tests, subcommands) use [`nobody`], which answers
//! `None` at once.
//!
//! Asking from the UI thread itself would wait forever for a dialog that
//! can never be painted, so such requests are refused (`None`).

pub mod key;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel as std_channel, sync_channel};
use std::thread::ThreadId;

/// What kind of secret is asked for (it decides the prompt's wording).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// A login password (or an S3 secret key).
    Password,
    /// The passphrase of an encrypted private key file.
    Passphrase,
}

/// A request for one secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialRequest {
    pub kind: SecretKind,
    /// What the secret is for: `user@host` or the key file path.
    pub subject: String,
}

impl CredentialRequest {
    pub fn password(subject: impl Into<String>) -> Self {
        Self {
            kind: SecretKind::Password,
            subject: subject.into(),
        }
    }

    pub fn passphrase(key: &std::path::Path) -> Self {
        Self {
            kind: SecretKind::Passphrase,
            subject: key.display().to_string(),
        }
    }
}

/// How a login asks the user for a secret (`None`: cancelled or nobody to
/// ask). Connections and Git network jobs get it from whoever started them.
pub type Ask<'a> = &'a (dyn Fn(CredentialRequest) -> Option<String> + Sync);

/// A request waiting for the user, with the way back to the job.
#[derive(Debug)]
pub struct Pending {
    pub id: u64,
    pub request: CredentialRequest,
    reply: SyncSender<Option<String>>,
}

impl Pending {
    /// Sends the answer (`None`: cancelled) to the waiting job.
    pub fn answer(self, secret: Option<String>) {
        // The job may have given up already; nothing to do then.
        let _ = self.reply.send(secret);
    }
}

/// The asking half, cheap to clone into background jobs.
#[derive(Debug, Clone)]
pub struct Asker {
    tx: Sender<Pending>,
    wake: fn(),
    ui_thread: Option<ThreadId>,
    next_id: Arc<AtomicU64>,
}

impl Asker {
    /// The same asker, refusing requests made from the calling thread (the
    /// one that answers them).
    pub fn answered_on_this_thread(mut self) -> Self {
        self.ui_thread = Some(std::thread::current().id());
        self
    }

    /// Asks the user and blocks until the answer comes (`None`: cancelled,
    /// or nobody is listening anymore).
    pub fn ask(&self, request: CredentialRequest) -> Option<String> {
        if self.ui_thread == Some(std::thread::current().id()) {
            tracing::warn!("credential request from the UI thread refused");
            return None;
        }
        let (reply, answer) = sync_channel(1);
        let pending = Pending {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            request,
            reply,
        };
        self.tx.send(pending).ok()?;
        (self.wake)();
        answer.recv().ok().flatten()
    }
}

/// The answering half, polled by the UI thread.
#[derive(Debug)]
pub struct Inbox {
    rx: Receiver<Pending>,
}

impl Inbox {
    /// The next request that arrived, if any (never blocks).
    pub fn try_next(&self) -> Option<Pending> {
        self.rx.try_recv().ok()
    }
}

/// A connected asker / inbox pair; `wake` is called after every request
/// so the event loop notices it without waiting for input.
pub fn channel(wake: fn()) -> (Asker, Inbox) {
    let (tx, rx) = std_channel();
    let asker = Asker {
        tx,
        wake,
        ui_thread: None,
        next_id: Arc::new(AtomicU64::new(1)),
    };
    (asker, Inbox { rx })
}

/// The answer when nobody can be asked: always cancelled.
pub fn nobody(_request: CredentialRequest) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_wake() {}

    #[test]
    fn a_job_gets_the_answer_the_ui_sends() {
        let (asker, inbox) = channel(no_wake);
        let job = std::thread::spawn(move || asker.ask(CredentialRequest::password("me@host")));
        let pending = loop {
            if let Some(p) = inbox.try_next() {
                break p;
            }
            std::thread::yield_now();
        };
        assert_eq!(pending.request, CredentialRequest::password("me@host"));
        pending.answer(Some("pw".into()));
        assert_eq!(job.join().unwrap().as_deref(), Some("pw"));
    }

    #[test]
    fn cancelling_or_dropping_the_request_answers_none() {
        let (asker, inbox) = channel(no_wake);
        let job = std::thread::spawn({
            let asker = asker.clone();
            move || asker.ask(CredentialRequest::passphrase(std::path::Path::new("k")))
        });
        let pending = loop {
            if let Some(p) = inbox.try_next() {
                break p;
            }
            std::thread::yield_now();
        };
        assert_eq!(pending.request.kind, SecretKind::Passphrase);
        drop(pending);
        assert_eq!(job.join().unwrap(), None);
        drop(inbox);
        assert_eq!(asker.ask(CredentialRequest::password("x")), None);
    }

    #[test]
    fn the_answering_thread_cannot_ask_itself() {
        let (asker, inbox) = channel(no_wake);
        let asker = asker.answered_on_this_thread();
        assert_eq!(asker.ask(CredentialRequest::password("x")), None);
        assert!(inbox.try_next().is_none());
    }
}
