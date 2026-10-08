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

/// Runs `job` in the background and shows `ok`, or `err` with the `{:?}` in
/// its message replaced by the error.
pub fn spawn_with_toast<T, E, F>(job: F, ok: Toast, err: Toast)
where
    F: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Debug + Send + 'static,
{
    tokio::spawn(async move {
        match job.await {
            Ok(_) => notify(ok, "info").await,
            Err(e) => {
                let msg = err.msg.replace("{:?}", &format!("{:?}", e));
                notify(Toast { msg, ..err }, "error").await;
            }
        }
    });
}
