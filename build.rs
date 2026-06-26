use std::env;
use std::process::Command;

fn main() {
    // 1. Get compilation target triple
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=PAIREE_TARGET={}", target);

    // 2. Get Git commit hash
    let git_hash = match Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "no-git".to_string(),
    };
    println!("cargo:rustc-env=PAIREE_GIT_HASH={}", git_hash);

    // 3. Get build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=PAIREE_BUILD_PROFILE={}", profile);

    // Re-run build script if HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
