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
