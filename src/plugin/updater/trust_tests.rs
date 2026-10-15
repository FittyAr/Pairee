use super::*;

// Fixtures made once with a throwaway keypair (the secret keys were
// discarded): KEY_A plays the pinned registry key, KEY_B another key.
const KEY_A: &str = "RWSfhsXKBPIn8EbhO/44TbULSXpYTHCmtzEr5UQ63Bda4KTU4T/16+jj";
const KEY_B: &str = "RWQ/4/lbqfpvEQNCB2kvDSyG6Esmbk8/D1lfhvh/JTQiSC/uSbX/wmjL";

const INDEX: &[u8] = b"[plugins.demo]\nversion = \"1.0.0\"\n";
const OLD_INDEX: &[u8] = b"[plugins.demo]\nversion = \"0.9.0\"\n";
const MANIFEST: &[u8] = b"name = \"demo\"\n";
const MANIFEST_PATH: &str = "plugins/a/alice/demo/manifest.toml";

/// `INDEX`, key A, `serial:5 timestamp:1760000000`.
const INDEX_S5: &str = "untrusted comment: signature from minisign secret key
RUSfhsXKBPIn8BxbhIVUS57W/L3tKvPmClih2EX96NPGhpfLABDnca5Ug0M+F9Ct8tgTMJAFHqhBMTnxG6G04vhnqUUjQ4VEwQs=
trusted comment: file:index.toml serial:5 timestamp:1760000000
RAuxS8VrzObI6k3vOH0mEUgO2YLG+F4e8dS/aGQbmjFqFI62Xc72qWFy6ts/+DXrSFNsH6TFjtcT2bWZqceSDQ==
";
/// `OLD_INDEX`, key A, `serial:4 timestamp:1759000000`.
const INDEX_S4_OLD: &str = "untrusted comment: signature from minisign secret key
RUSfhsXKBPIn8CC7/xRyKeYlfCE+eemnSeweU/e1mPxPgR9yXPcM/lr7NR7C7qbtVfUUdvfEM6xsEeZlo/A+rK50VETkCSSdYQM=
trusted comment: file:index.toml serial:4 timestamp:1759000000
JJjHgM3LiDavFDaO5kBCqKn/WvLjuo6bhoDc/7f9vVB2jxUkOaJYXVZzhM3XswsRA/cQF+IrZdcAx/sqDsXoBg==
";
/// `INDEX`, key B, `serial:5 timestamp:1760000000`.
const INDEX_S5_KEY_B: &str = "untrusted comment: signature from minisign secret key
RUQ/4/lbqfpvEZONsCg/3WsouZf66ge1acFmR4rjAZ9kcO5f/CvMOi0g8voPajs8g1z/i4rJPMNCXPrE+VfsRuIy0UvPjeZ+RAI=
trusted comment: file:index.toml serial:5 timestamp:1760000000
k7U2SaoRjJEHZ0fMBNndYVgi1jd/+YXKWgSOd9MeLpLOSYfD0nBjf8+jWYkBWAuLB67NaztndsMNqSpEYtvTBA==
";
/// `INDEX`, key A, only `file:index.toml`.
const INDEX_NO_FRESHNESS: &str = "untrusted comment: signature from minisign secret key
RUSfhsXKBPIn8Ni/uLI7eC1Xx6MBtMtjr6cUT4h55yDQZEYjhGRo0oy5oZTX85csfKusEc5gjUx1Mhl7ELX+vi5PbFjJI5gXGgc=
trusted comment: file:index.toml
0nu7z5L2H5GwxRmNJ95pXNdZueTAeouR+baAtjAo19B0xBPy7Kusev95J1c396ILRH7prPeu/yONse5H+Z1tBA==
";
/// `MANIFEST`, key A, `file:plugins/a/alice/demo/manifest.toml`.
const MANIFEST_SIG: &str = "untrusted comment: signature from minisign secret key
RUSfhsXKBPIn8ELP2DHDcHZduipoMQTtV8oHlbMhjXBPSI5OOd6UoZhdtXZlQ4C2vkOS3g6LbSuz2IZN2NkidhmZ8aafIf9Tdg0=
trusted comment: file:plugins/a/alice/demo/manifest.toml
UvdhQLvMnrJxV1E/fVgvpXjTDiOWXvreO8Nl2bg8r2htG2/zDxJZ4rIE1REvX+GSogwYx+3zI7ijOVrNS+i9Bw==
";

/// When `INDEX_S5` was signed, and a day later.
const SIGNED_AT: u64 = 1_760_000_000;
const NOW: u64 = SIGNED_AT + SECONDS_PER_DAY;

/// Pinned key A, signatures required, the registry not yet declared signed,
/// no state file.
fn trust() -> RegistryTrust {
    RegistryTrust {
        public_key: KEY_A.to_string(),
        registry_signed: false,
        ..RegistryTrust::default()
    }
}

fn with_state(dir: &tempfile::TempDir) -> RegistryTrust {
    RegistryTrust {
        state_path: Some(dir.path().join(REGISTRY_STATE_FILE)),
        ..trust()
    }
}

fn verify(
    t: &RegistryTrust,
    data: &[u8],
    sig: Option<&str>,
    name: &str,
) -> Result<SignatureStatus> {
    t.verify_at(data, sig, name, NOW)
}

#[test]
fn first_fetch_is_checked_against_the_pinned_key() {
    // No state at all: a fresh installation accepts the index only because
    // it verifies with the pinned key, and records its serial.
    let dir = tempfile::tempdir().unwrap();
    let t = with_state(&dir);
    assert_eq!(
        verify(&t, INDEX, Some(INDEX_S5), "index.toml").unwrap(),
        SignatureStatus::Verified
    );
    let state = RegistryState::read_from(t.state_path.as_deref().unwrap()).unwrap();
    assert_eq!(state.key, KEY_A);
    assert_eq!(
        state.files.get("index.toml"),
        Some(&Freshness {
            serial: 5,
            timestamp: SIGNED_AT
        })
    );
    // Manifests carry no freshness and need none.
    assert_eq!(
        verify(&t, MANIFEST, Some(MANIFEST_SIG), MANIFEST_PATH).unwrap(),
        SignatureStatus::Verified
    );
}

#[test]
fn a_signature_by_another_key_is_rejected_on_the_first_fetch() {
    let dir = tempfile::tempdir().unwrap();
    let t = with_state(&dir);
    let err = verify(&t, INDEX, Some(INDEX_S5_KEY_B), "index.toml").unwrap_err();
    assert!(err.to_string().contains("invalid"), "{err:#}");
    // Nothing was learned from it.
    assert!(RegistryState::read_from(t.state_path.as_deref().unwrap()).is_none());
    // Not even when signatures are not required.
    let lax = RegistryTrust {
        require_signature: false,
        ..trust()
    };
    assert!(verify(&lax, INDEX, Some(INDEX_S5_KEY_B), "index.toml").is_err());
    // The release key pinned by default did not sign the fixtures either.
    assert!(
        verify(
            &RegistryTrust::default(),
            INDEX,
            Some(INDEX_S5),
            "index.toml"
        )
        .is_err()
    );
}

#[test]
fn tampered_data_and_misnamed_files_are_rejected() {
    assert!(verify(&trust(), OLD_INDEX, Some(INDEX_S5), "index.toml").is_err());
    assert!(verify(&trust(), INDEX, Some(INDEX_S5), "blocklist.toml").is_err());
    assert!(verify(&trust(), INDEX, Some("garbage"), "index.toml").is_err());
}

#[test]
fn a_replayed_older_serial_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let t = with_state(&dir);
    verify(&t, INDEX, Some(INDEX_S5), "index.toml").unwrap();
    let err = verify(&t, OLD_INDEX, Some(INDEX_S4_OLD), "index.toml").unwrap_err();
    assert!(err.to_string().contains("4 < 5"), "{err:#}");
    // Also for a new process, and with the age limit off.
    let fresh_process = RegistryTrust {
        max_age_days: 0,
        ..with_state(&dir)
    };
    assert!(verify(&fresh_process, OLD_INDEX, Some(INDEX_S4_OLD), "index.toml").is_err());
    // The same serial again is fine (a refetch).
    verify(&t, INDEX, Some(INDEX_S5), "index.toml").unwrap();
    // The serial recorded never goes down.
    let state = RegistryState::read_from(t.state_path.as_deref().unwrap()).unwrap();
    assert_eq!(state.files["index.toml"].serial, 5);
}

#[test]
fn serials_are_tracked_per_file_and_per_key() {
    let dir = tempfile::tempdir().unwrap();
    let t = with_state(&dir);
    verify(&t, INDEX, Some(INDEX_S5), "index.toml").unwrap();
    // Another configured key starts over (its registry has its own serials).
    let other = RegistryState::default()
        .for_key(KEY_A)
        .accept(
            "index.toml",
            Some(Freshness {
                serial: 9,
                timestamp: 1,
            }),
        )
        .for_key(KEY_B);
    assert!(other.files.is_empty());
    assert_eq!(other.key, KEY_B);
    // An older accept does not lower the record.
    let kept = RegistryState::default()
        .accept(
            "index.toml",
            Some(Freshness {
                serial: 9,
                timestamp: 9,
            }),
        )
        .accept(
            "index.toml",
            Some(Freshness {
                serial: 3,
                timestamp: 3,
            }),
        );
    assert_eq!(kept.files["index.toml"].serial, 9);
}

#[test]
fn a_stale_signature_is_rejected_unless_the_limit_is_raised_or_off() {
    let t = trust();
    let late = SIGNED_AT + 31 * SECONDS_PER_DAY;
    let err = t
        .verify_at(INDEX, Some(INDEX_S5), "index.toml", late)
        .unwrap_err();
    assert!(err.to_string().contains("registry_max_age_days"), "{err:#}");
    assert!(err.to_string().contains("31"), "{err:#}");
    // 30 days old is still within the default limit.
    let edge = SIGNED_AT + 30 * SECONDS_PER_DAY;
    t.verify_at(INDEX, Some(INDEX_S5), "index.toml", edge)
        .unwrap();
    for max_age_days in [0, 60] {
        let relaxed = RegistryTrust {
            max_age_days,
            ..trust()
        };
        relaxed
            .verify_at(INDEX, Some(INDEX_S5), "index.toml", late)
            .unwrap();
    }
}

#[test]
fn a_signed_index_without_freshness_is_rejected() {
    let err = verify(&trust(), INDEX, Some(INDEX_NO_FRESHNESS), "index.toml").unwrap_err();
    assert!(err.to_string().contains("serial"), "{err:#}");
}

#[test]
fn unsigned_files_are_accepted_until_the_registry_is_seen_signed() {
    let dir = tempfile::tempdir().unwrap();
    let strict = with_state(&dir);
    assert_eq!(
        verify(&strict, INDEX, None, "index.toml").unwrap(),
        SignatureStatus::Unsigned
    );
    verify(&strict, MANIFEST, Some(MANIFEST_SIG), MANIFEST_PATH).unwrap();
    // Signatures stripped later: refused, also by a new process.
    assert!(verify(&strict, INDEX, None, "index.toml").is_err());
    assert!(verify(&with_state(&dir), INDEX, None, "index.toml").is_err());
    // Opted out: only a warning.
    let lax = RegistryTrust {
        require_signature: false,
        ..with_state(&dir)
    };
    assert_eq!(
        verify(&lax, INDEX, None, "index.toml").unwrap(),
        SignatureStatus::Unsigned
    );
}

#[test]
fn a_registry_declared_signed_refuses_unsigned_files_from_the_first_fetch() {
    let signed = RegistryTrust {
        registry_signed: true,
        ..trust()
    };
    let err = verify(&signed, INDEX, None, "index.toml").unwrap_err();
    assert!(
        err.to_string().contains("registry_require_signature"),
        "{err:#}"
    );
    let lax = RegistryTrust {
        require_signature: false,
        ..signed
    };
    assert_eq!(
        verify(&lax, INDEX, None, "index.toml").unwrap(),
        SignatureStatus::Unsigned
    );
}

#[test]
fn another_registry_key_starts_unsigned_again() {
    let dir = tempfile::tempdir().unwrap();
    verify(
        &with_state(&dir),
        MANIFEST,
        Some(MANIFEST_SIG),
        MANIFEST_PATH,
    )
    .unwrap();
    let other = RegistryTrust {
        public_key: KEY_B.to_string(),
        ..with_state(&dir)
    };
    assert_eq!(
        verify(&other, INDEX, None, "index.toml").unwrap(),
        SignatureStatus::Unsigned
    );
}

#[test]
fn settings_override_the_pinned_key_and_the_limits() {
    let mut settings = Settings::default();
    let pinned = RegistryTrust::from_settings(&settings);
    assert_eq!(pinned.public_key, RegistryTrust::default().public_key);
    assert!(pinned.require_signature);
    assert_eq!(pinned.registry_signed, REGISTRY_SIGNED);
    assert_eq!(pinned.max_age_days, 30);
    assert!(pinned.state_path.is_some());
    settings.security.registry_public_key = format!(" {KEY_A} ");
    settings.security.registry_require_signature = false;
    settings.security.registry_max_age_days = 0;
    let custom = RegistryTrust::from_settings(&settings);
    assert_eq!(
        custom,
        RegistryTrust {
            require_signature: false,
            registry_signed: REGISTRY_SIGNED,
            max_age_days: 0,
            state_path: pinned.state_path,
            ..trust()
        }
    );
}
