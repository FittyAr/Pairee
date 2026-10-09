use super::*;
use crate::config::AppConfig;
use crate::config::settings::TabExpansion;

fn draft() -> ConfigDraft {
    ConfigDraft::new(&AppConfig::default())
}

fn ctx_rows(tab: usize, settings: &ConfigDraft) -> Vec<Row> {
    tab_rows(
        tab,
        &RowCtx {
            settings,
            keybindings: &Default::default(),
        },
    )
}

/// Activates the setting row whose label key is `key` on `tab`.
fn activate(tab: usize, key: &str, settings: &mut ConfigDraft) -> Activation {
    let rows = ctx_rows(tab, settings);
    let setting = rows
        .iter()
        .filter_map(Row::setting)
        .find(|s| matches!(s.label, Label::Key(k) if k == key))
        .unwrap_or_else(|| panic!("no row {key} on tab {tab}"))
        .clone();
    setting.activate(settings, &AppContext::new(AppConfig::default()))
}

#[test]
fn toggles_flip_their_flags_and_render_checkboxes() {
    let mut s = draft();
    assert!(s.ssh_enabled);
    activate(0, "feature_ssh", &mut s);
    assert!(!s.ssh_enabled);
    activate(0, "feature_plugins", &mut s);
    assert!(!s.plugins_enabled);
    let rows = ctx_rows(0, &s);
    let ssh = rows
        .iter()
        .filter_map(Row::setting)
        .find(|r| matches!(r.label, Label::Key("feature_ssh")))
        .unwrap();
    assert!(ssh.text(&s, None).starts_with("[ ] "));
}

#[test]
fn cycles_and_nested_fields() {
    let mut s = draft();
    activate(5, "ed_expand_tabs", &mut s);
    assert_eq!(s.editor_expand_tabs, TabExpansion::NewTabs);
    s.editor_tab_size = 8;
    activate(5, "ed_tab_size", &mut s);
    assert_eq!(s.editor_tab_size, 2);
    let before = s.confirmations.confirm_quit;
    activate(3, "conf_exit", &mut s);
    assert_ne!(s.confirmations.confirm_quit, before);
}

#[test]
fn edit_rows_start_editing_and_commit() {
    let mut s = draft();
    let Activation::StartEdit(field) = activate(7, "git_log_limit", &mut s) else {
        panic!("expected edit");
    };
    assert_eq!(field.text(), s.git_log_limit.to_string());
    let rows = ctx_rows(7, &s);
    let limit = rows
        .iter()
        .filter_map(Row::setting)
        .find(|r| matches!(r.label, Label::Key("git_log_limit")))
        .unwrap();
    limit.commit_edit(&mut s, "999999");
    assert_eq!(s.git_log_limit, 10_000);
    assert!(limit.text(&s, Some(&TextField::new("12"))).ends_with("12█"));
}

#[test]
fn view_keymap_issues_opens_info_panel() {
    let mut s = draft();
    match activate(2, "int_keymap_view", &mut s) {
        Activation::Open(popup) => {
            let PopupType::InfoPanel { lines } = *popup else {
                panic!("expected InfoPanel");
            };
            assert!(
                lines
                    .iter()
                    .any(|l| l.contains("Gray+") || l.contains("Plus"))
            );
        }
        _ => panic!("expected InfoPanel"),
    }
}

#[test]
fn keymap_section_has_status_and_view_rows() {
    let rows = ctx_rows(2, &draft());
    assert!(rows.iter().any(|r| matches!(
        r,
        Row::Hint(Label::Owned(_)) | Row::Subtitle(Label::Owned(_))
    )));
}

#[test]
fn plugin_path_row_only_in_developer_mode() {
    let mut s = draft();
    s.plugins_developer_mode = false;
    let count = ctx_rows(4, &s).iter().filter(|r| r.is_selectable()).count();
    s.plugins_developer_mode = true;
    assert_eq!(
        ctx_rows(4, &s).iter().filter(|r| r.is_selectable()).count(),
        count + 1
    );
}

#[test]
fn keymap_cycle_edits_the_draft_preset() {
    let mut s = draft();
    assert_eq!(s.keymap_preset, "norton");
    activate(2, "int_keybindings", &mut s);
    assert_eq!(s.keymap_preset, "standard");
}
