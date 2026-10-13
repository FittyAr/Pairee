use super::index::{INDEX_ACCOUNT, LEGACY_INDEX_ACCOUNT};
use super::*;
use crate::config::settings::RemoteKind;

fn reset() {
    test_store::set_available(true);
    test_store::set_reads_fail(false);
}

fn settings_with(password: Option<&str>) -> Settings {
    let mut settings = Settings::default();
    settings.connections.push(Connection {
        name: "box".into(),
        host: "h".into(),
        port: "22".into(),
        username: "u".into(),
        password: password.map(str::to_owned),
        ..Connection::default()
    });
    settings
}

#[test]
fn plaintext_passwords_migrate_to_the_keyring() {
    reset();
    let mut settings = settings_with(Some("s3cret"));
    let outcome = load_passwords(&mut settings);
    assert!(outcome.migrated && !outcome.not_persisted);
    assert_eq!(
        test_store::secret("connection:box").as_deref(),
        Some("s3cret")
    );
    assert_eq!(settings.connections[0].password.as_deref(), Some("s3cret"));
    let disk = for_disk(&settings);
    assert!(
        disk.connections[0].password.is_none(),
        "never written to the file"
    );
    let toml = toml::to_string(&disk).unwrap();
    assert!(!toml.contains("s3cret"));
}

#[test]
fn passwords_are_read_back_and_cleared_ones_deleted() {
    reset();
    let saved = for_disk(&settings_with(Some("pw")));
    let mut loaded = saved.clone();
    assert_eq!(load_passwords(&mut loaded), LoadOutcome::default());
    assert_eq!(loaded.connections[0].password.as_deref(), Some("pw"));
    let _ = for_disk(&settings_with(None));
    assert!(test_store::secret("connection:box").is_none());
}

#[test]
fn removed_and_renamed_connections_lose_their_keyring_entry() {
    reset();
    let mut settings = settings_with(Some("pw"));
    let mut other = settings.connections[0].clone();
    other.name = "other".into();
    other.kind = RemoteKind::Ftp;
    settings.connections.push(other);
    let _ = for_disk(&settings);
    assert!(test_store::secret("connection:other").is_some());

    settings.connections[0].name = "renamed".into();
    settings.connections.pop();
    let _ = for_disk(&settings);
    assert!(test_store::secret("connection:box").is_none(), "renamed");
    assert!(test_store::secret("connection:other").is_none(), "removed");
    assert_eq!(
        test_store::secret("connection:renamed").as_deref(),
        Some("pw")
    );

    settings.connections.clear();
    let _ = for_disk(&settings);
    assert!(test_store::secret("connection:renamed").is_none());
    assert!(test_store::secret(INDEX_ACCOUNT).is_none());
}

#[test]
fn entries_from_before_the_index_are_pruned_once_loaded() {
    reset();
    test_store::put("connection:box", "pw");
    let mut settings = settings_with(None);
    load_passwords(&mut settings);
    assert_eq!(settings.connections[0].password.as_deref(), Some("pw"));
    settings.connections.clear();
    let _ = for_disk(&settings);
    assert!(test_store::secret("connection:box").is_none());
}

#[test]
fn ssh_preset_entries_of_older_releases_move_to_connection_accounts() {
    reset();
    test_store::put("ssh-preset:box", "old-pw");
    test_store::put("ssh-preset:gone", "stale");
    test_store::put(LEGACY_INDEX_ACCOUNT, r#"["box","gone"]"#);
    let mut settings = settings_with(None);
    load_passwords(&mut settings);
    assert_eq!(settings.connections[0].password.as_deref(), Some("old-pw"));
    assert_eq!(
        test_store::secret("connection:box").as_deref(),
        Some("old-pw")
    );
    for gone in ["ssh-preset:box", "ssh-preset:gone", LEGACY_INDEX_ACCOUNT] {
        assert!(test_store::secret(gone).is_none(), "{gone}");
    }
}

/// A transient read failure must never cost the user the stored password.
#[test]
fn an_unreadable_keyring_never_deletes_the_entry_on_save() {
    reset();
    test_store::put("connection:box", "pw");
    test_store::set_reads_fail(true);
    let mut settings = settings_with(None);
    load_passwords(&mut settings);
    assert!(settings.connections[0].password.is_none());
    assert!(settings.connections[0].secret_unread);

    let disk = for_disk(&settings);
    assert!(disk.connections[0].password.is_none());
    assert_eq!(
        test_store::secret("connection:box").as_deref(),
        Some("pw"),
        "kept although the password was unknown at save time"
    );

    // Once the keyring answers again the password comes back.
    test_store::set_reads_fail(false);
    let mut conn = settings.connections[0].clone();
    retry_unread(&mut conn);
    assert_eq!(conn.password.as_deref(), Some("pw"));
    assert!(!conn.secret_unread);
}

#[test]
fn without_keyring_plaintext_needs_the_opt_in() {
    reset();
    test_store::set_available(false);
    let mut settings = settings_with(Some("pw"));
    let outcome = load_passwords(&mut settings);
    assert!(outcome.not_persisted);
    assert!(for_disk(&settings).connections[0].password.is_none());

    settings.security.passwords_in_config = true;
    assert!(!load_passwords(&mut settings).not_persisted);
    assert_eq!(
        for_disk(&settings).connections[0].password.as_deref(),
        Some("pw")
    );
    reset();
}

#[test]
fn ssh_presets_of_older_config_files_load_as_sftp_connections() {
    let mut table: toml::Table =
        toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
    table.remove("connections");
    let old: toml::Table = toml::from_str(
        r#"
        [[ssh_presets]]
        name = "web"
        host = "example.com"
        port = "2222"
        username = "me"
        password = "pw"
        key_path = "~/.ssh/id"

        [security]
        ssh_passwords_in_config = true
    "#,
    )
    .unwrap();
    table.extend(old);
    let settings: Settings = toml::from_str(&toml::to_string(&table).unwrap()).unwrap();
    let conn = &settings.connections[0];
    assert_eq!(conn.kind, RemoteKind::Sftp);
    assert_eq!(conn.port_number(), Some(2222));
    assert!(settings.security.passwords_in_config);
    assert!(conn.matches(RemoteKind::Sftp, "example.com", Some(2222), "me"));
    let saved = toml::to_string(&settings).unwrap();
    assert!(saved.contains("[[connections]]") && !saved.contains("ssh_presets"));
}
