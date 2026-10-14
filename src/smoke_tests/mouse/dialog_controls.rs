//! Dialog controls driven through the dialog's own keys: option rows,
//! buttons and text fields.

use super::top;
use crate::app::state::PopupType;
use crate::app::state::popup::FilePanelFilterPrompt;
use crate::app::state::popup::MkDirPrompt;
use crate::app::state::popup::RenamePrompt;
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;

/// Cell of the first option row (a control that ends pressing Space) of
/// the open dialog.
fn option_row(h: &mut Harness) -> (u16, u16) {
    let (x, y) = h.hit_cell(|t| {
        matches!(t, HitTarget::PopupKeys(keys)
            if keys.last().map(|k| k.code) == Some(crossterm::event::KeyCode::Char(' ')))
    });
    (x + 1, y)
}

/// Cell of the first dialog control showing `label` on screen.
fn control(h: &mut Harness, label: &str) -> (u16, u16) {
    h.render();
    let (x, y) = h.locate(label);
    (x + 1, y)
}

#[test]
fn form_controls_toggle_type_and_press_buttons() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    let multiple = |h: &Harness| match top(h) {
        Some(PopupType::MkDirPrompt(MkDirPrompt {
            process_multiple, ..
        })) => *process_multiple,
        other => panic!("mkdir closed: {other:?}"),
    };
    let before = multiple(&h);
    let (x, y) = option_row(&mut h);
    h.click(x, y);
    assert_ne!(multiple(&h), before, "the checkbox row toggles");
    // Back in the field, typing goes to it; then OK creates the folder.
    let (fx, fy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    h.click(fx + 10, fy);
    h.text("made_by_mouse");
    let ok = crate::config::localization::t("btn_ok_bracket");
    let (x, y) = control(&mut h, &ok);
    h.click(x, y);
    assert!(top(&h).is_none(), "{}", h.screen_text());
    assert!(h.exists("work/left/made_by_mouse"));
}

#[test]
fn a_cancel_button_closes_its_form() {
    let mut h = Harness::new();
    h.keys("@mkdir");
    let cancel = crate::config::localization::t("btn_cancel_bracket");
    let (x, y) = control(&mut h, &cancel);
    h.click(x, y);
    assert!(top(&h).is_none());
}

#[test]
fn clicking_in_a_field_moves_its_cursor() {
    let mut h = Harness::new();
    h.write("work/left/report.txt", "r");
    h.reread();
    h.focus("report.txt").keys("@rename");
    let (fx, fy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    let text_x = match h.state.hits.at(fx, fy).map(|hit| hit.target) {
        Some(HitTarget::PopupField { field, .. }) => field.text_x,
        other => panic!("{other:?}"),
    };
    h.click(text_x + 3, fy);
    let cursor = match top(&h) {
        Some(PopupType::RenamePrompt(RenamePrompt { input, .. })) => input.cursor(),
        other => panic!("{other:?}"),
    };
    assert_eq!(cursor, 3);
}

/// A field longer than the dialog scrolls sideways so the cursor stays in
/// view, and a click maps through that scroll and through wide characters.
#[test]
fn long_fields_scroll_and_clicks_follow_the_scroll() {
    let mut h = Harness::new();
    h.keys("@file_panel_filter");
    // 80 x, a wide 日, then "yEND": far wider than the 50-column prompt.
    h.text(&format!("{}日yEND", "x".repeat(80)));
    let (_, fy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    let (yx, y_row) = h.locate("yEND");
    assert_eq!(
        y_row, fy,
        "the end of the text (and the cursor) stay in view"
    );
    let cursor = |h: &Harness| match top(h) {
        Some(PopupType::FilePanelFilterPrompt(p)) => {
            p.input.text()[..p.input.cursor()].chars().count()
        }
        other => panic!("{other:?}"),
    };
    // The second cell of 日 (just left of "y"): the cursor goes before it.
    h.click(yx - 1, fy);
    assert_eq!(cursor(&h), 80, "before 日");
    // The view did not move (the cursor stayed visible), so "E" is where
    // it was.
    h.click(yx + 1, fy);
    assert_eq!(cursor(&h), 82, "before END");
}

#[test]
fn copy_dialog_option_rows_toggle_on_click() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.reread();
    h.focus("a.txt").keys("@copy");
    let flags = |h: &Harness| match top(h) {
        Some(PopupType::TransferPrompt(p)) => (
            p.already_existing,
            p.flags().into_iter().map(|(on, _)| on).collect::<Vec<_>>(),
        ),
        other => panic!("{other:?}"),
    };
    let before = flags(&h);
    let (x, y) = option_row(&mut h);
    h.click(x, y);
    assert_ne!(flags(&h), before, "{}", h.screen_text());
}

#[test]
fn configuration_rows_toggle_and_tabs_switch_on_click() {
    let mut h = Harness::new();
    h.keys("@system_settings");
    let dialog = |h: &Harness| match top(h) {
        Some(PopupType::ConfigurationDialog(d)) => (d.active_tab, d.draft.clone()),
        other => panic!("{other:?}"),
    };
    // The Interface tab, through its title on the left.
    let title = crate::config::localization::t("tab_interface");
    let clean = crate::ui::hotkey::parse_hotkey(&title).clean_text;
    let (x, y) = control(&mut h, &clean);
    h.click(x, y);
    assert_eq!(dialog(&h).0, 2);
    let label = crate::config::localization::t("int_clock");
    let before = dialog(&h).1.interface_clock;
    let (x, y) = control(&mut h, &label);
    h.click(x, y);
    assert_ne!(dialog(&h).1.interface_clock, before, "{}", h.screen_text());
}

/// Every dialog the actions below open must record its body (so clicks
/// inside never count as outside) and, unless it is plain text, at least
/// one control the mouse can use.
#[test]
fn every_dialog_records_its_body_and_its_controls() {
    // (action, has controls)
    const DIALOGS: &[(&str, bool)] = &[
        ("about", false),
        ("help", true),
        ("mkdir", true),
        ("rename", true),
        ("copy", true),
        ("move", true),
        ("delete", true),
        ("multi_rename_tool", true),
        ("create_link", false),
        ("file_attributes", true),
        ("describe_file", false),
        ("apply_command", false),
        ("compress_files", false),
        ("find_file", true),
        ("quick_filter", false),
        ("file_panel_filter", false),
        ("command_history", false),
        ("file_view_history", false),
        ("folders_history", false),
        ("sort_modes", true),
        ("context_menu", true),
        ("user_menu", true),
        ("hotlist", false),
        ("drive_select_left", true),
        ("tree_view", true),
        ("file_associations", false),
        ("task_list", false),
        ("plugin_menu", false), // plugins are off in the harness: an Info
        ("system_settings", true),
        ("command_palette", true),
        ("screens_list", true),
        ("sync_dirs", true),
        ("save_setup", false),
        ("rename_tab", false),
        ("which_key", true),
        ("ssh_connect", true),
    ];
    for (action, has_controls) in DIALOGS {
        let mut h = Harness::new();
        h.write("work/left/subject.txt", "x");
        h.reread();
        h.focus("subject.txt").keys(&format!("@{action}"));
        if top(&h).is_none() {
            panic!("@{action} opened no dialog:\n{}", h.screen_text());
        }
        h.render();
        let has =
            |pick: fn(&HitTarget) -> bool| h.state.hits.first(|t| pick(t).then_some(())).is_some();
        assert!(
            has(|t| *t == HitTarget::PopupBody),
            "@{action}: no dialog body"
        );
        if *has_controls {
            assert!(
                has(|t| matches!(
                    t,
                    HitTarget::PopupKeys(_)
                        | HitTarget::PopupRow { .. }
                        | HitTarget::PopupField { .. }
                )),
                "@{action}: no clickable control\n{}",
                h.screen_text()
            );
        }
    }
}

#[test]
fn clicking_in_a_one_field_prompt_moves_its_cursor() {
    let mut h = Harness::new();
    h.keys("@file_panel_filter").text("abcdef");
    let (fx, fy) = h.hit_cell(|t| matches!(t, HitTarget::PopupField { .. }));
    let text_x = match h.state.hits.at(fx, fy).map(|hit| hit.target) {
        Some(HitTarget::PopupField { field, .. }) => field.text_x,
        other => panic!("{other:?}"),
    };
    // The field shows where the hit says it starts.
    let row: String = h
        .screen_text()
        .lines()
        .nth(fy as usize)
        .unwrap()
        .chars()
        .skip(text_x as usize)
        .collect();
    assert!(row.starts_with("abcdef"), "{row:?}");
    h.click(text_x + 2, fy);
    let cursor = match top(&h) {
        Some(PopupType::FilePanelFilterPrompt(FilePanelFilterPrompt { input })) => input.cursor(),
        other => panic!("{other:?}"),
    };
    assert_eq!(cursor, 2);
}

#[test]
fn plugin_settings_rows_and_buttons_take_clicks() {
    use crate::app::state::popup::{PluginSettingValue, PluginSettingsDialog};
    use crate::plugin::runtime::settings::{Kind, SettingSpec};
    let spec = |key: &str, kind, default: &str| SettingSpec {
        key: key.into(),
        kind,
        default: Some(default.into()),
        description: Some("help".into()),
    };
    let mut h = Harness::new();
    h.state.dialogs.replace(PluginSettingsDialog::new(
        "demo",
        vec![
            spec("depth", Kind::Integer, "3"),
            spec("flag", Kind::Bool, "false"),
        ],
        &Default::default(),
    ));
    let flag = |h: &Harness| match h.state.dialogs.top_as::<PluginSettingsDialog>() {
        Some(d) => matches!(d.rows[1].value, PluginSettingValue::Toggle(true)),
        None => panic!("settings closed:\n{}", h.screen_text()),
    };
    h.render();
    assert!(h.screen_text().contains("depth:"), "{}", h.screen_text());
    let (x, y) = option_row(&mut h);
    h.click(x, y);
    assert!(flag(&h));
    let (x, y) = control(&mut h, "[ Defaults ]");
    h.click(x, y);
    assert!(!flag(&h));
    let (x, y) = control(&mut h, "[ Cancel ]");
    h.click(x, y);
    assert!(top(&h).is_none());
}
