//! The connection manager (FEAT-11) and credential prompts: saving
//! connections of any kind, connecting panels to the in-process FTP and
//! WebDAV test servers and copying local ↔ remote and remote ↔ remote
//! through the transfer engine; a background job asking for a password.

use crate::app::state::PopupType;
use crate::app::state::popup::{ConnectionPromptState, CredentialPrompt};
use crate::config::settings::{Connection, RemoteKind};
use crate::credential_prompt::CredentialRequest;
use crate::fs::remote::tests::served_folder;
use crate::test_harness::Harness;

/// Tab presses from the connection list to the Connect button.
const TO_CONNECT: usize = ConnectionPromptState::BUTTON_CONNECT;

fn saved(name: &str, kind: RemoteKind, addr: std::net::SocketAddr, folder: &str) -> Connection {
    Connection {
        name: name.into(),
        kind,
        host: addr.ip().to_string(),
        port: addr.port().to_string(),
        path: format!("/{folder}"),
        ..Connection::default()
    }
}

#[test]
fn new_connections_of_any_kind_are_saved_from_the_dialog() {
    let mut h = Harness::new();
    h.keys("@ssh_connect");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::ConnectionPrompt(_))
    ));
    // Name, then the protocol (two steps right: FTP, FTPS), then the host.
    h.text("files")
        .keys("Down Right Right Down")
        .text("ftp.example.org");
    h.keys(&format!("Tab*{}", ConnectionPromptState::BUTTON_SAVE - 3))
        .keys("Enter");
    let conn = &h.ctx.config.settings.connections[0];
    assert_eq!(
        (
            conn.name.as_str(),
            conn.kind,
            conn.host.as_str(),
            conn.port.as_str()
        ),
        ("files", RemoteKind::Ftps, "ftp.example.org", "21")
    );
    h.render();
    h.assert_screen("files (FTPS)");
}

#[test]
fn panels_connect_and_copy_between_local_and_remote_folders() {
    let (ftp_dir, ftp_addr, ftp_folder, _ftp_turn) = served_folder(false);
    let (dav_dir, dav_addr, dav_folder, _) = served_folder(true);
    let mut h = Harness::builder()
        .settings(|s| {
            s.connections = vec![
                saved("ftp", RemoteKind::Ftp, ftp_addr, &ftp_folder),
                saved("dav", RemoteKind::WebDav, dav_addr, &dav_folder),
            ];
        })
        .build();
    let remote = |h: &Harness, right: bool| {
        let side = if right {
            crate::app::state::ActivePanel::Right
        } else {
            crate::app::state::ActivePanel::Left
        };
        h.state.panels.side(side).source.remote().is_some()
    };

    // Left panel: the FTP connection (first in the list).
    h.keys(&format!("@ssh_connect Tab*{TO_CONNECT} Enter"));
    h.wait_until("FTP panel", |h| remote(h, false));
    h.settle();
    h.render();
    h.assert_screen("[FTP: ");

    // Upload: local (right) → FTP (left).
    h.write("work/right/up.txt", "uploaded");
    h.keys("Tab").reread().focus("up.txt").keys("@copy Enter");
    h.wait_until("upload", |_| ftp_dir.path().join("up.txt").exists());
    h.settle();
    assert_eq!(
        std::fs::read(ftp_dir.path().join("up.txt")).unwrap(),
        b"uploaded"
    );

    // Download: FTP (left) → local (right).
    std::fs::write(ftp_dir.path().join("down.txt"), "downloaded").unwrap();
    h.keys("Tab").reread().focus("down.txt").keys("@copy Enter");
    h.wait_until("download", |h| h.exists("work/right/down.txt"));
    h.settle();
    assert_eq!(h.read("work/right/down.txt"), b"downloaded");

    // Right panel: the WebDAV connection; then FTP → WebDAV.
    h.keys(&format!("Tab @ssh_connect Down Tab*{TO_CONNECT} Enter"));
    h.wait_until("WebDAV panel", |h| remote(h, true));
    h.settle();
    h.keys("Tab").reread().focus("up.txt").keys("@copy Enter");
    h.wait_until("remote to remote", |_| {
        dav_dir.path().join("up.txt").exists()
    });
    h.settle();
    assert_eq!(
        std::fs::read(dav_dir.path().join("up.txt")).unwrap(),
        b"uploaded"
    );
}

/// Waits for the prompt of a request made on another thread.
fn wait_for_prompt(h: &mut Harness) {
    h.wait_until("credential prompt", |h| {
        h.state.dialogs.top_as::<CredentialPrompt>().is_some()
    });
}

#[test]
fn background_jobs_get_the_secret_typed_in_the_prompt() {
    let mut h = Harness::new();
    let asker = h.state.credential_requests.asker();
    let job = std::thread::spawn(move || asker.ask(CredentialRequest::password("me@host")));
    wait_for_prompt(&mut h);
    h.text("s3cret");
    h.render();
    h.assert_screen("me@host");
    h.assert_screen("******");
    h.assert_no_screen("s3cret");
    h.keys("Enter");
    assert_eq!(job.join().unwrap().as_deref().map(String::as_str), Some("s3cret"));
    assert!(h.state.dialogs.top_as::<CredentialPrompt>().is_none());
}

#[test]
fn escape_or_a_closed_prompt_cancels_the_request() {
    let mut h = Harness::new();
    let asker = h.state.credential_requests.asker();
    let ask = |asker: crate::credential_prompt::Asker| {
        std::thread::spawn(move || {
            asker.ask(CredentialRequest::passphrase(std::path::Path::new("id")))
        })
    };
    let job = ask(asker.clone());
    wait_for_prompt(&mut h);
    h.keys("Esc");
    assert_eq!(job.join().unwrap(), None);

    // Another dialog replaced the prompt: the job is not left waiting.
    let job = ask(asker);
    wait_for_prompt(&mut h);
    h.state.dialogs.clear();
    h.pump();
    assert_eq!(job.join().unwrap(), None);
}
