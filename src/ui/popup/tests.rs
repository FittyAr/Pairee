//! Painting the dialog stack: dialogs under the top one, questions and
//! plugin dialogs (one view per dialog type, through `render_dialog`).

use crate::app::state::popup::{
    ConfirmClearHistory, ConfirmInterrupt, ConfirmQuit, ConfirmReload, CopyMoveFilterPrompt,
    PluginDialog, SaveSetupConfirm,
};
use crate::app::state::{CopyMovePromptState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton};

/// The Copy dialog on `a.txt`, with the file mask prompt pushed over it.
fn mask_over_copy() -> Harness {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread();
    h.focus("a.txt").keys("@copy");
    assert!(h.state.dialogs.top_as::<CopyMovePromptState>().is_some());
    h.state.dialogs.push(CopyMoveFilterPrompt {
        input: "*.txt".into(),
    });
    h.render();
    h
}

/// Controls recorded last frame that end by pressing Space (option rows).
fn option_rows(h: &mut Harness) -> usize {
    h.render();
    h.state
        .hits
        .collect(|t| {
            matches!(t, HitTarget::PopupKeys(keys)
                if keys.last().map(|k| k.code) == Some(KeyCode::Char(' ')))
            .then_some(())
        })
        .len()
}

fn bodies(h: &mut Harness) -> usize {
    h.render();
    h.state
        .hits
        .collect(|t| (*t == HitTarget::PopupBody).then_some(()))
        .len()
}

#[test]
fn a_dialog_under_another_stays_painted_but_only_the_top_takes_the_mouse() {
    let mut h = mask_over_copy();
    let tree = t("btn_f10_tree");
    h.assert_screen(&t("prompt_filter_title"));
    h.assert_screen(&tree);
    assert_eq!(bodies(&mut h), 1, "only the mask prompt records its body");
    assert_eq!(option_rows(&mut h), 0, "the copy options are not clickable");
    // A click on the copy dialog under it is outside the prompt, which
    // holds typed input and stays.
    let (x, y) = h.locate(&tree);
    h.click(x + 1, y);
    assert!(h.state.dialogs.top_as::<CopyMoveFilterPrompt>().is_some());

    h.keys("Esc");
    assert_eq!(h.state.dialogs.depth(), 1);
    assert!(
        option_rows(&mut h) > 0,
        "the copy dialog has the mouse again"
    );
}

#[test]
fn moving_the_top_dialog_leaves_the_one_under_it_in_place() {
    let mut h = mask_over_copy();
    // The first line of the copy dialog, above the prompt (which moves down).
    let existing = t("prompt_copy_sing").replace("{}", "a.txt");
    let under = h.locate(&existing);
    let title = t("prompt_filter_title");
    let before = h.locate(title.trim());
    let (x, y) = h.hit_cell(|t| *t == HitTarget::PopupTitle);
    h.drag(
        MouseButton::Left,
        (x + 3, y),
        (x + 3, y + 4),
        KeyModifiers::NONE,
    );
    let after = h.locate(title.trim());
    assert_eq!(after, (before.0, before.1 + 4), "the prompt moved");
    assert_eq!(h.locate(&existing), under, "the copy dialog did not");
}

#[test]
fn the_screens_list_over_dialogs_brings_them_all_back() {
    let mut h = mask_over_copy();
    h.dispatch(Action::ScreensList);
    assert_eq!(h.state.dialogs.depth(), 3);
    h.assert_screen(&t("screens_menu_title"));
    h.keys("Esc");
    assert_eq!(h.state.dialogs.depth(), 2);
    assert!(h.state.dialogs.top_as::<CopyMoveFilterPrompt>().is_some());
}

#[test]
fn a_message_over_a_dialog_returns_to_it() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    h.state.dialogs.push(PopupType::error("disk full"));
    h.assert_screen("disk full");
    h.assert_screen(&t("prompt_mkdir_title"));
    h.keys("Enter");
    assert_eq!(h.state.dialogs.depth(), 1);
    h.assert_no_screen("disk full");
}

#[test]
fn every_question_paints_its_title() {
    let questions: [(PopupType, &str); 5] = [
        (ConfirmQuit.into(), "prompt_exit_title"),
        (ConfirmInterrupt.into(), "prompt_abort_title"),
        (ConfirmReload.into(), "prompt_reload_title"),
        (SaveSetupConfirm.into(), "prompt_save_setup_title"),
        (
            ConfirmClearHistory {
                history_type: "folder".into(),
            }
            .into(),
            "prompt_clear_history_title",
        ),
    ];
    let mut h = Harness::new();
    for (question, title_key) in questions {
        h.state.dialogs.replace(question);
        h.assert_screen(t(title_key).trim());
    }
    // The history kind is named in its own words.
    h.assert_screen(&t("history_type_folder"));
}

#[test]
fn plugin_dialogs_paint_what_the_plugin_asked() {
    let mut h = Harness::new();
    h.state.dialogs.replace(PluginDialog::Confirm {
        title: "Deploy".into(),
        msg: "Push to production?".into(),
        cursor_idx: 0,
        position: None,
    });
    h.assert_screen("Deploy");
    h.assert_screen("Push to production?");

    h.state.dialogs.replace(PluginDialog::Input {
        title: "Token".into(),
        input: "secret".into(),
        obscure: true,
        position: None,
    });
    h.assert_screen("Token");
    h.assert_no_screen("secret");

    h.state.dialogs.replace(PluginDialog::Which {
        candidates: vec![crate::plugin::manager::WhichCandidate {
            on: vec!["g".into()],
            desc: Some("go home".into()),
        }],
        silent: false,
        position: None,
    });
    h.assert_screen("go home");
}
