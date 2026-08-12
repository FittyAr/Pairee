//! Build [`TransferOptions`] from app settings (DRY for action call sites).

use super::options::{BufferSize, HashAlgorithm, TransferOptions};
use crate::config::settings::Settings;

/// Map user settings into transfer options (defaults for local engine jobs).
pub fn transfer_options_from_settings(settings: &Settings) -> TransferOptions {
    TransferOptions {
        verify_after_copy: settings.transfer_verify_after_copy,
        hash_algorithm: match settings.transfer_default_hash.as_str() {
            "crc32" => HashAlgorithm::Crc32,
            "md5" => HashAlgorithm::Md5,
            "sha1" => HashAlgorithm::Sha1,
            "sha256" => HashAlgorithm::Sha256,
            _ => HashAlgorithm::Blake3,
        },
        buffer_size: match settings.transfer_buffer_size {
            65536 => BufferSize::_64KB,
            262144 => BufferSize::_256KB,
            1048576 => BufferSize::_1MB,
            4194304 => BufferSize::_4MB,
            n if n <= 65536 => BufferSize::_64KB,
            n if n <= 262144 => BufferSize::_256KB,
            n if n <= 1048576 => BufferSize::_1MB,
            _ => BufferSize::_4MB,
        },
        direct_io: settings.transfer_direct_io,
        preserve_timestamps: settings.transfer_preserve_timestamps,
        preserve_attributes: settings.transfer_preserve_attributes,
        preserve_acl: settings.transfer_preserve_acl,
        preserve_streams: settings.transfer_preserve_streams,
        skip_symlinks: settings.transfer_skip_symlinks,
        follow_symlinks: settings.transfer_follow_symlinks,
        limit_bandwidth_rate: settings.transfer_limit_bandwidth_rate,
        halt_on_error: settings.transfer_halt_on_error,
        max_retries: settings.transfer_max_retries,
        conflict_resolution: settings.transfer_conflict_resolution.clone(),
        delete_to_recycle_bin: settings.delete_to_recycle_bin,
        ..Default::default()
    }
}
