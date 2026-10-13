//! Background plugin jobs (install / update) reported as toasts.

use crate::plugin::manager::PluginRequest;
use std::fmt::Debug;
use std::future::Future;

/// Title and message of a toast.
pub struct Toast {
    pub title: String,
    pub msg: String,
}

/// Sends a toast to the UI (ignored when the plugin manager is gone).
pub async fn notify(toast: Toast, level: &str) {
    let _ = crate::plugin::PluginManager::get_sender()
        .send(PluginRequest::Notify {
            title: toast.title,
            msg: toast.msg,
            level: level.to_string(),
        })
        .await;
}

/// Extra text a successful job adds to its toast (e.g. an unsigned
/// registry warning).
pub trait ToastDetail {
    fn detail(&self) -> Option<String>;
}

impl ToastDetail for crate::plugin::updater::Report {
    fn detail(&self) -> Option<String> {
        self.unsigned
            .then(|| crate::config::localization::t("plugin_registry_unsigned_warning"))
    }
}

/// Runs `job` in the background and shows `ok` (plus the job's detail), or
/// `err` with the `{:?}` in its message replaced by the error.
pub fn spawn_with_toast<T, E, F>(job: F, ok: Toast, err: Toast)
where
    F: Future<Output = Result<T, E>> + Send + 'static,
    T: ToastDetail + Send + 'static,
    E: Debug + Send + 'static,
{
    tokio::spawn(async move {
        match job.await {
            Ok(done) => match done.detail() {
                Some(extra) => {
                    let msg = format!("{}\n{extra}", ok.msg);
                    notify(Toast { msg, ..ok }, "warn").await
                }
                None => notify(ok, "info").await,
            },
            Err(e) => {
                let msg = err.msg.replace("{:?}", &format!("{:?}", e));
                notify(Toast { msg, ..err }, "error").await;
            }
        }
    });
}
