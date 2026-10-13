#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TransferOptions {
    /// Re-read every copy from the disk and compare its hash with the source.
    pub verify_after_copy: bool,
    pub hash_algorithm: HashAlgorithm,
    pub buffer_size: BufferSize,
    pub direct_io: bool,
    pub preserve_timestamps: bool,
    /// Windows file attributes (read-only, hidden, system, archive).
    pub preserve_attributes: bool,
    /// Access mode: Unix permission bits; on Windows the access control list
    /// (as [`Self::preserve_acl`]).
    pub preserve_permissions: bool,
    pub preserve_acl: bool,
    /// NTFS alternate data streams.
    pub preserve_streams: bool,
    /// Skip runs of zero bytes, leaving holes (sparse files).
    pub produce_sparse_files: bool,
    /// Clone the data blocks (copy-on-write) where the filesystem can.
    pub use_copy_on_write: bool,
    /// Send a file that a copy replaces to the trash (so an undo can bring
    /// it back) instead of overwriting it for good.
    pub trash_replaced: bool,
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
            preserve_permissions: true,
            preserve_acl: false,
            preserve_streams: false,
            produce_sparse_files: false,
            use_copy_on_write: false,
            trash_replaced: false,
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
        }
    }
}

impl TransferOptions {
    /// The options a job of `operation` runs with: deletes and moves never
    /// follow links (they would delete, or move away, the files a link
    /// points to outside the tree); links are removed or moved as links.
    pub fn for_operation(&self, operation: super::job::TransferOperation) -> Self {
        use super::job::TransferOperation as Op;
        let mut options = self.clone();
        if matches!(operation, Op::Delete | Op::Move) {
            options.follow_symlinks = false;
        }
        options
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

    /// The next algorithm, in the order the option dialogs cycle them.
    pub fn next(self) -> Self {
        match self {
            HashAlgorithm::Blake3 => HashAlgorithm::Crc32,
            HashAlgorithm::Crc32 => HashAlgorithm::Md5,
            HashAlgorithm::Md5 => HashAlgorithm::Sha1,
            HashAlgorithm::Sha1 => HashAlgorithm::Sha256,
            HashAlgorithm::Sha256 => HashAlgorithm::Blake3,
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
    pub fn to_bytes(self) -> usize {
        match self {
            BufferSize::_64KB => 64 * 1024,
            BufferSize::_256KB => 256 * 1024,
            BufferSize::_1MB => 1024 * 1024,
            BufferSize::_4MB => 4 * 1024 * 1024,
        }
    }
}
