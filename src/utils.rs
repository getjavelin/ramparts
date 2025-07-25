use crate::security::SecurityIssue;
use crate::types::*;
use anyhow::{anyhow, Result};
use colored::*;
use std::time::Instant;
use tracing::warn;

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
        self.start_time.elapsed().as_millis() as u64
    }
}

/// Enhanced error handling utilities
pub mod error_utils {
    use super::*;

    /// Create a standardized error message
    pub fn create_error_msg(operation: &str, details: &str) -> String {
        format!("{operation} failed: {details}")
    }

    /// Wrap an error with context
    pub fn wrap_error<T>(result: Result<T>, context: &str) -> Result<T> {
        result.map_err(|e| anyhow!("{context}: {e}"))
    }
}

/// Generic array response parser
pub fn parse_jsonrpc_array_response<T>(
    response: &serde_json::Value,
    result_key: &str,
) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    let array = response["result"][result_key]
        .as_array()
        .ok_or_else(|| anyhow!("Invalid {result_key} response format"))?;

    let mut items = Vec::new();
    for item_value in array {
        let item: T = serde_json::from_value(item_value.clone())
            .map_err(|e| anyhow!("Failed to parse {result_key} item: {e}"))?;
        items.push(item);
    }

    Ok(items)
}

/// Retry utility for HTTP requests
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    max_retries: usize,
    initial_delay_ms: u64,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    use tokio::time::{sleep, Duration};

    let mut last_err = None;
    let mut delay = Duration::from_millis(initial_delay_ms);

    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < max_retries {
                    sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("Operation failed after {} retries", max_retries)))
}

/// Performance monitoring utilities
pub mod performance {
    use super::*;

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
            if elapsed > 1000 {
                warn!(
                    "Slow operation detected: {} took {}ms",
                    self.operation_name, elapsed
                );
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
    let json = serde_json::to_string_pretty(result).unwrap();
    println!("{json}");
}

fn print_raw_json_result(result: &ScanResult) {
    // Create a raw JSON structure that preserves the original MCP server schema
    let mut raw_result = serde_json::Map::new();

    // Add basic scan info
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

    // Add server info if available
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

    // Add tools with their raw JSON schema
    if !result.tools.is_empty() {
        let tools_array = result
            .tools
            .iter()
            .map(|tool| {
                if let Some(ref raw_json) = tool.raw_json {
                    raw_json.clone()
                } else {
                    // Fallback to our parsed structure if raw JSON is not available
                    serde_json::to_value(tool).unwrap_or(serde_json::Value::Null)
                }
            })
            .collect();
        raw_result.insert("tools".to_string(), serde_json::Value::Array(tools_array));
    }

    // Add resources with their raw JSON schema
    if !result.resources.is_empty() {
        let resources_array = result
            .resources
            .iter()
            .map(|resource| {
                if let Some(ref raw_json) = resource.raw_json {
                    raw_json.clone()
                } else {
                    // Fallback to our parsed structure if raw JSON is not available
                    serde_json::to_value(resource).unwrap_or(serde_json::Value::Null)
                }
            })
            .collect();
        raw_result.insert(
            "resources".to_string(),
            serde_json::Value::Array(resources_array),
        );
    }

    // Add prompts with their raw JSON schema
    if !result.prompts.is_empty() {
        let prompts_array = result
            .prompts
            .iter()
            .map(|prompt| {
                if let Some(ref raw_json) = prompt.raw_json {
                    raw_json.clone()
                } else {
                    // Fallback to our parsed structure if raw JSON is not available
                    serde_json::to_value(prompt).unwrap_or(serde_json::Value::Null)
                }
            })
            .collect();
        raw_result.insert(
            "prompts".to_string(),
            serde_json::Value::Array(prompts_array),
        );
    }

    // Add YARA scan results if any
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

    // Add errors if any
    if !result.errors.is_empty() {
        let errors_array = result
            .errors
            .iter()
            .map(|e| serde_json::Value::String(e.clone()))
            .collect();
        raw_result.insert("errors".to_string(), serde_json::Value::Array(errors_array));
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(raw_result)).unwrap();
    println!("{json}");
}

fn print_table_result(result: &ScanResult, detailed: bool) {
    println!("{}", "=".repeat(80));
    println!("MCP Server Scan Result");
    println!("{}", "=".repeat(80));

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
                println!("{}", "=".repeat(60));
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
        .to_string();
        println!("{resource_table}");
    }

    // Prompts
    if !result.prompts.is_empty() {
        println!("\n{}", "Prompts".bold());
        let prompt_table = Table::new(result.prompts.iter().map(|p| PromptRow {
            name: p.name.clone(),
            description: p.description.clone().unwrap_or_else(|| "N/A".to_string()),
            arguments: p.arguments.as_ref().map(|args| args.len()).unwrap_or(0),
        }))
        .to_string();
        println!("{prompt_table}");
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
    if !result.yara_results.is_empty() {
        println!("\n{}", "YARA Scan Results".bold());
        println!("{}", "=".repeat(80));

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
            if let Some(rules_executed) = &summary.rules_executed {
                if !rules_executed.is_empty() && rules_executed[0] != "none" {
                    println!("  Rules executed: {}", rules_executed.join(", "));
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
    } else {
        // Show YARA execution status even when no results at all
        println!("\n{}", "YARA Scan Results".bold());
        println!("{}", "=".repeat(80));
        println!("❌ YARA scanning not executed or no results available");
        println!();
    }

    // Errors
    if !result.errors.is_empty() {
        println!("\n{}", "Errors".bold().red());
        for error in &result.errors {
            println!("- {error}");
        }
    }

    println!("{}", "=".repeat(80));
}

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
                .map(|s| s.as_str())
                .unwrap_or("MEDIUM");
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

#[derive(Tabled)]
struct PromptRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Description")]
    description: String,
    #[tabled(rename = "Arguments")]
    arguments: usize,
}

/// Enhanced security assessment table with per-tool results
fn print_enhanced_security_table(result: &ScanResult) {
    if let Some(security_issues) = &result.security_issues {
        println!("\n{}", "Security Assessment Results".bold());
        println!("{}", "=".repeat(80));

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
