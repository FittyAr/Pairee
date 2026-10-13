//! Build [`TransferOptions`] from app settings (DRY for action call sites).

use super::options::{BufferSize, HashAlgorithm, TransferOptions};
use crate::config::settings::Settings;

/// Map user settings into transfer options (defaults for local engine jobs).
pub fn transfer_options_from_settings(settings: &Settings) -> TransferOptions {
    TransferOptions {
        verify_after_copy: settings.transfer.verify_after_copy,
        hash_algorithm: match settings.transfer.default_hash.as_str() {
            "crc32" => HashAlgorithm::Crc32,
            "md5" => HashAlgorithm::Md5,
            "sha1" => HashAlgorithm::Sha1,
            "sha256" => HashAlgorithm::Sha256,
            _ => HashAlgorithm::Blake3,
        },
        buffer_size: match settings.transfer.buffer_size {
            65536 => BufferSize::_64KB,
            262144 => BufferSize::_256KB,
            1048576 => BufferSize::_1MB,
            4194304 => BufferSize::_4MB,
            n if n <= 65536 => BufferSize::_64KB,
            n if n <= 262144 => BufferSize::_256KB,
            n if n <= 1048576 => BufferSize::_1MB,
            _ => BufferSize::_4MB,
        },
        direct_io: settings.transfer.direct_io,
        preserve_timestamps: settings.transfer.preserve_timestamps,
        preserve_attributes: settings.transfer.preserve_attributes,
        preserve_acl: settings.transfer.preserve_acl,
        preserve_streams: settings.transfer.preserve_streams,
        skip_symlinks: settings.transfer.skip_symlinks,
        follow_symlinks: settings.transfer.follow_symlinks,
        limit_bandwidth_rate: settings.transfer.limit_bandwidth_rate,
        halt_on_error: settings.transfer.halt_on_error,
        max_retries: settings.transfer.max_retries,
        conflict_resolution: settings.transfer.conflict_resolution.clone(),
        delete_to_recycle_bin: settings.delete_to_recycle_bin,
        // Overwritten files go to the trash, so undo can restore them.
        trash_replaced: true,
        ..Default::default()
    }
}
