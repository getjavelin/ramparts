use colored::*;
use std::env;

/// Displays a clean banner with version, date, git commit info, and other useful information
pub fn display_banner() {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    let build_date = env!("BUILD_DATE");
    let git_commit_short = env!("GIT_COMMIT_SHORT");
    let git_commit_full = env!("GIT_COMMIT_FULL");
    let git_branch = env!("GIT_BRANCH");
    let git_dirty = env!("GIT_DIRTY");
    
    // Get current date/time
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    
    // Create the banner
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════════════╗".bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", format!("║  {}  ║", name.to_uppercase().bold().white()).bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", format!("║  {}  ║", "MCP Security Scanner".italic().cyan()).bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", format!("║  Version: {}  ║", version.bright_green()).bright_blue());
    println!("{}", format!("║  Build Date: {}  ║", build_date.bright_yellow()).bright_blue());
    println!("{}", format!("║  Current Time: {}  ║", current_date.bright_yellow()).bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", format!("║  Git Commit: {} ({})  ║", git_commit_short.bright_cyan(), git_commit_full[..8].bright_cyan()).bright_blue());
    println!("{}", format!("║  Git Branch: {}  ║", git_branch.bright_magenta()).bright_blue());
    println!("{}", format!("║  Git Status: {}  ║", 
        if git_dirty == "dirty" { "dirty".bright_red() } else { "clean".bright_green() }
    ).bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", format!("║  Repository: {}  ║", "https://github.com/getjavelin/ramparts".bright_blue()).bright_blue());
    println!("{}", format!("║  License: {}  ║", "Apache-2.0".bright_blue()).bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", "║  Security Assessments: Tool Poisoning • SQL Injection • Command Injection  ║".bright_blue());
    println!("{}", "║  • Path Traversal • Authentication Bypass • Secrets Leakage • Prompt      ║".bright_blue());
    println!("{}", "║  • Injection • Jailbreak • PII Leakage • Cross-Origin Escalation        ║".bright_blue());
    println!("{}", "║                                                                              ║".bright_blue());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════════════╝".bright_blue());
    println!();
} 