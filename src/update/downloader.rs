use anyhow::{Context as _, Result};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Download a release asset from `url` into `dest_dir`.
/// Sends progress updates (0.0 – 1.0) via `progress_tx` (may be None).
/// Returns the path to the downloaded file.
pub async fn download_asset(
    url: &str,
    dest_dir: &std::path::Path,
    filename: &str,
    progress_tx: Option<mpsc::Sender<f32>>,
) -> Result<PathBuf> {
    use tokio::io::AsyncWriteExt as _;

    let client = build_client()?;
    let mut response = client
        .get(url)
        .header(
            "User-Agent",
            format!("pairee/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .context("failed to start download")?;

    if !response.status().is_success() {
        anyhow::bail!("download returned status {}", response.status());
    }

    let total = response.content_length().unwrap_or(0);
    let dest_path = dest_dir.join(filename);
    let mut file = tokio::fs::File::create(&dest_path)
        .await
        .context("failed to create download destination file")?;

    let mut downloaded: u64 = 0;

    loop {
        match response.chunk().await.context("stream error")? {
            Some(chunk) => {
                file.write_all(&chunk)
                    .await
                    .context("failed to write chunk")?;
                downloaded += chunk.len() as u64;
                if total > 0 {
                    let progress = downloaded as f32 / total as f32;
                    if let Some(tx) = &progress_tx {
                        let _ = tx.try_send(progress);
                    }
                }
            }
            None => break,
        }
    }

    {
        file.flush().await.context("failed to flush download file")?;
    }

    if let Some(tx) = &progress_tx {
        let _ = tx.try_send(1.0);
    }

    Ok(dest_path)
}

/// Verify a file against a SHA-256 checksum string (hex, 64 chars).
/// Returns Ok(()) on match, Err otherwise.
pub fn verify_sha256(file_path: &std::path::Path, expected_hex: &str) -> Result<()> {
    use std::io::Read as _;
    let mut file = std::fs::File::open(file_path).context("failed to open file for verification")?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).context("read error during verification")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = hasher.finalize();
    let actual_hex = hex_encode(&actual);
    if actual_hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        anyhow::bail!(
            "SHA-256 mismatch: expected {}, got {}",
            expected_hex,
            actual_hex
        )
    }
}

// ─── Minimal SHA-256 implementation (no extra dependency) ────────────────────

struct Sha256 {
    state: [u32; 8],
    buffer: Vec<u8>,
    total: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: Vec::new(),
            total: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.total += data.len() as u64;
        self.buffer.extend_from_slice(data);
        while self.buffer.len() >= 64 {
            let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
            self.buffer.drain(..64);
            process_block(&mut self.state, &block);
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total * 8;
        self.buffer.push(0x80);
        while self.buffer.len() % 64 != 56 {
            self.buffer.push(0);
        }
        self.buffer.extend_from_slice(&bit_len.to_be_bytes());
        while self.buffer.len() >= 64 {
            let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
            self.buffer.drain(..64);
            process_block(&mut self.state, &block);
        }
        let mut out = [0u8; 32];
        for (i, &word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

fn process_block(state: &mut [u32; 8], block: &[u8; 64]) {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ─── Platform target selection ───────────────────────────────────────────────

/// Returns the expected asset filename for the current platform and target.
pub fn expected_asset_name(version: &str) -> String {
    let target = current_target();
    // Linux: pairee-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz
    // Windows: pairee-vX.Y.Z-x86_64-pc-windows-msvc.zip
    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };
    format!("pairee-v{}-{}.{}", version, target, ext)
}

/// Returns the Inno Setup installer asset filename for Windows.
#[cfg(target_os = "windows")]
pub fn expected_installer_name(version: &str) -> String {
    if cfg!(target_arch = "aarch64") {
        format!("pairee-setup-{}-arm64.exe", version)
    } else {
        format!("pairee-setup-{}-x64.exe", version)
    }
}

/// Returns the current target triple for this binary.
fn current_target() -> &'static str {
    // These cfg values are set by the Rust compiler
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-musl"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "aarch64-unknown-linux-musl"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "aarch64-pc-windows-msvc"
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "unknown"
    }
}

fn build_client() -> Result<reqwest::Client> {
    use std::time::Duration;
    reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to build HTTP client")
}
