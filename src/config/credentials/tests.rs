use super::index::{INDEX_ACCOUNT, LEGACY_INDEX_ACCOUNT, NAME_INDEX_ACCOUNT};
use super::*;
use crate::config::settings::RemoteKind;
use crate::credential_prompt::Secret;

fn reset() {
    test_store::set_available(true);
    test_store::set_reads_fail(false);
}

/// The keyring account of the connection with id `id`.
fn by_id(id: &str) -> String {
    format!("connection-id:{id}")
}

fn connection(name: &str, id: &str, password: Option<&str>) -> Connection {
    Connection {
        name: name.into(),
        id: id.into(),
        host: "h".into(),
        port: "22".into(),
        username: "u".into(),
        password: password.map(|p| Secret::new(p.to_owned())),
        ..Connection::default()
    }
}

fn settings_with(password: Option<&str>) -> Settings {
    let mut settings = Settings::default();
    settings.connections.push(connection("box", "b1", password));
    settings
}

#[test]
fn plaintext_passwords_migrate_to_the_keyring() {
    reset();
    let mut settings = settings_with(Some("s3cret"));
    let outcome = load_passwords(&mut settings);
    assert!(outcome.migrated && !outcome.not_persisted);
    assert_eq!(test_store::secret(&by_id("b1")).as_deref(), Some("s3cret"));
    assert_eq!(settings.connections[0].secret(), Some("s3cret"));
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
    assert_eq!(loaded.connections[0].secret(), Some("pw"));
    let _ = for_disk(&settings_with(None));
    assert!(test_store::secret(&by_id("b1")).is_none());
}

#[test]
fn connections_with_the_same_name_keep_their_own_password() {
    reset();
    let mut settings = Settings::default();
    settings
        .connections
        .push(connection("box", "", Some("one")));
    settings
        .connections
        .push(connection("box", "", Some("two")));
    assert!(load_passwords(&mut settings).migrated);
    let ids: Vec<String> = settings.connections.iter().map(|c| c.id.clone()).collect();
    assert!(!ids[0].is_empty() && ids[0] != ids[1], "{ids:?}");
    let disk = for_disk(&settings);
    let mut loaded = disk.clone();
    load_passwords(&mut loaded);
    assert_eq!(loaded.connections[0].secret(), Some("one"));
    assert_eq!(loaded.connections[1].secret(), Some("two"));
}

#[test]
fn a_copied_id_is_replaced() {
    let mut settings = Settings::default();
    settings.connections.push(connection("a", "same", None));
    settings.connections.push(connection("b", "same", None));
    assert!(assign_ids(&mut settings));
    assert_eq!(settings.connections[0].id, "same");
    assert_ne!(settings.connections[1].id, "same");
    assert!(!assign_ids(&mut settings), "stable once assigned");
}

#[test]
fn removed_connections_lose_their_entry_and_renamed_ones_keep_it() {
    reset();
    let mut settings = settings_with(Some("pw"));
    let mut other = connection("other", "o1", Some("pw"));
    other.kind = RemoteKind::Ftp;
    settings.connections.push(other);
    let _ = for_disk(&settings);
    assert!(test_store::secret(&by_id("o1")).is_some());

    settings.connections[0].name = "renamed".into();
    settings.connections.pop();
    let _ = for_disk(&settings);
    assert!(test_store::secret(&by_id("o1")).is_none(), "removed");
    assert_eq!(
        test_store::secret(&by_id("b1")).as_deref(),
        Some("pw"),
        "a rename keeps the id"
    );

    settings.connections.clear();
    let _ = for_disk(&settings);
    assert!(test_store::secret(&by_id("b1")).is_none());
    assert!(test_store::secret(INDEX_ACCOUNT).is_none());
}

#[test]
fn name_keyed_entries_move_to_the_id_and_stale_ones_are_pruned() {
    reset();
    test_store::put("connection:box", "pw");
    test_store::put("connection:gone", "stale");
    test_store::put(NAME_INDEX_ACCOUNT, r#"["box","gone"]"#);
    let mut settings = settings_with(None);
    load_passwords(&mut settings);
    assert_eq!(settings.connections[0].secret(), Some("pw"));
    assert_eq!(test_store::secret(&by_id("b1")).as_deref(), Some("pw"));
    assert!(test_store::secret("connection:box").is_none(), "moved");
    assert!(test_store::secret(NAME_INDEX_ACCOUNT).is_none(), "folded");
    let _ = for_disk(&settings);
    assert!(test_store::secret("connection:gone").is_none(), "pruned");
    assert_eq!(test_store::secret(&by_id("b1")).as_deref(), Some("pw"));
}

#[test]
fn ssh_preset_entries_of_older_releases_move_to_connection_accounts() {
    reset();
    test_store::put("ssh-preset:box", "old-pw");
    test_store::put("ssh-preset:gone", "stale");
    test_store::put(LEGACY_INDEX_ACCOUNT, r#"["box","gone"]"#);
    let mut settings = settings_with(None);
    load_passwords(&mut settings);
    assert_eq!(settings.connections[0].secret(), Some("old-pw"));
    assert_eq!(test_store::secret(&by_id("b1")).as_deref(), Some("old-pw"));
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
    assert_eq!(conn.secret(), Some("pw"));
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
    assert_eq!(for_disk(&settings).connections[0].secret(), Some("pw"));
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
