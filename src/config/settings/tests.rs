use super::Settings;

#[test]
fn feature_flags_default_on() {
    let s = Settings::default();
    assert!(s.git_enabled);
    assert!(s.ssh_enabled);
    assert!(s.plugins_enabled);
    assert!(s.image_preview_enabled);
}

#[test]
fn feature_flags_missing_fields_stay_on() {
    let mut table: toml::Table =
        toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
    table.remove("ssh_enabled");
    table.remove("plugins_enabled");
    table.remove("image_preview_enabled");
    table.remove("git_enabled");
    let loaded: Settings = toml::from_str(&toml::to_string(&table).unwrap()).unwrap();
    assert!(loaded.ssh_enabled);
    assert!(loaded.plugins_enabled);
    assert!(loaded.image_preview_enabled);
    assert!(loaded.git_enabled);
}

#[test]
fn removed_external_editor_keys_are_ignored() {
    let mut table: toml::Table =
        toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
    table.insert("default_editor".into(), "vim".into());
    table.insert("editor_use_external".into(), true.into());
    // A sample of other options removed because they had no effect.
    table.insert("viewer_command".into(), "less %f".into());
    table.insert("editor_default_codepage".into(), "1252".into());
    table.insert("interface_screen_saver_minutes".into(), 5.into());
    table.insert("transfer_engine_enabled".into(), true.into());
    if let Some(toml::Value::Table(conf)) = table.get_mut("confirmations") {
        conf.insert("confirm_overwrite".into(), true.into());
    }
    let loaded: Result<Settings, _> = toml::from_str(&toml::to_string(&table).unwrap());
    assert!(loaded.is_ok(), "old config.toml must still load");
}

#[test]
fn expand_tabs_reads_old_labels_and_tolerates_unknown() {
    use super::TabExpansion;
    let parse = |v: &str| -> TabExpansion {
        let mut table: toml::Table =
            toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
        table.insert("editor_expand_tabs".into(), v.into());
        let s: Settings = toml::from_str(&toml::to_string(&table).unwrap()).unwrap();
        s.editor_expand_tabs
    };
    assert_eq!(parse("Do not expand tabs"), TabExpansion::Keep);
    assert_eq!(
        parse("Expand newly entered tabs to spaces"),
        TabExpansion::NewTabs
    );
    assert_eq!(
        parse("Convert all tabs to spaces"),
        TabExpansion::ConvertAll
    );
    assert_eq!(parse("something else"), TabExpansion::Keep);
}

#[test]
fn new_install_default_needs_onboarding() {
    assert!(!Settings::default().onboarding_completed);
}

#[test]
fn existing_config_without_field_skips_onboarding() {
    let mut table: toml::Table =
        toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
    table.remove("onboarding_completed");
    let stripped = toml::to_string(&table).unwrap();
    let loaded: Settings = toml::from_str(&stripped).unwrap();
    assert!(
        loaded.onboarding_completed,
        "missing field must mean already onboarded"
    );
}
