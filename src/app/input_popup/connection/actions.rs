//! Enter in the connection dialog: pick a saved connection, save / delete
//! one, cancel or connect.

use crate::app::context::AppContext;
use crate::app::state::popup::ConnectionField;
use crate::app::state::{AppState, ConnectionPromptState as Prompt, PopupType, TabId};
use crate::config::localization::t;
use crate::config::settings::{Connection, RemoteKind};

pub fn handle_enter(state: &mut AppState, context: &mut AppContext) {
    let Some(PopupType::ConnectionPrompt(prompt)) = state.dialogs.top_mut() else {
        return;
    };
    match prompt.cursor_idx {
        Prompt::ROW_LIST => {
            let saved = &context.config.settings.connections;
            let idx = prompt.selected_idx.unwrap_or(0);
            match saved.get(idx) {
                Some(conn) => prompt.load(idx, conn),
                None => connect(state, context),
            }
        }
        Prompt::BUTTON_CANCEL => state.dialogs.clear(),
        Prompt::BUTTON_SAVE => {
            if let Err(key) = save(prompt, context) {
                state.dialogs.replace(PopupType::error(t(key)));
            }
        }
        Prompt::BUTTON_DELETE => delete(prompt, context),
        _ => connect(state, context),
    }
}

/// The message key of the first required field `conn` is missing, if any
/// (`saving`: the name is required too).
fn missing_field(conn: &Connection, saving: bool) -> Option<&'static str> {
    if saving && conn.name.is_empty() {
        return Some("error_ssh_preset_name_empty");
    }
    if conn.host.is_empty() {
        return Some(if conn.kind == RemoteKind::S3 {
            "error_conn_bucket_empty"
        } else {
            "error_ssh_host_empty"
        });
    }
    // FTP and WebDAV may log in anonymously, S3 with the environment's keys.
    if conn.kind == RemoteKind::Sftp && conn.username.is_empty() {
        return Some("error_ssh_user_empty");
    }
    None
}

/// Saves the fields as a connection (replacing one with the same name).
/// `Err` holds the message key of a missing required field.
fn save(prompt: &mut Prompt, context: &mut AppContext) -> Result<(), &'static str> {
    let conn = prompt.to_connection();
    if let Some(key) = missing_field(&conn, true) {
        return Err(key);
    }
    let saved = &mut context.config.settings.connections;
    let idx = match saved.iter().position(|c| c.name == conn.name) {
        Some(idx) => {
            saved[idx] = conn;
            idx
        }
        None => {
            saved.push(conn);
            saved.len() - 1
        }
    };
    prompt.selected_idx = Some(idx);
    context.config.save_logging();
    Ok(())
}

/// Deletes the selected connection and shows the next one (or empty fields).
fn delete(prompt: &mut Prompt, context: &mut AppContext) {
    let Some(idx) = prompt.selected_idx else {
        return;
    };
    let saved = &mut context.config.settings.connections;
    if idx >= saved.len() {
        return;
    }
    saved.remove(idx);
    match saved.len() {
        0 => {
            prompt.clear_fields();
            prompt.cursor_idx = ConnectionField::Name.row();
        }
        len => {
            let next = idx.min(len - 1);
            prompt.load(next, &saved[next]);
            prompt.cursor_idx = Prompt::ROW_LIST;
        }
    }
    context.config.save_logging();
}

/// Validates the fields and connects in the background.
fn connect(state: &mut AppState, context: &AppContext) {
    let Some(PopupType::ConnectionPrompt(prompt)) = state.dialogs.top() else {
        return;
    };
    let conn = prompt.to_connection();
    let panel = state.panels.active_tab_id(prompt.panel);
    if let Some(key) = missing_field(&conn, false) {
        state.dialogs.replace(PopupType::error(t(key)));
        return;
    }
    let message = t("progress_connecting_remote").replacen(
        "{}",
        &crate::fs::remote::user_at_host(&conn.username, &conn.host),
        1,
    );
    state.dialogs.replace(PopupType::info(message));
    start_connection(state, context, panel, conn);
}

/// Connects tab `panel` with `conn` in the background (blocking login on
/// the job pool, where a missing password may be asked for), applied by
/// `process_remote_connect_updates` on the UI thread.
pub fn start_connection(
    state: &mut AppState,
    context: &AppContext,
    panel: TabId,
    mut conn: Connection,
) {
    if conn.insecure_tls() {
        state.dialogs.push(PopupType::info(
            t("remote_tls_insecure_warning").replacen("{}", &conn.name, 1),
        ));
    }
    // Starting replaces a running connection: a restored tab waiting for
    // it tries again later instead of waiting forever.
    if state.remote_connect.is_running() {
        crate::app::session::remote::requeue_pending(state);
    }
    let timeout = std::time::Duration::from_secs(context.config.settings.ssh_timeout_secs);
    let asker = state.credential_requests.asker();
    state.remote_connect.start(move |_| {
        crate::config::credentials::retry_unread(&mut conn);
        let ask = |request| asker.ask(request);
        (panel, crate::fs::remote::open(&conn, timeout, &ask))
    });
}
