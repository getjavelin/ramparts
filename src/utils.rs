use crate::security::SecurityIssue;
use crate::types::{ScanResult, ScanStatus};
use anyhow::{anyhow, Result};
use colored::Colorize;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use tracing::{debug, warn};

use tabled::{Table, Tabled};

// ============================================================================
// UTILITY FUNCTIONS FOR REDUCING REDUNDANCY
// ============================================================================

/// Timing utility for measuring execution time
pub struct Timer {
    start_time: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        #[allow(clippy::cast_possible_truncation)]
        {
            self.start_time.elapsed().as_millis() as u64
        }
    }
}

/// Enhanced error handling utilities
pub mod error_utils {
    use super::{anyhow, Result};

    /// Format a standardized error message
    pub fn format_error(operation: &str, details: &str) -> String {
        format!("{operation} failed: {details}")
    }

    /// Wrap an error with context
    /// Wraps an error with additional context information
    #[allow(dead_code)] // Used in tests and for error context enhancement
    pub fn wrap_error<T>(result: Result<T>, context: &str) -> Result<T> {
        result.map_err(|e| anyhow!("{context}: {e}"))
    }
}

/// Performance monitoring utilities
pub mod performance {
    use super::{debug, warn, Result, Timer};

    /// Track performance metrics
    pub struct PerformanceTracker {
        timer: Timer,
        operation_name: String,
    }

    impl PerformanceTracker {
        pub fn start(operation_name: &str) -> Self {
            Self {
                timer: Timer::start(),
                operation_name: operation_name.to_string(),
            }
        }

        pub fn finish(self) -> u64 {
            let elapsed = self.timer.elapsed_ms();
            if elapsed > 5000 {
                // Only warn if over 5 seconds
                warn!("Slow operation: {} took {}ms", self.operation_name, elapsed);
            } else if elapsed > 1000 {
                debug!("{} completed in {}ms", self.operation_name, elapsed);
            }
            elapsed
        }
    }

    /// Execute an operation with performance tracking
    pub async fn track_performance<F, Fut, T>(operation_name: &str, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let tracker = PerformanceTracker::start(operation_name);
        let result = operation().await;
        let _elapsed = tracker.finish();
        result
    }
}

// ============================================================================
// EXISTING PRINTING FUNCTIONS
// ============================================================================

pub fn print_result(result: &ScanResult, format: &str, detailed: bool) {
    match format.to_lowercase().as_str() {
        "json" => print_json_result(result),
        "table" => print_table_result(result, detailed),
        "text" => print_text_result(result),
        "raw" => print_raw_json_result(result),
        _ => {
            eprintln!("Unknown format: {format}. Using table format.");
            print_table_result(result, detailed);
        }
    }
}

fn print_json_result(result: &ScanResult) {
    match serde_json::to_string_pretty(result) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("Error serializing result to JSON: {e}");
            println!("{{\"error\": \"Failed to serialize scan result\"}}");
        }
    }
}

fn print_raw_json_result(result: &ScanResult) {
    let raw_result = build_raw_json_result(result);
    println!(
        "{}",
        serde_json::to_string_pretty(&raw_result)
            .unwrap_or_else(|e| { format!("{{\"error\": \"Failed to serialize result: {e}\"}}") })
    );
}

/// Builds a raw JSON structure that preserves the original MCP server schema
fn build_raw_json_result(result: &ScanResult) -> serde_json::Map<String, serde_json::Value> {
    let mut raw_result = serde_json::Map::new();

    add_basic_scan_info(&mut raw_result, result);
    add_server_info(&mut raw_result, result);
    add_tools_info(&mut raw_result, result);
    add_resources_info(&mut raw_result, result);
    add_prompts_info(&mut raw_result, result);
    add_yara_results_info(&mut raw_result, result);
    add_errors_info(&mut raw_result, result);

    raw_result
}

/// Adds basic scan information to the raw JSON result
fn add_basic_scan_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    raw_result.insert(
        "url".to_string(),
        serde_json::Value::String(result.url.clone()),
    );
    raw_result.insert(
        "status".to_string(),
        serde_json::Value::String(format!("{:?}", result.status)),
    );
    raw_result.insert(
        "response_time_ms".to_string(),
        serde_json::Value::Number(serde_json::Number::from(result.response_time_ms)),
    );
    raw_result.insert(
        "timestamp".to_string(),
        serde_json::Value::String(result.timestamp.to_rfc3339()),
    );
}

/// Adds server information to the raw JSON result
fn add_server_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    if let Some(server_info) = &result.server_info {
        let mut server_info_obj = serde_json::Map::new();
        server_info_obj.insert(
            "name".to_string(),
            serde_json::Value::String(server_info.name.clone()),
        );
        server_info_obj.insert(
            "version".to_string(),
            serde_json::Value::String(server_info.version.clone()),
        );

        if let Some(desc) = &server_info.description {
            server_info_obj.insert(
                "description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }

        server_info_obj.insert(
            "capabilities".to_string(),
            serde_json::Value::Array(
                server_info
                    .capabilities
                    .iter()
                    .map(|c| serde_json::Value::String(c.clone()))
                    .collect(),
            ),
        );

        raw_result.insert(
            "server_info".to_string(),
            serde_json::Value::Object(server_info_obj),
        );
    }
}

/// Adds tools information to the raw JSON result
fn add_tools_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    if !result.tools.is_empty() {
        let tools_array = result
            .tools
            .iter()
            .map(|tool| {
                tool.raw_json.clone().unwrap_or_else(|| {
                    serde_json::to_value(tool).unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        raw_result.insert("tools".to_string(), serde_json::Value::Array(tools_array));
    }
}

/// Adds resources information to the raw JSON result
fn add_resources_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    if !result.resources.is_empty() {
        let resources_array = result
            .resources
            .iter()
            .map(|resource| {
                resource.raw_json.clone().unwrap_or_else(|| {
                    serde_json::to_value(resource).unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        raw_result.insert(
            "resources".to_string(),
            serde_json::Value::Array(resources_array),
        );
    }
}

/// Adds prompts information to the raw JSON result
fn add_prompts_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    if !result.prompts.is_empty() {
        let prompts_array = result
            .prompts
            .iter()
            .map(|prompt| {
                prompt.raw_json.clone().unwrap_or_else(|| {
                    serde_json::to_value(prompt).unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        raw_result.insert(
            "prompts".to_string(),
            serde_json::Value::Array(prompts_array),
        );
    }
}

/// Adds YARA scan results to the raw JSON result
fn add_yara_results_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    if !result.yara_results.is_empty() {
        let yara_results_array = result
            .yara_results
            .iter()
            .map(|yara_result| serde_json::to_value(yara_result).unwrap_or(serde_json::Value::Null))
            .collect();
        raw_result.insert(
            "yara_results".to_string(),
            serde_json::Value::Array(yara_results_array),
        );
    }
}

/// Adds error information to the raw JSON result
fn add_errors_info(
    raw_result: &mut serde_json::Map<String, serde_json::Value>,
    result: &ScanResult,
) {
    if !result.errors.is_empty() {
        let errors_array = result
            .errors
            .iter()
            .map(|e| serde_json::Value::String(e.clone()))
            .collect();
        raw_result.insert("errors".to_string(), serde_json::Value::Array(errors_array));
    }
}

#[allow(clippy::too_many_lines)]
fn print_table_result(result: &ScanResult, detailed: bool) {
    println!("Ramparts MCP Server Scan Result");

    // Server Info
    println!("URL: {}", result.url.blue());
    println!("Status: {}", format_status(&result.status));
    println!("Response Time: {}ms", result.response_time_ms);
    println!(
        "Timestamp: {}",
        result.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );

    if let Some(server_info) = &result.server_info {
        println!("\n{}", "Server Information".bold());
        println!("Name: {}", server_info.name);
        println!("Version: {}", server_info.version);
        if let Some(desc) = &server_info.description {
            println!("Description: {desc}");
        }
        if !server_info.capabilities.is_empty() {
            println!("Capabilities: {}", server_info.capabilities.join(", "));
        }
    }

    // Tools
    if !result.tools.is_empty() {
        println!("\n{}", "Tools".bold());
        if detailed {
            // Show detailed tool information
            for tool in &result.tools {
                println!("Tool: {}", tool.name.bold());
                if let Some(desc) = &tool.description {
                    println!("Description: {desc}");
                }
                if let Some(category) = &tool.category {
                    println!("Category: {category}");
                }
                if !tool.tags.is_empty() {
                    println!("Tags: {}", tool.tags.join(", "));
                }
                if tool.deprecated {
                    println!("Status: {}", "DEPRECATED".red().bold());
                }
                if let Some(input_schema) = &tool.input_schema {
                    println!(
                        "Input Schema: {}",
                        serde_json::to_string_pretty(input_schema)
                            .unwrap_or_else(|_| "Invalid JSON".to_string())
                    );
                }
                if let Some(output_schema) = &tool.output_schema {
                    println!(
                        "Output Schema: {}",
                        serde_json::to_string_pretty(output_schema)
                            .unwrap_or_else(|_| "Invalid JSON".to_string())
                    );
                }
                if !tool.parameters.is_empty() {
                    println!("Parameters:");
                    for (key, value) in &tool.parameters {
                        println!(
                            "  {}: {}",
                            key,
                            serde_json::to_string_pretty(value)
                                .unwrap_or_else(|_| "Invalid JSON".to_string())
                        );
                    }
                }
                if let Some(raw_json) = &tool.raw_json {
                    println!("Raw JSON Schema:");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(raw_json)
                            .unwrap_or_else(|_| "Invalid JSON".to_string())
                    );
                }
                println!();
            }
        } else {
            // Only print the number of tools
            println!("Number of tools: {}", result.tools.len());
        }
    }

    // Resources
    if !result.resources.is_empty() {
        println!("\n{}", "Resources".bold());
        let resource_table = Table::new(result.resources.iter().map(|r| ResourceRow {
            uri: r.uri.clone(),
            name: r.name.clone(),
            description: r.description.clone().unwrap_or_else(|| "N/A".to_string()),
            mime_type: r.mime_type.clone().unwrap_or_else(|| "N/A".to_string()),
        }))
        .with(tabled::settings::Style::empty())
        .to_string();
        println!("{resource_table}");
    }

    // Security Assessments Completed
    println!("\n{}", "Security Assessments".bold());
    let mut assessments = Vec::new();

    if !result.tools.is_empty() {
        assessments.push("Tool Security (Tool Poisoning, Secrets Leakage, SQL Injection, Command Injection, Path Traversal, Auth Bypass)");
    }
    if !result.tools.is_empty() || !result.prompts.is_empty() {
        assessments.push("Input Security (Prompt Injection, PII Leakage, Jailbreak)");
    }
    if !result.resources.is_empty() {
        assessments.push("Resource Security (Path Traversal, Sensitive Data)");
    }

    if assessments.is_empty() {
        println!("Assessments executed: None");
    } else {
        println!("Assessments executed: {}", assessments.join(", "));
    }

    // Security issues - use enhanced table format only
    if result.security_issues.is_some() {
        print_enhanced_security_table(result);
    }

    // YARA scan results
    if result.yara_results.is_empty() {
        // Show YARA execution status even when no results at all
        println!("\n{}", "YARA Scan Results".bold());
        println!("❌ YARA scanning not executed or no results available");
        println!();
    } else {
        println!("\n{}", "YARA Scan Results".bold());

        // Separate summary results from individual match results
        let summary_results: Vec<_> = result
            .yara_results
            .iter()
            .filter(|r| r.target_type == "summary")
            .collect();
        let match_results: Vec<_> = result
            .yara_results
            .iter()
            .filter(|r| r.target_type != "summary")
            .collect();

        // Show summary information first
        for summary in &summary_results {
            let status_icon = match summary.status.as_deref() {
                Some("passed") => "✅",
                Some("warning") => "⚠️",
                _ => "🔍",
            };

            let status_text = match summary.status.as_deref() {
                Some("passed") => "PASSED".green(),
                Some("warning") => "WARNING".yellow(),
                _ => "UNKNOWN".white(),
            };

            println!(
                "{} {} - {}",
                status_icon,
                summary.target_name.to_uppercase(),
                status_text
            );
            println!("  Context: {}", summary.context);

            if let Some(total_items) = summary.total_items_scanned {
                println!("  Items scanned: {total_items}");
            }
            if let Some(total_matches) = summary.total_matches {
                println!("  Security matches: {total_matches}");
            }
            if let Some(rules) = &summary.rules_executed {
                if !rules.is_empty() {
                    println!("  Rules executed: {}", rules.join(", "));
                }
            }
            if let Some(security_issues) = &summary.security_issues_detected {
                if !security_issues.is_empty() {
                    println!("  Security issues detected: {}", security_issues.join(", "));
                }
            }
            println!();
        }

        // Show individual match results if any
        if !match_results.is_empty() {
            println!(
                "🔍 {} Individual Security Matches:",
                "Detailed Results".bold()
            );
            println!();

            for yara_result in &match_results {
                let status_icon = match yara_result.status.as_deref() {
                    Some("warning") => "⚠️",
                    _ => "🔍",
                };

                println!(
                    "{} {} ({})",
                    status_icon, yara_result.target_name, yara_result.target_type
                );

                if let Some(metadata) = &yara_result.rule_metadata {
                    let severity = metadata.severity.as_deref().unwrap_or("MEDIUM");
                    let severity_color = match severity {
                        "CRITICAL" => severity.red().bold(),
                        "HIGH" => severity.yellow().bold(),
                        "MEDIUM" => severity.blue().bold(),
                        _ => severity.green().bold(),
                    };

                    println!("  Rule: {} ({})", yara_result.rule_name, severity_color);

                    if let Some(name) = &metadata.name {
                        println!("  Name: {name}");
                    }
                    if let Some(desc) = &metadata.description {
                        println!("  Description: {desc}");
                    }
                    if let Some(author) = &metadata.author {
                        println!("  Author: {author}");
                    }
                    if let Some(version) = &metadata.version {
                        println!("  Version: {version}");
                    }
                    if let Some(confidence) = &metadata.confidence {
                        println!("  Confidence: {confidence}");
                    }
                    if !metadata.tags.is_empty() {
                        println!("  Tags: {}", metadata.tags.join(", "));
                    }
                } else {
                    println!("  Rule: {} (MEDIUM)", yara_result.rule_name);
                }

                if let Some(matched_text) = &yara_result.matched_text {
                    println!("  Matched: {matched_text}");
                }
                println!("  Context: {}", yara_result.context);
                println!();
            }
        }
    }

    // Errors
    if !result.errors.is_empty() {
        println!("\n{}", "Errors".bold().red());
        for error in &result.errors {
            println!("- {error}");
        }
    }
}

#[allow(clippy::too_many_lines)]
fn print_text_result(result: &ScanResult) {
    println!("Scan Result for: {}", result.url);
    println!("Status: {}", format_status(&result.status));
    println!("Response Time: {}ms", result.response_time_ms);

    if let Some(server_info) = &result.server_info {
        println!("Server: {} v{}", server_info.name, server_info.version);
        if let Some(desc) = &server_info.description {
            println!("Description: {desc}");
        }
        if !server_info.capabilities.is_empty() {
            println!("Capabilities: {}", server_info.capabilities.join(", "));
        }
    }

    println!("Tools: {}", result.tools.len());
    for tool in &result.tools {
        println!("  - {}", tool.name);
    }

    println!("Resources: {}", result.resources.len());
    for resource in &result.resources {
        println!("  - {} ({})", resource.name, resource.uri);
    }

    println!("Prompts: {}", result.prompts.len());
    for prompt in &result.prompts {
        println!(
            "  - {} ({})",
            prompt.name,
            prompt.description.as_deref().unwrap_or("No description")
        );
    }

    // Security issues
    if let Some(security_issues) = &result.security_issues {
        println!("Security Issues: {}", security_issues.total_issues());
        if security_issues.has_critical_issues() {
            println!("  ⚠️  CRITICAL ISSUES DETECTED");
        }
        if security_issues.has_high_issues() {
            println!("  ⚠️  HIGH SEVERITY ISSUES DETECTED");
        }

        if !security_issues.tool_issues.is_empty() {
            println!("  Tool Issues: {}", security_issues.tool_issues.len());
            for issue in &security_issues.tool_issues {
                println!("    - {}: {}", issue.severity, issue.message);
            }
        }

        if !security_issues.prompt_issues.is_empty() {
            println!("  Prompt Issues: {}", security_issues.prompt_issues.len());
            for issue in &security_issues.prompt_issues {
                println!("    - {}: {}", issue.severity, issue.message);
            }
        }

        if !security_issues.resource_issues.is_empty() {
            println!(
                "  Resource Issues: {}",
                security_issues.resource_issues.len()
            );
            for issue in &security_issues.resource_issues {
                println!("    - {}: {}", issue.severity, issue.message);
            }
        }
    }

    // YARA scan results
    if !result.yara_results.is_empty() {
        // Separate summary and match results
        let summary_results: Vec<_> = result
            .yara_results
            .iter()
            .filter(|r| r.target_type == "summary")
            .collect();
        let match_results: Vec<_> = result
            .yara_results
            .iter()
            .filter(|r| r.target_type != "summary")
            .collect();

        println!(
            "YARA Scan Results: {} total results",
            result.yara_results.len()
        );

        // Show summary results
        for summary in &summary_results {
            let status = summary.status.as_deref().unwrap_or("unknown");
            println!(
                "  {} - {}: {}",
                summary.target_name.to_uppercase(),
                status.to_uppercase(),
                summary.context
            );
            if let Some(total_matches) = summary.total_matches {
                println!("    Security matches found: {total_matches}");
            }
        }

        // Show individual matches
        for yara_result in &match_results {
            let severity = yara_result
                .rule_metadata
                .as_ref()
                .and_then(|m| m.severity.as_ref())
                .map_or("MEDIUM", String::as_str);
            let status = yara_result.status.as_deref().unwrap_or("unknown");

            println!(
                "    {} ({}): {} - {} [{}]",
                yara_result.target_name,
                yara_result.target_type,
                yara_result.rule_name,
                severity,
                status.to_uppercase()
            );
        }
    }

    if !result.errors.is_empty() {
        println!("Errors:");
        for error in &result.errors {
            println!("  - {error}");
        }
    }
}

fn format_status(status: &ScanStatus) -> String {
    match status {
        ScanStatus::Success => "SUCCESS".green().to_string(),
        ScanStatus::Failed(msg) => format!("FAILED: {msg}").red().to_string(),
        ScanStatus::Timeout => "TIMEOUT".yellow().to_string(),
        ScanStatus::ConnectionError(msg) => format!("CONNECTION ERROR: {msg}").red().to_string(),
        ScanStatus::AuthenticationError(msg) => {
            format!("AUTHENTICATION ERROR: {msg}").red().to_string()
        }
    }
}

#[derive(Tabled)]
struct ResourceRow {
    #[tabled(rename = "URI")]
    uri: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Description")]
    description: String,
    #[tabled(rename = "MIME Type")]
    mime_type: String,
}

/// Enhanced security assessment table with per-tool results
#[allow(clippy::too_many_lines)]
fn print_enhanced_security_table(result: &ScanResult) {
    if let Some(security_issues) = &result.security_issues {
        println!("\n{}", "Security Assessment Results".bold());

        // Get server name
        let server_name = if let Some(server_info) = &result.server_info {
            server_info.name.clone()
        } else {
            "Unknown MCP Server".to_string()
        };

        println!("🌐 {}", server_name.bold());

        let mut total_warnings = 0;
        let mut tools_with_warnings = 0;

        // Count warnings first for summary
        for tool in &result.tools {
            let tool_issues: Vec<&SecurityIssue> = security_issues
                .tool_issues
                .iter()
                .filter(|issue| issue.tool_name.as_ref() == Some(&tool.name))
                .collect();
            if !tool_issues.is_empty() {
                tools_with_warnings += 1;
                total_warnings += tool_issues.len();
            }
        }

        // Show quick summary
        if total_warnings == 0 {
            println!("  ✅ All tools passed security checks");
        } else {
            println!(
                "  ⚠️  {tools_with_warnings} tools have security warnings ({total_warnings} total warnings)"
            );
        }
        println!();

        for tool in &result.tools {
            let tool_issues: Vec<&SecurityIssue> = security_issues
                .tool_issues
                .iter()
                .filter(|issue| issue.tool_name.as_ref() == Some(&tool.name))
                .collect();

            let warning_count = tool_issues.len();

            // Determine overall status for this tool
            let status = if warning_count == 0 {
                "passed".green()
            } else {
                "warning".yellow()
            };

            // Print tool with tree structure
            println!("  └── {} {}", tool.name, status);

            // Show detailed analysis only for tools with issues
            if !tool_issues.is_empty() {
                // Show LLM analysis details for tools with issues
                if let Some(analysis_details) =
                    security_issues.tool_analysis_details.get(&tool.name)
                {
                    println!("      📋 Analysis: {analysis_details}");
                }

                // Show specific security issues
                for issue in tool_issues {
                    let severity_color = match issue.severity.as_str() {
                        "CRITICAL" => issue.severity.red().bold(),
                        "HIGH" => issue.severity.yellow().bold(),
                        "MEDIUM" => issue.severity.blue().bold(),
                        _ => issue.severity.green().bold(),
                    };
                    println!("      ├── {}: {}", severity_color, issue.message);
                    if let Some(details) = &issue.details {
                        println!("      │   Details: {details}");
                    }
                }
            }
        }

        // Show prompt security issues if any
        if !security_issues.prompt_issues.is_empty() {
            println!("\n  📝 Prompts:");
            for issue in &security_issues.prompt_issues {
                let severity_color = match issue.severity.as_str() {
                    "CRITICAL" => issue.severity.red().bold(),
                    "HIGH" => issue.severity.yellow().bold(),
                    "MEDIUM" => issue.severity.blue().bold(),
                    _ => issue.severity.green().bold(),
                };
                println!(
                    "    └── {}: {} ({})",
                    severity_color,
                    issue.message,
                    issue.prompt_name.as_ref().unwrap_or(&"Unknown".to_string())
                );
            }
        }

        // Show resource security issues if any
        if !security_issues.resource_issues.is_empty() {
            println!("\n  📁 Resources:");
            for issue in &security_issues.resource_issues {
                let severity_color = match issue.severity.as_str() {
                    "CRITICAL" => issue.severity.red().bold(),
                    "HIGH" => issue.severity.yellow().bold(),
                    "MEDIUM" => issue.severity.blue().bold(),
                    _ => issue.severity.green().bold(),
                };
                println!(
                    "    └── {}: {} ({})",
                    severity_color,
                    issue.message,
                    issue
                        .resource_uri
                        .as_ref()
                        .unwrap_or(&"Unknown".to_string())
                );
            }
        }

        // Add summary
        println!("\n{}", "Summary:".bold());
        println!("  • Tools scanned: {}", result.tools.len());
        if total_warnings > 0 {
            println!(
                "  • Warnings found: {tools_with_warnings} tools with {total_warnings} total warnings"
            );
        } else {
            println!(
                "  • Status: {} All tools passed security checks",
                "PASSED".green()
            );
        }
    }
}

// ============================================================================
// MULTI-SERVER PRINTING FUNCTIONS
// ============================================================================

/// Print results from multiple MCP servers
pub fn print_multi_server_results(results: &[ScanResult], format: &str, detailed: bool) {
    match format.to_lowercase().as_str() {
        "json" => print_multi_server_json(results),
        "table" => print_multi_server_tree(results, detailed),
        "text" => print_multi_server_text(results),
        "raw" => print_multi_server_raw_json(results),
        _ => {
            eprintln!("Unknown format: {format}. Using tree view format.");
            print_multi_server_tree(results, detailed);
        }
    }
}

/// Enhanced tree view for multiple MCP servers grouped by IDE
#[allow(clippy::too_many_lines)]
fn print_multi_server_tree(results: &[ScanResult], _detailed: bool) {
    // Group results by IDE source
    let mut results_by_ide: std::collections::HashMap<String, Vec<&ScanResult>> =
        std::collections::HashMap::new();

    for result in results {
        let ide_name = result
            .ide_source
            .as_deref()
            .unwrap_or("UNKNOWN IDE")
            .to_string();
        results_by_ide.entry(ide_name).or_default().push(result);
    }

    // Print overall summary header
    println!("\n{}", "🌍 MCP Servers Security Scan Summary".bold());
    println!("{}", "─".repeat(60));

    let total_servers = results.len();
    let mut successful_servers = 0;
    let mut failed_servers = 0;
    let mut total_tools = 0;
    let mut total_resources = 0;
    let mut total_prompts = 0;
    let mut total_warnings = 0;
    let mut servers_with_warnings = 0;

    // Calculate summary statistics
    for result in results {
        match &result.status {
            crate::types::ScanStatus::Success => successful_servers += 1,
            _ => failed_servers += 1,
        }

        total_tools += result.tools.len();
        total_resources += result.resources.len();
        total_prompts += result.prompts.len();

        if let Some(security_issues) = &result.security_issues {
            let server_warnings = security_issues.tool_issues.len()
                + security_issues.prompt_issues.len()
                + security_issues.resource_issues.len();
            if server_warnings > 0 {
                servers_with_warnings += 1;
                total_warnings += server_warnings;
            }
        }
    }

    // Print scan summary statistics
    println!("📊 Scan Summary:");
    println!(
        "  • Servers: {total_servers} total ({successful_servers} ✅ successful, {failed_servers} ❌ failed)"
    );
    println!(
        "  • Resources: {total_tools} tools, {total_resources} resources, {total_prompts} prompts"
    );
    if total_warnings > 0 {
        println!(
            "  • Security: ⚠️  {servers_with_warnings} servers with {total_warnings} total warnings"
        );
    } else {
        println!("  • Security: ✅ All servers passed security checks");
    }

    println!("\n{}", "📋 Results by IDE:".bold());

    // Sort IDE names for consistent output
    let mut ide_names: Vec<&String> = results_by_ide.keys().collect();
    ide_names.sort();

    // Print results grouped by IDE
    for (ide_index, ide_name) in ide_names.iter().enumerate() {
        let ide_results = &results_by_ide[*ide_name];
        let is_last_ide = ide_index == ide_names.len() - 1;
        let ide_prefix = if is_last_ide {
            "└── "
        } else {
            "├── "
        };
        let ide_continuation = if is_last_ide { "    " } else { "│   " };

        println!("\n{}{} {}", ide_prefix, "🏢".bold(), ide_name.bold());

        // Print each server within this IDE
        for (i, result) in ide_results.iter().enumerate() {
            let is_last = i == ide_results.len() - 1;
            let prefix = if is_last { "└── " } else { "├── " };
            let continuation = if is_last { "    " } else { "│   " };

            // Server name and status
            let server_name = result
                .server_info
                .as_ref()
                .map_or_else(|| "Unknown Server".to_string(), |info| info.name.clone());

            let status_emoji = match &result.status {
                crate::types::ScanStatus::Success => "✅",
                crate::types::ScanStatus::Failed(_) => "❌",
                crate::types::ScanStatus::Timeout => "⏱️ ",
                crate::types::ScanStatus::ConnectionError(_) => "🔌",
                crate::types::ScanStatus::AuthenticationError(_) => "🔐",
            };

            println!(
                "{}{}{} {} ({})",
                ide_continuation,
                prefix,
                status_emoji,
                server_name.bold(),
                result.url
            );

            // Show basic stats and detailed results for successful scans
            if matches!(result.status, crate::types::ScanStatus::Success) {
                println!(
                    "{}{}📋 {} tools, {} resources, {} prompts",
                    ide_continuation,
                    continuation,
                    result.tools.len(),
                    result.resources.len(),
                    result.prompts.len()
                );

                // Show individual tool results with security status
                if !result.tools.is_empty() {
                    println!("{ide_continuation}{continuation}🔧 Tools:");
                    if let Some(security_issues) = &result.security_issues {
                        for tool in &result.tools {
                            let tool_issues: Vec<&crate::security::SecurityIssue> = security_issues
                                .tool_issues
                                .iter()
                                .filter(|issue| issue.tool_name.as_ref() == Some(&tool.name))
                                .collect();

                            if tool_issues.is_empty() {
                                println!(
                                    "{}{}    ├── {} ✅",
                                    ide_continuation, continuation, tool.name
                                );
                            } else {
                                println!(
                                    "{}{}    ├── {} ⚠️  {} warning{}",
                                    ide_continuation,
                                    continuation,
                                    tool.name.bold(),
                                    tool_issues.len(),
                                    if tool_issues.len() == 1 { "" } else { "s" }
                                );
                                for (i, issue) in tool_issues.iter().enumerate() {
                                    let severity = issue.issue_type.default_severity();
                                    let severity_color = match severity {
                                        "CRITICAL" => "🔴 CRITICAL".red(),
                                        "HIGH" => "🟠 HIGH".red(),
                                        "MEDIUM" => "🟡 MEDIUM".yellow(),
                                        _ => "🟢 LOW".green(),
                                    };
                                    let item_prefix = if i == tool_issues.len() - 1 {
                                        "└──"
                                    } else {
                                        "├──"
                                    };
                                    println!(
                                        "{}{}    │   {} {}: {}",
                                        ide_continuation,
                                        continuation,
                                        item_prefix,
                                        severity_color,
                                        issue.description
                                    );
                                }
                            }
                        }
                    } else {
                        // No security analysis available
                        for tool in &result.tools {
                            println!(
                                "{}{}    ├── {} ✅",
                                ide_continuation, continuation, tool.name
                            );
                        }
                    }
                }

                // Show resources if any
                if !result.resources.is_empty() {
                    println!("{ide_continuation}{continuation}📄 Resources:");
                    for resource in &result.resources {
                        println!(
                            "{}{}    ├── {} ✅",
                            ide_continuation, continuation, resource.name
                        );
                    }
                }

                // Show prompts if any
                if !result.prompts.is_empty() {
                    println!("{ide_continuation}{continuation}💬 Prompts:");
                    for prompt in &result.prompts {
                        println!(
                            "{}{}    ├── {} ✅",
                            ide_continuation, continuation, prompt.name
                        );
                    }
                }

                // YARA scan results summary
                if !result.yara_results.is_empty() {
                    let yara_matches: usize = result
                        .yara_results
                        .iter()
                        .map(|r| r.total_matches.unwrap_or(0))
                        .sum();
                    if yara_matches > 0 {
                        println!(
                            "{ide_continuation}{continuation}⚠️  YARA: {yara_matches} security issues detected"
                        );
                    } else {
                        println!(
                            "{ide_continuation}{continuation}✅ YARA: No security issues detected"
                        );
                    }
                }
            } else {
                // Show error details for failed scans
                match &result.status {
                    crate::types::ScanStatus::Failed(err) => {
                        println!("{ide_continuation}{continuation}❌ Error: {err}");
                    }
                    crate::types::ScanStatus::ConnectionError(err) => {
                        println!("{ide_continuation}{continuation}🔌 Connection Error: {err}");
                    }
                    crate::types::ScanStatus::AuthenticationError(err) => {
                        println!("{ide_continuation}{continuation}🔐 Authentication Error: {err}");
                    }
                    crate::types::ScanStatus::Timeout => {
                        println!("{ide_continuation}{continuation}⏱️  Timeout occurred");
                    }
                    crate::types::ScanStatus::Success => {}
                }
            }
        }
    }

    println!(); // Final newline
}

/// JSON format for multiple servers
fn print_multi_server_json(results: &[ScanResult]) {
    let json_output = serde_json::json!({
        "scan_type": "multi_server",
        "total_servers": results.len(),
        "results": results
    });

    match serde_json::to_string_pretty(&json_output) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Error serializing multi-server results to JSON: {e}"),
    }
}

/// Raw JSON format for multiple servers
fn print_multi_server_raw_json(results: &[ScanResult]) {
    for result in results {
        print_raw_json_result(result);
    }
}

/// Text format for multiple servers
fn print_multi_server_text(results: &[ScanResult]) {
    println!("Multi-Server Scan Results");
    println!("========================");
    println!("Total servers scanned: {}", results.len());
    println!();

    for (i, result) in results.iter().enumerate() {
        println!("Server {}: {}", i + 1, result.url);
        print_text_result(result);
        println!();
    }
}

/// Generate a detailed markdown report from scan results
#[allow(clippy::too_many_lines)]
pub fn generate_markdown_report(results: &[ScanResult]) -> Result<String> {
    use chrono::Utc;
    use std::fmt::Write;

    let timestamp = Utc::now();
    let mut report = String::new();

    // Header
    writeln!(report, "# MCP Security Scan Report")?;
    writeln!(report)?;
    writeln!(
        report,
        "**Generated:** {}",
        timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    )?;
    writeln!(report, "**Scanner:** Ramparts MCP Security Scanner")?;
    writeln!(report)?;

    // Executive Summary
    writeln!(report, "## Executive Summary")?;
    writeln!(report)?;

    let total_servers = results.len();
    let successful_scans = results
        .iter()
        .filter(|r| matches!(r.status, ScanStatus::Success))
        .count();
    let failed_scans = total_servers - successful_scans;

    // Count security issues by severity
    let mut critical_count = 0;
    let mut high_count = 0;
    let mut medium_count = 0;
    let mut low_count = 0;

    for result in results {
        if let Some(ref security_issues) = result.security_issues {
            for issue in &security_issues.tool_issues {
                match issue.severity.as_str() {
                    "CRITICAL" => critical_count += 1,
                    "HIGH" => high_count += 1,
                    "MEDIUM" => medium_count += 1,
                    _ => low_count += 1,
                }
            }
            for issue in &security_issues.prompt_issues {
                match issue.severity.as_str() {
                    "CRITICAL" => critical_count += 1,
                    "HIGH" => high_count += 1,
                    "MEDIUM" => medium_count += 1,
                    _ => low_count += 1,
                }
            }
            for issue in &security_issues.resource_issues {
                match issue.severity.as_str() {
                    "CRITICAL" => critical_count += 1,
                    "HIGH" => high_count += 1,
                    "MEDIUM" => medium_count += 1,
                    _ => low_count += 1,
                }
            }
        }
    }

    writeln!(report, "- **Total Servers Scanned:** {total_servers}")?;
    writeln!(report, "- **Successful Scans:** {successful_scans}")?;
    writeln!(report, "- **Failed Scans:** {failed_scans}")?;
    writeln!(report)?;
    writeln!(report, "### Security Issues Summary")?;
    if critical_count + high_count + medium_count + low_count == 0 {
        writeln!(report, "✅ **No security issues detected**")?;
    } else {
        writeln!(report, "| Severity | Count |")?;
        writeln!(report, "|----------|-------|")?;
        writeln!(report, "| 🔴 **CRITICAL** | {critical_count} |")?;
        writeln!(report, "| 🟠 **HIGH** | {high_count} |")?;
        writeln!(report, "| 🟡 **MEDIUM** | {medium_count} |")?;
        writeln!(report, "| 🟢 **LOW** | {low_count} |")?;
    }
    writeln!(report)?;

    // Detailed Results
    writeln!(report, "## Detailed Scan Results")?;

    for (i, result) in results.iter().enumerate() {
        writeln!(report, "### Server {} - {}", i + 1, result.url)?;

        // Server Info
        if let Some(ref server_info) = result.server_info {
            writeln!(report, "**Server Information:**")?;
            writeln!(report, "- **Name:** {}", server_info.name)?;
            writeln!(report, "- **Version:** {}", server_info.version)?;
            if let Some(ref description) = server_info.description {
                writeln!(report, "- **Description:** {description}")?;
            }
        }

        // Scan Status
        match &result.status {
            ScanStatus::Success => {
                writeln!(report, "**Status:** ✅ Success")?;
                writeln!(report, "**Response Time:** {}ms", result.response_time_ms)?;
            }
            ScanStatus::Failed(error) => {
                writeln!(report, "**Status:** ❌ Failed")?;
                writeln!(report, "**Error:** {error}")?;
                writeln!(report)?;
                continue;
            }
            ScanStatus::Timeout => {
                writeln!(report, "**Status:** ⏰ Timeout")?;
                writeln!(report)?;
                continue;
            }
            ScanStatus::ConnectionError(error) => {
                writeln!(report, "**Status:** ❌ Connection Error")?;
                writeln!(report, "**Error:** {error}")?;
                writeln!(report)?;
                continue;
            }
            ScanStatus::AuthenticationError(error) => {
                writeln!(report, "**Status:** 🔐 Authentication Error")?;
                writeln!(report, "**Error:** {error}")?;
                writeln!(report)?;
                continue;
            }
        }
        writeln!(report)?;

        // Tools - only show tools with security issues
        if let Some(ref security_issues) = result.security_issues {
            let tools_with_issues: Vec<_> = result
                .tools
                .iter()
                .filter(|tool| {
                    security_issues
                        .tool_issues
                        .iter()
                        .any(|issue| issue.tool_name.as_ref() == Some(&tool.name))
                })
                .collect();

            if !tools_with_issues.is_empty() {
                writeln!(
                    report,
                    "#### Tools with Security Issues ({} of {} total)",
                    tools_with_issues.len(),
                    result.tools.len()
                )?;
                writeln!(report)?;

                for tool in tools_with_issues {
                    writeln!(report, "##### {}", tool.name)?;

                    if let Some(ref description) = tool.description {
                        writeln!(report, "{description}")?;
                        writeln!(report)?;
                    }

                    let tool_issues: Vec<_> = security_issues
                        .tool_issues
                        .iter()
                        .filter(|issue| issue.tool_name.as_ref() == Some(&tool.name))
                        .collect();

                    writeln!(report, "**Security Issues:**")?;
                    for issue in tool_issues {
                        let severity_emoji = match issue.severity.as_str() {
                            "CRITICAL" => "🔴",
                            "HIGH" => "🟠",
                            "MEDIUM" => "🟡",
                            _ => "🟢",
                        };
                        writeln!(
                            report,
                            "- {} **{}:** {}",
                            severity_emoji, issue.severity, issue.description
                        )?;
                        if let Some(ref details) = issue.details {
                            writeln!(report, "  - *Details:* {details}")?;
                        }
                    }
                    writeln!(report)?;
                }
            } else if !result.tools.is_empty() {
                writeln!(report, "#### Tools")?;
                writeln!(report)?;
                writeln!(
                    report,
                    "✅ All {} tools passed security checks",
                    result.tools.len()
                )?;
                writeln!(report)?;
            }
        } else if !result.tools.is_empty() {
            writeln!(report, "#### Tools")?;
            writeln!(report)?;
            writeln!(
                report,
                "⚠️ {} tools found but no security analysis available",
                result.tools.len()
            )?;
            writeln!(report)?;
        }

        // Resources - only show resources with security issues
        if let Some(ref security_issues) = result.security_issues {
            let resources_with_issues: Vec<_> = result
                .resources
                .iter()
                .filter(|resource| {
                    security_issues
                        .resource_issues
                        .iter()
                        .any(|issue| issue.resource_uri.as_ref() == Some(&resource.uri))
                })
                .collect();

            if !resources_with_issues.is_empty() {
                writeln!(
                    report,
                    "#### Resources with Security Issues ({} of {} total)",
                    resources_with_issues.len(),
                    result.resources.len()
                )?;
                writeln!(report)?;

                for resource in resources_with_issues {
                    writeln!(report, "##### {}", resource.name)?;
                    writeln!(report, "- **URI:** {}", resource.uri)?;
                    if let Some(ref description) = resource.description {
                        writeln!(report, "- **Description:** {description}")?;
                    }

                    let resource_issues: Vec<_> = security_issues
                        .resource_issues
                        .iter()
                        .filter(|issue| issue.resource_uri.as_ref() == Some(&resource.uri))
                        .collect();

                    writeln!(report)?;
                    writeln!(report, "**Security Issues:**")?;
                    for issue in resource_issues {
                        let severity_emoji = match issue.severity.as_str() {
                            "CRITICAL" => "🔴",
                            "HIGH" => "🟠",
                            "MEDIUM" => "🟡",
                            _ => "🟢",
                        };
                        writeln!(
                            report,
                            "- {} **{}:** {}",
                            severity_emoji, issue.severity, issue.description
                        )?;
                    }
                    writeln!(report)?;
                }
            } else if !result.resources.is_empty() {
                writeln!(report, "#### Resources")?;
                writeln!(report)?;
                writeln!(
                    report,
                    "✅ All {} resources passed security checks",
                    result.resources.len()
                )?;
                writeln!(report)?;
            }
        } else if !result.resources.is_empty() {
            writeln!(report, "#### Resources")?;
            writeln!(report)?;
            writeln!(
                report,
                "⚠️ {} resources found but no security analysis available",
                result.resources.len()
            )?;
            writeln!(report)?;
        }

        // Prompts - only show prompts with security issues
        if let Some(ref security_issues) = result.security_issues {
            let prompts_with_issues: Vec<_> = result
                .prompts
                .iter()
                .filter(|prompt| {
                    security_issues
                        .prompt_issues
                        .iter()
                        .any(|issue| issue.prompt_name.as_ref() == Some(&prompt.name))
                })
                .collect();

            if !prompts_with_issues.is_empty() {
                writeln!(
                    report,
                    "#### Prompts with Security Issues ({} of {} total)",
                    prompts_with_issues.len(),
                    result.prompts.len()
                )?;
                writeln!(report)?;

                for prompt in prompts_with_issues {
                    writeln!(report, "##### {}", prompt.name)?;
                    if let Some(ref description) = prompt.description {
                        writeln!(report, "{description}")?;
                        writeln!(report)?;
                    }

                    let prompt_issues: Vec<_> = security_issues
                        .prompt_issues
                        .iter()
                        .filter(|issue| issue.prompt_name.as_ref() == Some(&prompt.name))
                        .collect();

                    writeln!(report, "**Security Issues:**")?;
                    for issue in prompt_issues {
                        let severity_emoji = match issue.severity.as_str() {
                            "CRITICAL" => "🔴",
                            "HIGH" => "🟠",
                            "MEDIUM" => "🟡",
                            _ => "🟢",
                        };
                        writeln!(
                            report,
                            "- {} **{}:** {}",
                            severity_emoji, issue.severity, issue.description
                        )?;
                    }
                    writeln!(report)?;
                }
            } else if !result.prompts.is_empty() {
                writeln!(report, "#### Prompts")?;
                writeln!(report)?;
                writeln!(
                    report,
                    "✅ All {} prompts passed security checks",
                    result.prompts.len()
                )?;
                writeln!(report)?;
            }
        } else if !result.prompts.is_empty() {
            writeln!(report, "#### Prompts")?;
            writeln!(report)?;
            writeln!(
                report,
                "⚠️ {} prompts found but no security analysis available",
                result.prompts.len()
            )?;
            writeln!(report)?;
        }

        writeln!(report, "---")?;
        writeln!(report)?;
    }

    // Footer
    writeln!(report, "## Report Information")?;
    writeln!(report)?;
    writeln!(report, "This report was generated by [Ramparts](https://github.com/getjavelin/ramparts), an MCP security scanner.")?;
    writeln!(report)?;
    writeln!(report, "For more information about security issues and remediation, see the [Ramparts documentation](https://github.com/getjavelin/ramparts/blob/main/docs/security-features.md).")?;

    Ok(report)
}

/// Write markdown report to file with timestamp
pub fn write_markdown_report(results: &[ScanResult]) -> Result<String> {
    let report_content = generate_markdown_report(results)?;
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("scan_{timestamp}.md");

    let mut file = File::create(&filename)
        .map_err(|e| anyhow!("Failed to create report file {}: {}", filename, e))?;

    file.write_all(report_content.as_bytes())
        .map_err(|e| anyhow!("Failed to write report to {}: {}", filename, e))?;

    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::{error_utils, format_status, Timer};
    use anyhow::anyhow;

    #[test]
    fn test_timer_functionality() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();

        assert!(elapsed >= 10);
        println!("Timer elapsed: {elapsed}ms");
    }

    #[test]
    fn test_error_utils() {
        // Test format_error
        let error_msg = error_utils::format_error("Test operation", "Something went wrong");
        assert_eq!(error_msg, "Test operation failed: Something went wrong");

        // Test wrap_error with success
        let result: Result<i32, anyhow::Error> = Ok(42);
        let wrapped = error_utils::wrap_error(result, "Test context");
        assert!(wrapped.is_ok());
        assert_eq!(wrapped.unwrap(), 42);

        // Test wrap_error with failure
        let result: Result<i32, anyhow::Error> = Err(anyhow!("Original error"));
        let wrapped = error_utils::wrap_error(result, "Test context");
        assert!(wrapped.is_err());
        let error_msg = wrapped.unwrap_err().to_string();
        assert!(error_msg.contains("Test context"));
        assert!(error_msg.contains("Original error"));
    }

    // TODO: Add test for track_performance when async closure type inference is resolved

    #[test]
    fn test_format_status() {
        use crate::types::ScanStatus;

        let success = format_status(&ScanStatus::Success);
        assert!(success.contains("SUCCESS"));

        let failed = format_status(&ScanStatus::Failed("Test error".to_string()));
        assert!(failed.contains("FAILED"));
        assert!(failed.contains("Test error"));

        let timeout = format_status(&ScanStatus::Timeout);
        assert!(timeout.contains("TIMEOUT"));

        let connection_error = format_status(&ScanStatus::ConnectionError(
            "Connection failed".to_string(),
        ));
        assert!(connection_error.contains("CONNECTION ERROR"));
        assert!(connection_error.contains("Connection failed"));

        let auth_error = format_status(&ScanStatus::AuthenticationError("Auth failed".to_string()));
        assert!(auth_error.contains("AUTHENTICATION ERROR"));
        assert!(auth_error.contains("Auth failed"));
    }
}
