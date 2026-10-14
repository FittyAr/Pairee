//! `cargo xtask sign-registry`: signs the plugin registry with minisign.
//!
//! `index.toml` and `blocklist.toml` get a trusted comment with a serial
//! and a timestamp (`file:index.toml serial:42 timestamp:1760000000`): the
//! client refuses an index whose serial is lower than one it accepted
//! before (rollback) or whose timestamp is too old (a frozen mirror). The
//! serial is one more than the highest one in the current signatures.
//! Every plugin `manifest.toml` is signed with `file:<path>` only (the
//! signed index already names the version it must carry).
//!
//! The secret key is read like the release signing (`.github/scripts/
//! sign.sh`): the contents of `MINISIGN_SECRET_KEY` (CI), or a key file
//! with `--key` (the maintainer's offline key); its password, if any, from
//! `MINISIGN_PASSWORD` (otherwise minisign asks for it). Each signature is
//! then checked with `minisign -V` against the public key embedded in the
//! updater (`src/update/signature.rs`), so a wrong secret key fails here
//! instead of in every client.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Result, ui};

/// The registry files whose signature carries a serial and a timestamp
/// (kept in line with `plugin::updater::trust::FRESHNESS_TRACKED`).
const FRESHNESS_TRACKED: [&str; 2] = ["index.toml", "blocklist.toml"];

/// Where the public key the clients pin is declared.
const SIGNATURE_RS: &str = "src/update/signature.rs";

const USAGE: &str = "\
Usage: cargo xtask sign-registry <registry-dir> [--key <secret-key-file>] [--serial N] [--public-key <base64>]

  <registry-dir>  the `registry/` folder of a plugin-registry checkout
                  (absolute, or relative to the Pairee workspace root)
  --key           minisign secret key file (default: the contents of
                  MINISIGN_SECRET_KEY); password from MINISIGN_PASSWORD
  --serial        serial to sign with (default: one more than the current)
  --public-key    key to check the signatures with (default: the one in
                  src/update/signature.rs)
";

struct Options {
    dir: PathBuf,
    key_file: Option<PathBuf>,
    serial: Option<u64>,
    public_key: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Options> {
    let mut dir = None;
    let mut key_file = None;
    let mut serial = None;
    let mut public_key = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n\n{USAGE}"))
        };
        match arg.as_str() {
            "--key" => key_file = Some(PathBuf::from(value("--key")?)),
            "--serial" => {
                let text = value("--serial")?;
                serial = Some(
                    text.parse()
                        .map_err(|_| format!("--serial: not a number: {text}"))?,
                );
            }
            "--public-key" => public_key = Some(value("--public-key")?),
            "-h" | "--help" => return Err(USAGE.into()),
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}\n\n{USAGE}").into());
            }
            path if dir.is_none() => dir = Some(PathBuf::from(path)),
            extra => return Err(format!("unexpected argument {extra}\n\n{USAGE}").into()),
        }
    }
    Ok(Options {
        dir: dir.ok_or_else(|| format!("missing <registry-dir>\n\n{USAGE}"))?,
        key_file,
        serial,
        public_key,
    })
}

/// `cargo xtask sign-registry ...`
pub fn run(args: &[String]) -> Result {
    let opts = parse_args(args)?;
    let dir = &opts.dir;
    if !dir.join("index.toml").is_file() {
        return Err(format!("{} has no index.toml", dir.display()).into());
    }
    let public_key = match opts.public_key {
        Some(key) => key,
        None => embedded_public_key(&fs::read_to_string(SIGNATURE_RS)?)
            .ok_or_else(|| format!("no RELEASE_PUBLIC_KEY in {SIGNATURE_RS}"))?
            .to_string(),
    };
    if !minisign_available() {
        return Err("minisign is not installed (https://jedisct1.github.io/minisign/)".into());
    }

    let fresh: Vec<&str> = FRESHNESS_TRACKED
        .into_iter()
        .filter(|name| dir.join(name).is_file())
        .collect();
    let previous = fresh
        .iter()
        .filter_map(|name| fs::read_to_string(dir.join(format!("{name}.minisig"))).ok())
        .filter_map(|sig| signed_serial(&sig))
        .max();
    let serial = next_serial(previous, opts.serial)?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let key = SecretKey::open(opts.key_file)?;
    let mut signed = 0;
    for name in &fresh {
        sign(dir, name, &freshness_comment(name, serial, timestamp), &key)?;
        verify(dir, name, &public_key)?;
        signed += 1;
    }
    let mut manifests = Vec::new();
    collect_manifests(dir, Path::new("plugins"), &mut manifests)?;
    manifests.sort();
    for rel in &manifests {
        sign(dir, rel, &format!("file:{rel}"), &key)?;
        verify(dir, rel, &public_key)?;
        signed += 1;
    }
    ui::ok(&format!(
        "signed {signed} registry files (serial {serial}, timestamp {timestamp})"
    ));
    Ok(())
}

/// The trusted comment of a freshness-tracked file.
fn freshness_comment(name: &str, serial: u64, timestamp: u64) -> String {
    format!("file:{name} serial:{serial} timestamp:{timestamp}")
}

/// The `serial:` of a `.minisig` file's trusted comment.
fn signed_serial(minisig: &str) -> Option<u64> {
    minisig
        .lines()
        .find_map(|line| line.strip_prefix("trusted comment:"))?
        .split_whitespace()
        .find_map(|word| word.strip_prefix("serial:")?.parse().ok())
}

/// The serial to sign with: `requested`, which must be above `previous`,
/// or one more than it.
fn next_serial(previous: Option<u64>, requested: Option<u64>) -> Result<u64> {
    let floor = previous.map_or(1, |p| p + 1);
    match requested {
        Some(serial) if serial < floor => Err(format!(
            "--serial {serial} is not above the current serial {}: clients would refuse it",
            floor - 1
        )
        .into()),
        Some(serial) => Ok(serial),
        None => Ok(floor),
    }
}

/// `RELEASE_PUBLIC_KEY` as declared in `src/update/signature.rs`.
fn embedded_public_key(source: &str) -> Option<&str> {
    let rest = source.split("RELEASE_PUBLIC_KEY: &str = \"").nth(1)?;
    rest.split('"').next().filter(|key| !key.is_empty())
}

/// Adds the `manifest.toml` files under `dir/rel` to `out`, as `/` paths
/// relative to `dir` (the names their trusted comments carry).
fn collect_manifests(dir: &Path, rel: &Path, out: &mut Vec<String>) -> Result {
    let Ok(entries) = fs::read_dir(dir.join(rel)) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let child = rel.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_manifests(dir, &child, out)?;
        } else if kind.is_file() && entry.file_name() == "manifest.toml" {
            let parts: Vec<String> = child
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
    Ok(())
}

fn minisign_available() -> bool {
    crate::cmd::quietly(Command::new("minisign").arg("-v"))
}

/// The secret key file minisign signs with; a key taken from
/// `MINISIGN_SECRET_KEY` lives in a private temporary file removed on drop.
struct SecretKey {
    path: PathBuf,
    temporary: bool,
    password: Option<String>,
}

impl SecretKey {
    fn open(key_file: Option<PathBuf>) -> Result<Self> {
        let password = std::env::var("MINISIGN_PASSWORD")
            .ok()
            .filter(|p| !p.is_empty());
        if let Some(path) = key_file {
            return Ok(Self {
                path,
                temporary: false,
                password,
            });
        }
        let contents = std::env::var("MINISIGN_SECRET_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .ok_or("no secret key: pass --key <file> or set MINISIGN_SECRET_KEY")?;
        let path = std::env::temp_dir().join(format!("pairee-registry-{}.key", std::process::id()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&path)?;
        let key = Self {
            path,
            temporary: true,
            password,
        };
        writeln!(file, "{}", contents.trim_end())?;
        Ok(key)
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        if self.temporary {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// `minisign -S` on `dir/rel` with `comment` as the trusted comment.
fn sign(dir: &Path, rel: &str, comment: &str, key: &SecretKey) -> Result {
    let mut command = Command::new("minisign");
    command
        .current_dir(dir)
        .arg("-S")
        .arg("-s")
        .arg(&key.path)
        .args(["-m", rel, "-t", comment]);
    let status = match &key.password {
        Some(password) => {
            let mut child = command.stdin(Stdio::piped()).spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                writeln!(stdin, "{password}")?;
            }
            child.wait()?
        }
        // An offline key file may ask for its password; a CI key has none.
        None if key.temporary => command.stdin(Stdio::null()).status()?,
        None => command.status()?,
    };
    if !status.success() {
        return Err(format!("minisign could not sign {rel} ({status})").into());
    }
    Ok(())
}

/// `minisign -V` of `dir/rel` against `public_key`.
fn verify(dir: &Path, rel: &str, public_key: &str) -> Result {
    let ok = crate::cmd::quietly(
        Command::new("minisign")
            .current_dir(dir)
            .args(["-V", "-q", "-P", public_key, "-m", rel]),
    );
    if ok {
        Ok(())
    } else {
        Err(
            format!("the signature of {rel} does not verify with {public_key}: wrong secret key?")
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_comment_names_file_serial_and_timestamp() {
        assert_eq!(
            freshness_comment("index.toml", 7, 1_760_000_000),
            "file:index.toml serial:7 timestamp:1760000000"
        );
    }

    #[test]
    fn serial_is_read_from_the_trusted_comment() {
        let sig = "untrusted comment: serial:99\nRUS...\n\
                   trusted comment: file:index.toml serial:41 timestamp:1\nabc==\n";
        assert_eq!(signed_serial(sig), Some(41));
        assert_eq!(
            signed_serial("trusted comment: file:index.toml version:x\n"),
            None
        );
        assert_eq!(signed_serial(""), None);
    }

    #[test]
    fn next_serial_only_moves_forward() {
        assert_eq!(next_serial(None, None).unwrap(), 1);
        assert_eq!(next_serial(Some(41), None).unwrap(), 42);
        assert_eq!(next_serial(Some(41), Some(100)).unwrap(), 100);
        assert!(next_serial(Some(41), Some(41)).is_err());
        assert!(next_serial(Some(41), Some(3)).is_err());
    }

    #[test]
    fn public_key_comes_from_the_updater_source() {
        let source = fs::read_to_string(Path::new("..").join(SIGNATURE_RS))
            .or_else(|_| fs::read_to_string(SIGNATURE_RS))
            .unwrap();
        let key = embedded_public_key(&source).unwrap();
        assert!(key.starts_with("RW") && key.len() == 56, "{key}");
        assert_eq!(embedded_public_key("nothing here"), None);
    }

    #[test]
    fn manifests_are_found_with_slash_paths() {
        let root = std::env::temp_dir().join(format!("xtask-registry-{}", std::process::id()));
        let plugin = root.join("plugins").join("a").join("alice").join("demo");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(plugin.join("manifest.toml"), "").unwrap();
        fs::write(plugin.join("main.lua"), "").unwrap();
        let mut found = Vec::new();
        collect_manifests(&root, Path::new("plugins"), &mut found).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(found, ["plugins/a/alice/demo/manifest.toml"]);
    }

    #[test]
    fn arguments_are_parsed() {
        let args: Vec<String> = ["reg", "--serial", "5", "--key", "k.key"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let opts = parse_args(&args).unwrap();
        assert_eq!(opts.dir, PathBuf::from("reg"));
        assert_eq!(opts.serial, Some(5));
        assert_eq!(opts.key_file, Some(PathBuf::from("k.key")));
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&["reg".into(), "--serial".into(), "x".into()]).is_err());
        assert!(parse_args(&["reg".into(), "--bogus".into()]).is_err());
    }
}
