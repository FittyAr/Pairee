//! Transfer engine defaults (`transfer_*` keys of `config.toml`).

use super::types::{
    default_transfer_buffer, default_transfer_conflict, default_transfer_hash,
    default_transfer_max_retries, default_transfer_report_format, default_true,
};
use serde::{Deserialize, Serialize};

/// Defaults the Copy / Move dialog and the transfer engine start from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TransferSettings {
    #[serde(rename = "transfer_default_hash", default = "default_transfer_hash")]
    pub default_hash: String,
    #[serde(rename = "transfer_buffer_size", default = "default_transfer_buffer")]
    pub buffer_size: u32,
    #[serde(rename = "transfer_verify_after_copy", default)]
    pub verify_after_copy: bool,
    #[serde(rename = "transfer_direct_io", default)]
    pub direct_io: bool,
    #[serde(rename = "transfer_preserve_timestamps", default = "default_true")]
    pub preserve_timestamps: bool,
    #[serde(rename = "transfer_preserve_attributes", default = "default_true")]
    pub preserve_attributes: bool,
    #[serde(
        rename = "transfer_max_retries",
        default = "default_transfer_max_retries"
    )]
    pub max_retries: u32,
    #[serde(
        rename = "transfer_conflict_resolution",
        default = "default_transfer_conflict"
    )]
    pub conflict_resolution: String,
    #[serde(rename = "transfer_skip_symlinks", default)]
    pub skip_symlinks: bool,
    #[serde(rename = "transfer_halt_on_error", default)]
    pub halt_on_error: bool,
    #[serde(rename = "transfer_preserve_acl", default)]
    pub preserve_acl: bool,
    #[serde(rename = "transfer_preserve_streams", default)]
    pub preserve_streams: bool,
    #[serde(rename = "transfer_follow_symlinks", default)]
    pub follow_symlinks: bool,
    #[serde(rename = "transfer_limit_bandwidth_rate", default)]
    pub limit_bandwidth_rate: Option<u64>,
    #[serde(rename = "transfer_auto_report", default)]
    pub auto_report: bool,
    #[serde(
        rename = "transfer_report_format",
        default = "default_transfer_report_format"
    )]
    pub report_format: String,
    /// A file a copy or move overwrites goes to the trash first, so undo
    /// can bring it back.
    #[serde(rename = "transfer_trash_replaced", default = "default_true")]
    pub trash_replaced: bool,
}

impl Default for TransferSettings {
    fn default() -> Self {
        Self {
            default_hash: default_transfer_hash(),
            buffer_size: default_transfer_buffer(),
            verify_after_copy: false,
            direct_io: false,
            preserve_timestamps: true,
            preserve_attributes: true,
            max_retries: default_transfer_max_retries(),
            conflict_resolution: default_transfer_conflict(),
            skip_symlinks: false,
            halt_on_error: false,
            preserve_acl: false,
            preserve_streams: false,
            follow_symlinks: false,
            limit_bandwidth_rate: None,
            auto_report: false,
            report_format: default_transfer_report_format(),
            trash_replaced: true,
        }
    }
}
