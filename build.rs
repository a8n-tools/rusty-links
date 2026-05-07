// Captures version, git hash, and build date at compile time.
//
// Resolution order for each value:
//   1. Env var override (BUILD_VERSION / BUILD_GIT_HASH / BUILD_DATE) — set by CI
//      and Docker build args so OCI builds (which exclude .git via .dockerignore)
//      still embed accurate metadata.
//   2. Git CLI fallback for local builds where .git is present.
//   3. Sensible default ("unknown" / current UTC date).

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
    println!("cargo:rerun-if-env-changed=BUILD_GIT_HASH");
    println!("cargo:rerun-if-env-changed=BUILD_DATE");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");

    let version = std::env::var("BUILD_VERSION")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| run_git(&["describe", "--tags", "--always", "--dirty"]))
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));

    let git_hash = std::env::var("BUILD_GIT_HASH")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| run_git(&["rev-parse", "--short=12", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());

    let build_date = std::env::var("BUILD_DATE")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("date")
                .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    println!("cargo:rustc-env=BUILD_GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
}

fn run_git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}
