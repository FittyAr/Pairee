#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TransferOptions {
    pub verify_after_copy: bool,
    pub hash_algorithm: HashAlgorithm,
    pub buffer_size: BufferSize,
    pub direct_io: bool,
    pub preserve_timestamps: bool,
    pub preserve_attributes: bool,
    pub preserve_acl: bool,
    pub preserve_streams: bool,
    pub skip_symlinks: bool,
    pub follow_symlinks: bool,
    pub max_retries: u32,
    pub conflict_resolution: String, // "ask", "overwrite", "skip", "rename", "overwrite_older"
    pub filter_mask: Option<String>,
    pub limit_bandwidth_rate: Option<u64>, // en bytes por segundo (opcional)
    pub auto_report: bool,
    pub report_format: String, // "html" o "csv"
    pub halt_on_error: bool,
    pub delete_to_recycle_bin: bool,
    /// Number of overwrite passes before a Delete actually
    /// removes the file (secure wipe). `0` means "no wipe, just
    /// delete". The worker currently implements 3 alternating
    /// passes: `0x00`, `0xFF`, `0x00` for `wipe_passes = 3`.
    /// Any other non-zero value is clamped to 3. Wipe is only
    /// applied when the destination is Local (SFTP cannot
    /// guarantee overwrite semantics on remote files).
    pub wipe_passes: u8,
}

impl Default for TransferOptions {
    fn default() -> Self {
        Self {
            verify_after_copy: false,
            hash_algorithm: HashAlgorithm::Blake3,
            buffer_size: BufferSize::_1MB,
            direct_io: false,
            preserve_timestamps: true,
            preserve_attributes: true,
            preserve_acl: false,
            preserve_streams: false,
            skip_symlinks: false,
            follow_symlinks: false,
            max_retries: 3,
            conflict_resolution: "ask".to_string(),
            filter_mask: None,
            limit_bandwidth_rate: None,
            auto_report: false,
            report_format: "html".to_string(),
            halt_on_error: false,
            delete_to_recycle_bin: false,
            wipe_passes: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HashAlgorithm {
    Crc32,
    Md5,
    Sha1,
    Sha256,
    Blake3,
}

impl HashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Crc32 => "CRC32",
            HashAlgorithm::Md5 => "MD5",
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Blake3 => "BLAKE3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BufferSize {
    _64KB,
    _256KB,
    _1MB,
    _4MB,
}

impl BufferSize {
    pub fn to_bytes(&self) -> usize {
        match self {
            BufferSize::_64KB => 64 * 1024,
            BufferSize::_256KB => 256 * 1024,
            BufferSize::_1MB => 1024 * 1024,
            BufferSize::_4MB => 4 * 1024 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_size_to_bytes_matches_documented_sizes() {
        assert_eq!(BufferSize::_64KB.to_bytes(), 64 * 1024);
        assert_eq!(BufferSize::_256KB.to_bytes(), 256 * 1024);
        assert_eq!(BufferSize::_1MB.to_bytes(), 1024 * 1024);
        assert_eq!(BufferSize::_4MB.to_bytes(), 4 * 1024 * 1024);
    }

    #[test]
    fn buffer_size_ordering_is_monotonic() {
        // Sanity: the four presets are ordered small → large.
        let mut sizes = [
            BufferSize::_64KB,
            BufferSize::_256KB,
            BufferSize::_1MB,
            BufferSize::_4MB,
        ];
        // Sort by byte value and verify it matches the input order.
        sizes.sort_by_key(|b| b.to_bytes());
        assert_eq!(sizes[0], BufferSize::_64KB);
        assert_eq!(sizes[1], BufferSize::_256KB);
        assert_eq!(sizes[2], BufferSize::_1MB);
        assert_eq!(sizes[3], BufferSize::_4MB);
    }

    #[test]
    fn hash_algorithm_as_str_is_stable_and_distinct() {
        // Names must not change silently — they show up in transfer reports.
        assert_eq!(HashAlgorithm::Crc32.as_str(), "CRC32");
        assert_eq!(HashAlgorithm::Md5.as_str(), "MD5");
        assert_eq!(HashAlgorithm::Sha1.as_str(), "SHA-1");
        assert_eq!(HashAlgorithm::Sha256.as_str(), "SHA-256");
        assert_eq!(HashAlgorithm::Blake3.as_str(), "BLAKE3");

        // No two algorithms share a label (would corrupt report columns).
        let labels = [
            HashAlgorithm::Crc32.as_str(),
            HashAlgorithm::Md5.as_str(),
            HashAlgorithm::Sha1.as_str(),
            HashAlgorithm::Sha256.as_str(),
            HashAlgorithm::Blake3.as_str(),
        ];
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len());
    }

    #[test]
    fn transfer_options_default_is_safe() {
        let opts = TransferOptions::default();
        // No destructive defaults: user must opt in to wipe, halt, etc.
        assert!(!opts.verify_after_copy);
        assert_eq!(opts.hash_algorithm, HashAlgorithm::Blake3);
        assert_eq!(opts.buffer_size, BufferSize::_1MB);
        assert!(!opts.direct_io);
        // Preservation is on by default — backwards-compatible behaviour.
        assert!(opts.preserve_timestamps);
        assert!(opts.preserve_attributes);
        assert!(!opts.preserve_acl);
        assert!(!opts.preserve_streams);
        // Conflict resolution: ask the user (safest default).
        assert_eq!(opts.conflict_resolution, "ask");
        // Bandwidth: uncapped.
        assert!(opts.limit_bandwidth_rate.is_none());
        // No wipe by default.
        assert_eq!(opts.wipe_passes, 0);
        // Symlink policy: leave alone.
        assert!(!opts.skip_symlinks);
        assert!(!opts.follow_symlinks);
        // Retries: a reasonable 3.
        assert_eq!(opts.max_retries, 3);
        // Reports off by default.
        assert!(!opts.auto_report);
        assert_eq!(opts.report_format, "html");
        // Don't halt on error by default — collect failures.
        assert!(!opts.halt_on_error);
        assert!(!opts.delete_to_recycle_bin);
    }

    #[test]
    fn transfer_options_serde_roundtrip_preserves_all_fields() {
        let original = TransferOptions {
            verify_after_copy: true,
            hash_algorithm: HashAlgorithm::Sha256,
            buffer_size: BufferSize::_4MB,
            direct_io: true,
            preserve_timestamps: false,
            preserve_attributes: false,
            preserve_acl: true,
            preserve_streams: true,
            skip_symlinks: true,
            follow_symlinks: false,
            max_retries: 7,
            conflict_resolution: "overwrite_older".to_string(),
            filter_mask: Some("*.rs".to_string()),
            limit_bandwidth_rate: Some(1_048_576),
            auto_report: true,
            report_format: "csv".to_string(),
            halt_on_error: true,
            delete_to_recycle_bin: true,
            wipe_passes: 3,
        };

        let serialized = toml::to_string(&original).expect("serialize");
        let deserialized: TransferOptions = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn transfer_options_minimal_toml_roundtrip() {
        // Simulate the minimal config a user might write by hand.
        let toml_src = r#"
verify_after_copy = true
hash_algorithm = "Sha1"
buffer_size = "_256KB"
direct_io = false
preserve_timestamps = true
preserve_attributes = false
preserve_acl = false
preserve_streams = false
skip_symlinks = false
follow_symlinks = false
max_retries = 5
conflict_resolution = "skip"
filter_mask = "*.log"
limit_bandwidth_rate = 524288
auto_report = false
report_format = "html"
halt_on_error = true
delete_to_recycle_bin = false
wipe_passes = 0
"#;
        let opts: TransferOptions = toml::from_str(toml_src).expect("parse minimal toml");
        assert!(opts.verify_after_copy);
        assert_eq!(opts.hash_algorithm, HashAlgorithm::Sha1);
        assert_eq!(opts.buffer_size, BufferSize::_256KB);
        assert!(opts.preserve_timestamps);
        assert!(!opts.preserve_attributes);
        assert_eq!(opts.max_retries, 5);
        assert_eq!(opts.conflict_resolution, "skip");
        assert_eq!(opts.filter_mask.as_deref(), Some("*.log"));
        assert_eq!(opts.limit_bandwidth_rate, Some(524288));
        assert!(opts.halt_on_error);
        assert_eq!(opts.wipe_passes, 0);
    }

    #[test]
    fn transfer_options_missing_optional_fields_default_safely() {
        // `limit_bandwidth_rate` and `filter_mask` are `Option` and may
        // be absent from a user-edited TOML. Other fields are required.
        let toml_src = r#"
verify_after_copy = false
hash_algorithm = "Blake3"
buffer_size = "_1MB"
direct_io = false
preserve_timestamps = true
preserve_attributes = true
preserve_acl = false
preserve_streams = false
skip_symlinks = false
follow_symlinks = false
max_retries = 3
conflict_resolution = "ask"
auto_report = false
report_format = "html"
halt_on_error = false
delete_to_recycle_bin = false
wipe_passes = 0
"#;
        let opts: TransferOptions = toml::from_str(toml_src).expect("parse");
        assert!(opts.filter_mask.is_none());
        assert!(opts.limit_bandwidth_rate.is_none());
    }
}
