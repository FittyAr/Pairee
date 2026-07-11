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
