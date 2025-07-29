use std::process::Command;

fn main() {
    // YARA-X is a pure Rust implementation and doesn't require system libraries
    // or build-time configuration unlike the original YARA which needed C bindings.
    // This build script is kept minimal for future build requirements.

    println!("cargo:rerun-if-changed=build.rs");

    // Capture git commit information
    let git_commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    let git_commit_full = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    let git_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    let git_dirty = Command::new("git")
        .args(["diff", "--quiet"])
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(false);

    // Set build-time environment variables
    println!("cargo:rustc-env=GIT_COMMIT_SHORT={}", git_commit.trim());
    println!("cargo:rustc-env=GIT_COMMIT_FULL={}", git_commit_full.trim());
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch.trim());
    println!(
        "cargo:rustc-env=GIT_DIRTY={}",
        if git_dirty { "dirty" } else { "clean" }
    );
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
}
