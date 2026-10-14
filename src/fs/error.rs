//! User-visible file-system errors in the active language. Panels and
//! dialogs show an error's outermost message only, so the localized
//! message carries the cause's text too; the cause itself stays in the
//! chain (it can still be downcast, e.g. to an `io::Error`).

use crate::config::localization::{t, t_fill};
use std::path::Path;

/// `cause` wrapped in the message `key`, whose two `{}` are filled with
/// `subject` and the cause's own message.
pub fn localized(key: &str, subject: &Path, cause: impl Into<anyhow::Error>) -> anyhow::Error {
    let cause = cause.into();
    let message = t_fill(key, &[&subject.display().to_string(), &cause.to_string()]);
    cause.context(message)
}

/// [`localized`] for results: `result.localized(key, subject)`.
pub trait LocalizedResult<T> {
    fn localized(self, key: &str, subject: &Path) -> anyhow::Result<T>;
}

impl<T, E: Into<anyhow::Error>> LocalizedResult<T> for Result<T, E> {
    fn localized(self, key: &str, subject: &Path) -> anyhow::Result<T> {
        self.map_err(|e| localized(key, subject, e))
    }
}

/// The error of a job the user cancelled.
pub fn cancelled() -> anyhow::Error {
    anyhow::anyhow!(t("transfer_job_cancelled"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_names_the_subject_and_the_cause_and_keeps_it() {
        let cause = std::io::Error::from(std::io::ErrorKind::NotFound);
        let text = cause.to_string();
        let err = localized("fs_error_create_dir", Path::new("box"), cause);
        let shown = err.to_string();
        assert!(shown.contains("box") && shown.contains(&text), "{shown}");
        assert!(err.downcast_ref::<std::io::Error>().is_some());
    }
}
