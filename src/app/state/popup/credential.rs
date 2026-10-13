//! The prompt a background job's credential request opens (see
//! `app::credential_requests`): one masked field for a password or a key
//! passphrase.

use crate::app::text_input::TextField;
use crate::credential_prompt::SecretKind;

#[derive(Debug, Clone)]
pub struct CredentialPrompt {
    /// The request this prompt answers.
    pub id: u64,
    pub kind: SecretKind,
    /// `user@host` or the key file.
    pub subject: String,
    pub input: TextField,
}
