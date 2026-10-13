//! The event loop's pieces without a terminal: start-up dialogs, update
//! notices, the input dispatcher and the developer-tool progress channel.

use super::background::channels::process_dev_progress_updates;
use super::updates::process_update_events;
use crate::app::context::AppContext;
use crate::app::state::popup::{InfoMessage, OnboardingKeymap, UpdateAvailable};
use crate::app::state::{AppState, DevProgress, PluginMenuState, PopupType};
use crate::config::AppConfig;
use crate::config::localization::t;
use crate::terminal::Event;
use crate::test_harness::Harness;
use crate::update::{UpdateInfo, UpdateStatus};
use std::path::PathBuf;

fn state() -> AppState {
    AppState::new(PathBuf::from("."), PathBuf::from("."))
}

fn release(tag: &str) -> UpdateInfo {
    UpdateInfo {
        tag: tag.into(),
        version: tag.trim_start_matches('v').into(),
        release_notes: "notes".into(),
        html_url: String::new(),
        assets: Vec::new(),
    }
}

/// An update check that finished with `found`.
fn checked(state: &mut AppState, found: Option<UpdateInfo>) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(found).expect("receiver alive");
    state.update.check_rx = Some(rx);
}

fn info_text(state: &AppState) -> Option<&str> {
    match state.dialogs.top() {
        Some(PopupType::Info(InfoMessage(text))) => Some(text),
        _ => None,
    }
}

fn update_dialog(state: &AppState) -> &UpdateAvailable {
    state
        .dialogs
        .top_as::<UpdateAvailable>()
        .expect("update dialog on top")
}

#[test]
fn a_found_update_opens_its_dialog_only_over_the_panels() {
    let mut context = AppContext::new(AppConfig::default());
    let mut quiet = state();
    checked(&mut quiet, Some(release("v9.0.0")));
    process_update_events(&mut quiet, &mut context);
    assert_eq!(update_dialog(&quiet).info.tag, "v9.0.0");
    assert!(quiet.update.check_rx.is_none());

    // Busy with another dialog: remembered, not shown.
    let mut busy = state();
    busy.dialogs.replace(PopupType::error("working"));
    checked(&mut busy, Some(release("v9.0.0")));
    process_update_events(&mut busy, &mut context);
    assert!(matches!(busy.dialogs.top(), Some(PopupType::Error(_))));
    assert_eq!(
        busy.update.available.map(|i| i.tag).as_deref(),
        Some("v9.0.0")
    );
}

#[test]
fn the_checking_notice_gives_way_to_the_outcome() {
    let mut context = AppContext::new(AppConfig::default());
    let mut none = state();
    none.dialogs.replace(PopupType::info(t("update_checking")));
    checked(&mut none, None);
    process_update_events(&mut none, &mut context);
    assert_eq!(info_text(&none), Some(t("update_no_updates").as_str()));

    let mut failed = state();
    failed
        .dialogs
        .replace(PopupType::info(t("update_checking")));
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<UpdateInfo>>();
    drop(tx);
    failed.update.check_rx = Some(rx);
    process_update_events(&mut failed, &mut context);
    assert_eq!(info_text(&failed), Some(t("update_check_failed").as_str()));

    // A background check (no notice on top) stays silent.
    let mut silent = state();
    checked(&mut silent, None);
    process_update_events(&mut silent, &mut context);
    assert!(silent.dialogs.is_empty());
}

#[test]
fn a_dismissed_version_is_only_mentioned() {
    let mut config = AppConfig::default();
    config.settings.dismissed_update_version = Some("v9.0.0".into());
    let mut context = AppContext::new(config);
    let mut state = state();
    checked(&mut state, Some(release("v9.0.0")));
    process_update_events(&mut state, &mut context);
    let expected = t("update_available_ignored").replace("{}", "v9.0.0");
    assert_eq!(info_text(&state), Some(expected.as_str()));
    assert!(state.update.available.is_none());
}

#[test]
fn download_progress_and_install_errors_reach_the_update_dialog() {
    let mut context = AppContext::new(AppConfig::default());
    let mut state = state();
    checked(&mut state, Some(release("v9.0.0")));
    process_update_events(&mut state, &mut context);

    let (tx, rx) = tokio::sync::mpsc::channel(4);
    tx.try_send(0.25).unwrap();
    tx.try_send(0.5).unwrap();
    drop(tx);
    state.update.progress_rx = Some(rx);
    process_update_events(&mut state, &mut context);
    assert_eq!(update_dialog(&state).install_progress, Some(0.5));
    assert_eq!(state.update.status, UpdateStatus::Downloading(0.5));
    assert!(state.update.progress_rx.is_none(), "sender gone");

    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(Err("boom".to_string())).unwrap();
    state.update.install_rx = Some(rx);
    process_update_events(&mut state, &mut context);
    let dialog = update_dialog(&state);
    assert_eq!(dialog.error.as_deref(), Some("boom"));
    assert_eq!(dialog.install_progress, None);
}

#[test]
fn install_results_without_the_dialog_become_notices() {
    use crate::update::installer::InstallResult;
    let mut context = AppContext::new(AppConfig::default());
    let mut state = state();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(Err("boom".to_string())).unwrap();
    state.update.install_rx = Some(rx);
    process_update_events(&mut state, &mut context);
    let failed = t("update_failed").replace("{}", "boom");
    assert_eq!(info_text(&state), Some(failed.as_str()));

    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(Ok(InstallResult::RestartRequired)).unwrap();
    state.update.install_rx = Some(rx);
    process_update_events(&mut state, &mut context);
    assert_eq!(
        info_text(&state),
        Some(t("update_installed_restart").as_str())
    );
    assert_eq!(state.update.status, UpdateStatus::Done);
}

#[test]
fn the_first_frame_queues_onboarding_and_the_config_error_over_it() {
    let mut h = Harness::builder()
        .settings(|s| s.onboarding_completed = false)
        .build();
    assert!(h.state.dialogs.top_as::<OnboardingKeymap>().is_some());
    h.ctx.config.load_error = Some("config.toml: bad".into());
    super::prepare_first_frame(&mut h.state, &mut h.ctx);
    assert_eq!(h.state.dialogs.depth(), 2, "the error goes over onboarding");
    assert!(matches!(
        h.state.dialogs.top(),
        Some(PopupType::Error(e)) if e.0 == "config.toml: bad"
    ));
    assert!(h.ctx.config.load_error.is_none(), "reported once");
    // Dismissing it returns to the keymap picker.
    h.keys("Esc");
    assert!(h.state.dialogs.top_as::<OnboardingKeymap>().is_some());
}

#[test]
fn an_open_dialog_owns_every_key() {
    let mut h = Harness::new();
    for name in ["a.txt", "b.txt", "c.txt"] {
        h.write(&format!("work/left/{name}"), name);
    }
    h.reread();
    let before = h.cursor_name();
    h.state.dialogs.replace(PopupType::error("stop"));
    // Down means nothing to a message, but it must not reach the panel.
    h.keys("Down Down");
    assert_eq!(h.cursor_name(), before);
    assert!(h.state.dialogs.is_open());
    h.keys("Esc Down");
    assert!(h.state.dialogs.is_empty());
    assert_ne!(h.cursor_name(), before, "the panel has the keys again");
}

#[test]
fn terminate_quits_and_a_paste_goes_to_the_focused_field() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    h.send(Event::Paste("from_clipboard".into()));
    let typed = h
        .state
        .dialogs
        .top_mut()
        .and_then(|popup| popup.dialog_mut().focused_field_mut())
        .map(|field| field.text().to_string());
    assert_eq!(typed.as_deref(), Some("from_clipboard"));
    assert!(!h.state.should_quit);
    h.send(Event::Terminate);
    assert!(h.state.should_quit);
}

#[test]
fn developer_tool_progress_drives_the_plugin_manager() {
    use crate::plugin::developer_tool::{progress_finish, progress_progress, progress_status};
    let mut state = state();
    state.dialogs.replace(PluginMenuState::default());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<DevProgress>();
    state.plugins.dev_progress_rx = Some(rx);
    let sender = Some(tx);

    progress_status(&sender, "reading");
    progress_progress(&sender, "checking", 2, 5);
    process_dev_progress_updates(&mut state);
    let menu = state.dialogs.top_as::<PluginMenuState>().unwrap();
    assert!(menu.dev_loading);
    assert_eq!(
        menu.dev_loading_status, "checking",
        "the latest update wins"
    );
    assert_eq!(menu.dev_loading_progress, Some((2, 5)));

    progress_finish(sender, Some("published".into()), None);
    process_dev_progress_updates(&mut state);
    let menu = state.dialogs.top_as::<PluginMenuState>().unwrap();
    assert!(!menu.dev_loading);
    assert_eq!(menu.dev_results, "published");
    assert_eq!(menu.dev_loading_progress, None);
    assert!(state.plugins.dev_progress_rx.is_none(), "channel closed");
}
