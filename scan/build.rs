use std::process::Command;

fn get_git_output(args: &[&str], default: &str) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn main() {
    // YARA-X is a pure Rust implementation and doesn't require system libraries
    // or build-time configuration unlike the original YARA which needed C bindings.
    // This build script is kept minimal for future build requirements.

    println!("cargo:rerun-if-changed=build.rs");

    // Capture git commit information
    let git_commit = get_git_output(&["rev-parse", "--short", "HEAD"], "unknown");
    let git_commit_full = get_git_output(&["rev-parse", "HEAD"], "unknown");
    let git_branch = get_git_output(&["rev-parse", "--abbrev-ref", "HEAD"], "unknown");
    let git_dirty = Command::new("git")
        .args(["diff", "--quiet"])
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(false);

    // Set build-time environment variables
    println!("cargo:rustc-env=GIT_COMMIT_SHORT={git_commit}");
    println!("cargo:rustc-env=GIT_COMMIT_FULL={git_commit_full}");
    println!("cargo:rustc-env=GIT_BRANCH={git_branch}");
    println!("cargo:rustc-env=GIT_DIRTY={git_dirty}");
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
}
