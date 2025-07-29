use crate::config::MCPConfigManager;
use crate::constants::{messages, protocol};
use crate::security::{
    cross_origin_scanner::CrossOriginScanner, SecurityScanResult, SecurityScanner,
};
use crate::types::{YaraScanResult, *};
use crate::utils::{
    error_utils, parse_jsonrpc_array_response, performance::track_performance, retry_with_backoff,
    Timer,
};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};
use url::Url;

#[cfg(feature = "yara-x-scanning")]
use yara_x::Rules;

#[cfg(feature = "yara-x-scanning")]
type YaraRules = Rules;

#[cfg(not(feature = "yara-x-scanning"))]
type YaraRules = ();

/// Map rule names to their file names for consistent naming
fn rule_name_to_file_name(rule_name: &str) -> Option<String> {
    match rule_name {
        // secrets_leakage.yar rules
        "SecretsLeakage" | "SSHKeyExposure" | "PEMFileAccess" | "EnvironmentVariableLeakage" => {
            Some("secrets_leakage".to_string())
        }
        // cross_origin_escalation.yar rules
        "CrossOriginEscalation"
        | "CrossDomainContamination"
        | "DomainOutlier"
        | "MixedSecuritySchemes" => Some("cross_origin_escalation".to_string()),
        // Add more mappings as needed
        _ => None,
    }
}

/// Sanitize header values for safe logging - prevents credential exposure
fn sanitize_header_for_logging(key: &str, value: &str) -> String {
    let key_lower = key.to_lowercase();

    // Comprehensive list of sensitive header patterns
    if key_lower.contains("auth")
        || key_lower.contains("key")
        || key_lower.contains("secret")
        || key_lower.contains("token")
        || key_lower.contains("bearer")
        || key_lower.contains("password")
        || key_lower.contains("credential")
        || key_lower.contains("session")
        || key_lower.contains("cookie")
        || key_lower.starts_with("x-api")
        || key_lower.starts_with("x-secret")
        || key_lower.starts_with("x-auth")
    {
        "[REDACTED]".to_string()
    } else {
        // Additional check for common header names
        match key_lower.as_str() {
            "authorization" | "x-api-key" | "x-secret-key" | "x-auth-token" | "x-session-token"
            | "api-key" | "apikey" | "secret" | "token" => "[REDACTED]".to_string(),
            _ => {
                // Final safety check: if value looks like a credential (long alphanumeric string)
                if value.len() > 20
                    && value
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    "[REDACTED]".to_string()
                } else {
                    value.to_string()
                }
            }
        }
    }
}

/// Generate descriptive context messages based on rule names
fn generate_context_message(item_type: &str, rule_name: &str) -> String {
    match rule_name {
        // Secrets leakage rules
        "SecretsLeakage" => format!("Potential secret exposure detected in {item_type}"),
        "SSHKeyExposure" => format!(
            "SSH key or configuration file access detected in {item_type}"
        ),
        "PEMFileAccess" => format!(
            "PEM certificate or private key access detected in {item_type}"
        ),
        "EnvironmentVariableLeakage" => format!(
            "Sensitive environment variable pattern detected in {item_type}"
        ),

        // Cross-origin rules
        "CrossOriginEscalation" => format!(
            "Cross-origin escalation vulnerability detected in {item_type}"
        ),
        "CrossDomainContamination" => format!(
            "Cross-domain contamination detected across multiple domains in {item_type}"
        ),
        "DomainOutlier" => format!(
            "Domain outlier detected - {item_type} uses different domain than majority"
        ),
        "MixedSecuritySchemes" => format!("Mixed HTTP/HTTPS schemes detected in {item_type}"),

        // Default fallback
        _ => format!("{item_type} matched by security rule {rule_name}"),
    }
}

/// Check if YARA is available and enabled
fn is_yara_available(config_enabled: bool) -> bool {
    if !config_enabled {
        return false;
    }

    #[cfg(feature = "yara-x-scanning")]
    {
        true
    }

    #[cfg(not(feature = "yara-x-scanning"))]
    {
        false
    }
}

/// Print friendly message about YARA installation
fn print_yara_install_message() {
    println!("📋 YARA-X Scanning Disabled");
    println!();
    println!("YARA-X rule scanning is enabled in your config but YARA-X is not available.");
    println!("To enable YARA-X scanning, please:");
    println!();
    println!("1. Reinstall ramparts with YARA-X support:");
    println!("   cargo install ramparts --force");
    println!();
    println!("2. Or disable YARA-X in your config.yaml:");
    println!("   scanner:");
    println!("     enable_yara: false");
    println!();
    println!("Continuing without YARA-X scanning...");
    println!();
}

// ============================================================================
// TRANSPORT LAYER - Support for multiple MCP transport mechanisms
// ============================================================================

/// Transport type for MCP communication
#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    Http,
    Stdio,
}

impl TransportType {
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("stdio://") || url.contains("|") || Path::new(url).exists() {
            TransportType::Stdio
        } else {
            TransportType::Http
        }
    }
}

/// STDIO transport implementation for MCP servers
pub struct STDIOTransport {
    command: String,
    args: Vec<String>,
    process: Option<tokio::process::Child>,
}

impl STDIOTransport {
    pub fn new(command: &str) -> Result<Self> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Empty command for STDIO transport"));
        }

        let cmd = parts[0].to_string();
        let args = if parts.len() > 1 {
            parts[1..].iter().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            command: cmd,
            args,
            process: None,
        })
    }

    pub fn from_stdio_url(url: &str) -> Result<Self> {
        // Handle different STDIO URL formats:
        // - stdio:///path/to/executable
        // - stdio:///path/to/executable --arg1 --arg2
        // - /path/to/executable
        // - executable --arg1 --arg2

        let command = if url.starts_with("stdio://") {
            url.strip_prefix("stdio://").unwrap_or(url)
        } else {
            url
        };

        Self::new(command)
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        self.process = Some(child);

        info!("Started STDIO process: {} {:?}", self.command, self.args);
        Ok(())
    }

    pub async fn send_request(&mut self, request: Value) -> Result<Value> {
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| anyhow!("STDIO process not started"))?;

        let stdin = process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("STDIO stdin not available"))?;

        let stdout = process
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow!("STDIO stdout not available"))?;

        // Send JSON-RPC request with newline delimiter
        let request_str = serde_json::to_string(&request)?;
        stdin
            .write_all(format!("{request_str}\n").as_bytes())
            .await?;
        stdin.flush().await?;

        // Read response with timeout
        let mut buffer = Vec::new();
        let mut response_buffer = [0u8; 4096];

        loop {
            match tokio::time::timeout(Duration::from_secs(30), stdout.read(&mut response_buffer))
                .await
            {
                Ok(Ok(n)) => {
                    if n == 0 {
                        break; // EOF
                    }
                    buffer.extend_from_slice(&response_buffer[..n]);

                    // Check if we have a complete JSON response
                    if let Ok(response_str) = String::from_utf8(buffer.clone()) {
                        for line in response_str.lines() {
                            if !line.trim().is_empty() {
                                if let Ok(json_response) = serde_json::from_str::<Value>(line) {
                                    debug!(
                                        "STDIO response: {}",
                                        serde_json::to_string_pretty(&json_response)
                                            .unwrap_or_default()
                                    );
                                    return Ok(json_response);
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => return Err(anyhow!("STDIO read error: {}", e)),
                Err(_) => return Err(anyhow!("STDIO read timeout")),
            }
        }

        Err(anyhow!("No valid JSON-RPC response received from STDIO"))
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
            info!("STDIO process terminated");
        }
        Ok(())
    }
}

impl Drop for STDIOTransport {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Spawn a task to handle the async kill operation
            // We can't await in Drop, so we spawn and forget
            tokio::spawn(async move {
                let _ = process.kill().await;
            });
        }
    }
}

// ============================================================================
// SCAN CAPABILITIES - Middleware-like system for extensible scanning
// ============================================================================

// Optimized request builder to reduce redundancy
#[derive(Debug, Clone)]
struct JsonRpcRequest {
    method: &'static str,
    id: u64,
    params: serde_json::Value,
}

impl JsonRpcRequest {
    fn new(method: &'static str, id: u64) -> Self {
        Self {
            method,
            id,
            params: json!({}),
        }
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "jsonrpc": protocol::JSONRPC_VERSION,
            "id": self.id,
            "method": self.method,
            "params": self.params
        })
    }
}

// 1. Migrate protocol fetch logic to simple async methods on MCPScanner
impl MCPScanner {
    /// Fetches the list of tools from the MCP server.
    pub async fn fetch_tools(
        &self,
        url: &str,
        options: &ScanOptions,
        transport_type: &TransportType,
        session: &MCPSession,
    ) -> Result<Vec<MCPTool>> {
        let request = JsonRpcRequest::new("tools/list", 2).to_json();
        let (response, _) = self
            .send_jsonrpc_request_with_headers_and_session(
                url,
                request,
                options,
                transport_type,
                session.session_id.clone(),
            )
            .await?;
        let tool_response = ToolResponse::from_json_response(&response)?;
        Ok(tool_response.tools)
    }
    /// Fetches the list of resources from the MCP server.
    pub async fn fetch_resources(
        &self,
        url: &str,
        options: &ScanOptions,
        transport_type: &TransportType,
        session: &MCPSession,
    ) -> Result<Vec<MCPResource>> {
        let request = JsonRpcRequest::new("resources/list", 3).to_json();
        let (response, _) = self
            .send_jsonrpc_request_with_headers_and_session(
                url,
                request,
                options,
                transport_type,
                session.session_id.clone(),
            )
            .await?;
        let resources = parse_jsonrpc_array_response::<MCPResource>(&response, "resources")?;
        Ok(resources)
    }
    /// Fetches the list of prompts from the MCP server.
    pub async fn fetch_prompts(
        &self,
        url: &str,
        options: &ScanOptions,
        transport_type: &TransportType,
        session: &MCPSession,
    ) -> Result<Vec<MCPPrompt>> {
        let request = JsonRpcRequest::new("prompts/list", 4).to_json();
        let (response, _) = self
            .send_jsonrpc_request_with_headers_and_session(
                url,
                request,
                options,
                transport_type,
                session.session_id.clone(),
            )
            .await?;
        let prompt_response = PromptResponse::from_json_response(&response)?;
        Ok(prompt_response.prompts)
    }
}

// =====================
// CAPABILITY TRAIT & CHAIN (Composable Middleware)
// =====================

/// Represents the phase in which a scan capability is executed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanPhase {
    /// Pre-scan phase: runs before the main security scan.
    PreScan,
    /// Post-scan phase: runs after the main security scan.
    PostScan,
}

/// Trait for a composable scan capability (middleware hook).
///
/// Implement this trait to add custom security, analysis, or filtering logic
/// to the scan pipeline. Scanners can be registered for the pre-scan phase.
pub trait Scanner: Send + Sync {
    /// Returns the name of the capability.
    fn name(&self) -> &'static str;
    /// Returns the phase in which this capability should run.
    fn phase(&self) -> ScanPhase;
    /// Runs the capability, modifying scan data or reporting issues as needed.
    fn run(&self, _scan_data: &mut ScanData) -> anyhow::Result<()>;
    /// Clones the scanner as a boxed trait object.
    fn box_clone(&self) -> Box<dyn Scanner>;
}

/// Chain of scanners (middleware hooks) for pre-scan processing.
///
/// Scanners are executed in the order they are added.
#[derive(Default)]
pub struct ScannerChain {
    pre_scan: Vec<Box<dyn Scanner>>,
    post_scan: Vec<Box<dyn Scanner>>,
}

impl ScannerChain {
    /// Creates a new, empty scanner chain.
    pub fn new() -> Self {
        Self {
            pre_scan: Vec::new(),
            post_scan: Vec::new(),
        }
    }
    /// Adds a scanner to the chain for its specified phase.
    pub fn add(&mut self, scanner: Box<dyn Scanner>) {
        match scanner.phase() {
            ScanPhase::PreScan => self.pre_scan.push(scanner),
            ScanPhase::PostScan => self.post_scan.push(scanner),
        }
    }
    /// Runs all pre-scan scanners on the provided scan data.
    pub fn run_pre_scan(&self, scan_data: &mut ScanData) {
        for scanner in &self.pre_scan {
            if let Err(e) = scanner.run(scan_data) {
                tracing::warn!("Pre-scan scanner '{}' failed: {}", scanner.name(), e);
            }
        }
    }

    /// Runs all post-scan scanners on the provided scan data.
    pub fn run_post_scan(&self, scan_data: &mut ScanData) {
        for scanner in &self.post_scan {
            if let Err(e) = scanner.run(scan_data) {
                tracing::warn!("Post-scan scanner '{}' failed: {}", scanner.name(), e);
            }
        }
    }
}

// Implement Clone for ScannerChain (requires dyn-clone for trait objects)
impl Clone for ScannerChain {
    fn clone(&self) -> Self {
        Self {
            pre_scan: self.pre_scan.iter().map(|c| c.box_clone()).collect(),
            post_scan: self.post_scan.iter().map(|c| c.box_clone()).collect(),
        }
    }
}

// =====================
// THREAT RULES ENGINE (Replaces hardcoded YaraScanner)
// =====================

#[cfg(feature = "yara-x-scanning")]
use glob::glob;
use std::path::Path;
use std::sync::Arc;

/// Enhanced YARA match with rule metadata
#[cfg(feature = "yara-x-scanning")]
pub struct YaraMatchInfo {
    pub rule_name: String,
    pub rule_metadata: Option<crate::types::YaraRuleMetadata>,
}

/// Enhanced YARA match with rule metadata (non-YARA fallback)
#[cfg(not(feature = "yara-x-scanning"))]
pub struct YaraMatchInfo {
    pub rule_name: String,
    pub rule_metadata: Option<crate::types::YaraRuleMetadata>,
}

/// Threat detection rules engine that loads YARA-X rules from directory structure
pub struct ThreatRules {
    pre_scan_rules: Vec<Arc<YaraRules>>,
    post_scan_rules: Vec<Arc<YaraRules>>,
    rules_dir: String,
    rule_metadata: HashMap<String, RuleMetadata>,
    memory_usage_bytes: usize,
    last_load_time: std::time::Instant,
}

/// Metadata for YARA rules
#[derive(Debug, Clone)]
pub struct RuleMetadata {
    pub name: String,
    pub tags: Vec<String>,
}

impl ThreatRules {
    /// Creates a new threat rules engine with rules from the specified directory
    pub fn new(rules_dir: &str) -> Result<Self> {
        Self::with_config(rules_dir, true)
    }

    /// Creates a new threat rules engine with optional YARA support based on config
    pub fn with_config(rules_dir: &str, enable_yara: bool) -> Result<Self> {
        if !is_yara_available(enable_yara) {
            if enable_yara {
                print_yara_install_message();
            }
            // Return a scanner that will skip YARA operations
            return Self::new_disabled(rules_dir);
        }

        Self::new_enabled(rules_dir)
    }

    /// Creates a disabled threat rules engine (no rule loading)
    fn new_disabled(rules_dir: &str) -> Result<Self> {
        let start_time = std::time::Instant::now();
        Ok(Self {
            pre_scan_rules: Vec::new(),
            post_scan_rules: Vec::new(),
            rules_dir: rules_dir.to_string(),
            rule_metadata: HashMap::new(),
            memory_usage_bytes: 0,
            last_load_time: start_time,
        })
    }

    /// Creates an enabled threat rules engine (loads rules)
    #[cfg(feature = "yara-x-scanning")]
    fn new_enabled(rules_dir: &str) -> Result<Self> {
        let start_time = std::time::Instant::now();
        let mut scanner = Self {
            pre_scan_rules: Vec::new(),
            post_scan_rules: Vec::new(),
            rules_dir: rules_dir.to_string(),
            rule_metadata: HashMap::new(),
            memory_usage_bytes: 0,
            last_load_time: start_time,
        };

        scanner.load_rules()?;
        scanner.last_load_time = start_time;
        Ok(scanner)
    }

    /// Fallback for when YARA feature is not available
    #[cfg(not(feature = "yara-x-scanning"))]
    fn new_enabled(_rules_dir: &str) -> Result<Self> {
        Err(anyhow!("YARA-X scanning feature is not available"))
    }

    /// Loads all rules from the directory structure
    #[cfg(feature = "yara-x-scanning")]
    fn load_rules(&mut self) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Load pre-scan rules
        let pre_dir = format!("{}/pre", self.rules_dir);
        if Path::new(&pre_dir).exists() {
            self.pre_scan_rules = self.load_rules_from_directory(&pre_dir, "pre")?;
        }

        // Load post-scan rules
        let post_dir = format!("{}/post", self.rules_dir);
        if Path::new(&post_dir).exists() {
            self.post_scan_rules = self.load_rules_from_directory(&post_dir, "post")?;
        }

        // Calculate memory usage
        self.calculate_memory_usage();

        let load_duration = start_time.elapsed();
        debug!(
            "Loaded {} pre-scan rules, {} post-scan rules in {}ms (memory: {}KB)",
            self.pre_scan_rules.len(),
            self.post_scan_rules.len(),
            load_duration.as_millis(),
            self.memory_usage_bytes / 1024
        );
        Ok(())
    }

    /// Calculates estimated memory usage of loaded rules
    #[cfg(feature = "yara-x-scanning")]
    fn calculate_memory_usage(&mut self) {
        // Estimate memory usage based on rule count and metadata
        let rule_memory = (self.pre_scan_rules.len() + self.post_scan_rules.len()) * 1024; // ~1KB per rule
        let metadata_memory = self.rule_metadata.len() * 256; // ~256 bytes per metadata entry

        self.memory_usage_bytes = rule_memory + metadata_memory;
    }

    /// Gets memory usage statistics
    #[cfg(test)]
    pub fn memory_stats(&self) -> RuleStats {
        RuleStats {
            pre_scan_count: self.pre_scan_rules.len(),
            post_scan_count: self.post_scan_rules.len(),
            pre_scan_rules: Vec::new(), // Empty for memory stats - not needed for this use case
            post_scan_rules: Vec::new(), // Empty for memory stats - not needed for this use case
            pre_scan_tags: HashMap::new(), // Empty for now - could be populated with actual tag counts
            post_scan_tags: HashMap::new(), // Empty for now - could be populated with actual tag counts
        }
    }

    /// Loads all .yar files from a directory and their metadata
    #[cfg(feature = "yara-x-scanning")]
    fn load_rules_from_directory(
        &mut self,
        dir_path: &str,
        phase: &str,
    ) -> Result<Vec<Arc<YaraRules>>> {
        let mut rules = Vec::new();
        let pattern = format!("{dir_path}/*.yar");

        for entry in glob(&pattern).map_err(|e| anyhow!("Glob error: {}", e))? {
            match entry {
                Ok(path) => {
                    // Safely convert path to string, skipping files with non-UTF8 characters
                    if let Some(path_str) = path.to_str() {
                        // Read the rule file content
                        let rule_content = std::fs::read_to_string(path_str)
                            .map_err(|e| anyhow!("Failed to read rule file {}: {}", path_str, e))?;

                        // Compile the rule using YARA-X compiler
                        let mut compiler = yara_x::Compiler::new();
                        if let Err(e) = compiler.add_source(rule_content.as_str()) {
                            warn!("Failed to add rule source from {}: {}", path_str, e);
                            continue;
                        }

                        let rule = compiler.build();
                        let rule_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        debug!("Loaded YARA-X rule: {} (phase: {})", path.display(), phase);

                        // Create metadata for the rule
                        let metadata = RuleMetadata {
                            name: rule_name.clone(),
                            tags: vec!["yara-x".to_string(), "security".to_string()],
                        };
                        let metadata_key = format!("{phase}:{rule_name}");
                        self.rule_metadata.insert(metadata_key, metadata);

                        rules.push(Arc::new(rule));
                    } else {
                        warn!(
                            "Skipping rule file with non-UTF8 characters: {}",
                            path.display()
                        );
                    }
                }
                Err(e) => warn!("Failed to read rule file: {}", e),
            }
        }

        Ok(rules)
    }

    /// Helper method to extract metadata from YARA-X rule matches
    #[cfg(feature = "yara-x-scanning")]
    fn extract_yara_metadata(
        &self,
        rule: &yara_x::Rule,
        phase: &str,
    ) -> Option<crate::types::YaraRuleMetadata> {
        let mut raw_metadata = HashMap::new();
        for (key, value) in rule.metadata() {
            let value_str = match value {
                yara_x::MetaValue::Integer(i) => i.to_string(),
                yara_x::MetaValue::Bool(b) => b.to_string(),
                yara_x::MetaValue::String(s) => s.to_string(),
                yara_x::MetaValue::Bytes(b) => format!("{b:?}"),
                yara_x::MetaValue::Float(f) => f.to_string(),
            };
            raw_metadata.insert(key.to_string(), value_str);
        }

        if !raw_metadata.is_empty() {
            let tags = if let Some(category) = raw_metadata.get("category") {
                category.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                vec!["yara-x".to_string(), "security".to_string()]
            };

            Some(crate::types::YaraRuleMetadata {
                name: raw_metadata.get("name").cloned(),
                author: raw_metadata.get("author").cloned(),
                date: raw_metadata.get("date").cloned(),
                version: raw_metadata.get("version").cloned(),
                description: raw_metadata.get("description").cloned(),
                severity: raw_metadata.get("severity").cloned(),
                category: raw_metadata.get("category").cloned(),
                confidence: raw_metadata.get("confidence").cloned(),
                tags,
            })
        } else {
            // Fallback to stored metadata if no match metadata available
            let rule_name = rule.identifier();
            let metadata_key = format!("{phase}:{rule_name}");
            self.rule_metadata.get(&metadata_key).map(|stored_meta| {
                crate::types::YaraRuleMetadata {
                    name: Some(stored_meta.name.clone()),
                    author: Some("Ramparts Security Team".to_string()),
                    date: None,
                    version: None,
                    description: Some(stored_meta.name.clone()),
                    severity: Some("MEDIUM".to_string()),
                    category: Some(stored_meta.tags.join(",")),
                    confidence: None,
                    tags: stored_meta.tags.clone(),
                }
            })
        }
    }

    /// Consolidated method to scan text with rules and return enhanced match information
    #[cfg(feature = "yara-x-scanning")]
    fn scan_with_rules_enhanced_internal(
        &self,
        text: &str,
        context: &str,
        rules: &[Arc<YaraRules>],
        phase: &str,
    ) -> Result<Vec<YaraMatchInfo>> {
        let mut all_matches = Vec::new();

        for (i, rule_set) in rules.iter().enumerate() {
            let mut scanner = yara_x::Scanner::new(rule_set);
            match scanner.scan(text.as_bytes()) {
                Ok(scan_results) => {
                    for m in scan_results.matching_rules() {
                        let rule_metadata = self.extract_yara_metadata(&m, phase);

                        all_matches.push(YaraMatchInfo {
                            rule_name: m.identifier().to_string(),
                            rule_metadata,
                        });
                    }
                }
                Err(e) => warn!("Failed to scan with {}-rule {}: {}", phase, i, e),
            }
        }

        if !all_matches.is_empty() {
            debug!(
                "{}-scan matches in {}: {} rules triggered",
                phase,
                context,
                all_matches.len()
            );
        }

        Ok(all_matches)
    }

    /// Scans text with pre-scan rules and returns enhanced match information
    #[cfg(feature = "yara-x-scanning")]
    pub fn pre_scan(&self, text: &str, context: &str) -> Result<Vec<YaraMatchInfo>> {
        self.scan_with_rules_enhanced_internal(text, context, &self.pre_scan_rules, "pre")
    }

    /// Scans text with post-scan rules and returns enhanced match information
    #[cfg(feature = "yara-x-scanning")]
    pub fn post_scan(&self, text: &str, context: &str) -> Result<Vec<YaraMatchInfo>> {
        self.scan_with_rules_enhanced_internal(text, context, &self.post_scan_rules, "post")
    }

    /// Gets statistics about loaded rules
    pub fn stats(&self) -> RuleStats {
        let mut pre_scan_tags = HashMap::new();
        let mut post_scan_tags = HashMap::new();
        let mut pre_scan_rules = Vec::new();
        let mut post_scan_rules = Vec::new();

        // Collect rule names and count tags for pre-scan rules
        for (key, metadata) in &self.rule_metadata {
            if key.starts_with("pre:") {
                pre_scan_rules.push(metadata.name.clone());
                for tag in &metadata.tags {
                    *pre_scan_tags.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        // Collect rule names and count tags for post-scan rules
        for (key, metadata) in &self.rule_metadata {
            if key.starts_with("post:") {
                post_scan_rules.push(metadata.name.clone());
                for tag in &metadata.tags {
                    *post_scan_tags.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        RuleStats {
            pre_scan_count: self.pre_scan_rules.len(),
            post_scan_count: self.post_scan_rules.len(),
            pre_scan_rules,
            post_scan_rules,
            pre_scan_tags,
            post_scan_tags,
        }
    }

    /// Validates that all loaded rules are valid
    #[cfg(all(test, feature = "yara-x-scanning"))]
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        // Check pre-scan rules
        for (i, rules) in self.pre_scan_rules.iter().enumerate() {
            let mut scanner = yara_x::Scanner::new(rules);
            if let Err(e) = scanner.scan(b"test") {
                issues.push(format!("Pre-scan rule {i}: {e}"));
            }
        }

        // Check post-scan rules
        for (i, rules) in self.post_scan_rules.iter().enumerate() {
            let mut scanner = yara_x::Scanner::new(rules);
            if let Err(e) = scanner.scan(b"test") {
                issues.push(format!("Post-scan rule {i}: {e}"));
            }
        }

        if issues.is_empty() {
            Ok(issues)
        } else {
            Err(anyhow!("Rule validation failed: {}", issues.join(", ")))
        }
    }

    /// Fallback validation for when YARA feature is not available  
    #[cfg(all(test, not(feature = "yara-x-scanning")))]
    pub fn validate(&self) -> Result<Vec<String>> {
        Ok(Vec::new()) // Always pass validation when YARA is disabled
    }
}

/// Statistics about loaded YARA rules
#[derive(Debug)]
pub struct RuleStats {
    pub pre_scan_count: usize,
    pub post_scan_count: usize,
    pub pre_scan_rules: Vec<String>,
    pub post_scan_rules: Vec<String>,
    #[allow(dead_code)]
    pub pre_scan_tags: HashMap<String, usize>,
    #[allow(dead_code)]
    pub post_scan_tags: HashMap<String, usize>,
}

impl Clone for ThreatRules {
    fn clone(&self) -> Self {
        // Clone that shares the same rules via Arc - efficient and safe
        Self {
            pre_scan_rules: self.pre_scan_rules.clone(), // Arc<Rules> can be cloned efficiently
            post_scan_rules: self.post_scan_rules.clone(),
            rules_dir: self.rules_dir.clone(),
            rule_metadata: self.rule_metadata.clone(),
            memory_usage_bytes: self.memory_usage_bytes,
            last_load_time: self.last_load_time,
        }
    }
}

/// Threat detection capability that loads rules from directory structure
pub struct YaraScanner {
    scanner: ThreatRules,
    phase: ScanPhase,
}

impl YaraScanner {
    /// Creates a new dynamic YARA capability
    pub fn new(rules_dir: &str, phase: ScanPhase) -> Result<Self> {
        let scanner = ThreatRules::new(rules_dir)?;
        Ok(Self { scanner, phase })
    }

    /// Generic YARA scanning method for any item type that implements BatchScannableItem
    fn scan_items_with_yara<T>(
        &self,
        items: &[T],
        phase: ScanPhase,
    ) -> anyhow::Result<Vec<YaraScanResult>>
    where
        T: crate::security::BatchScannableItem,
    {
        let mut results = Vec::new();

        for item in items {
            let item_text = self.format_item_for_yara_scan(item);
            let context = format!("{} '{}'", T::item_type(), item.name());

            // Use enhanced scanning methods that return metadata
            #[cfg(feature = "yara-x-scanning")]
            let enhanced_matches = match phase {
                ScanPhase::PreScan => self.scanner.pre_scan(&item_text, &context)?,
                ScanPhase::PostScan => self.scanner.post_scan(&item_text, &context)?,
            };

            #[cfg(not(feature = "yara-x-scanning"))]
            let enhanced_matches: Vec<YaraMatchInfo> = Vec::new();

            if !enhanced_matches.is_empty() {
                warn!(
                    "Security issue detected in {} '{}': {} rules matched",
                    T::item_type(),
                    item.name(),
                    enhanced_matches.len()
                );

                // Store YARA results for each match with metadata
                for match_info in enhanced_matches {
                    let yara_result = self.create_yara_result_with_metadata::<T>(
                        item,
                        &match_info.rule_name,
                        &item_text,
                        &context,
                        phase,
                        match_info.rule_metadata,
                    );
                    results.push(yara_result);
                }
            }
        }
        Ok(results)
    }

    /// Format an item for YARA scanning with specialized logic per type
    fn format_item_for_yara_scan<T>(&self, item: &T) -> String
    where
        T: crate::security::BatchScannableItem,
    {
        // For YARA scanning, we want a simple format without numbering
        // to avoid confusion and focus on the actual content
        format!("{}: {}", T::item_type().to_uppercase(), item.name())
    }

    /// Create a YARA scan result with original rule metadata
    fn create_yara_result_with_metadata<T>(
        &self,
        item: &T,
        rule_name: &str,
        _matched_text: &str, // Unused - removing redundant matched_text
        _context: &str,      // Unused - generating context from rule_name
        _phase: ScanPhase,
        _rule_metadata: Option<crate::types::YaraRuleMetadata>, // Unused - removing duplicate metadata
    ) -> YaraScanResult
    where
        T: crate::security::BatchScannableItem,
    {
        YaraScanResult {
            target_type: T::item_type().to_string(),
            target_name: item.name().to_string(),
            rule_name: rule_name.to_string(),
            rule_file: self.map_rule_name_to_file_name(rule_name),
            matched_text: None,
            context: generate_context_message(T::item_type(), rule_name),
            rule_metadata: None,
            phase: None,
            rules_executed: None,
            security_issues_detected: None,
            total_items_scanned: None,
            total_matches: None,
            status: Some("warning".to_string()),
        }
    }

    /// Maps a YARA rule name back to its source file name for consistent reporting
    fn map_rule_name_to_file_name(&self, rule_name: &str) -> Option<String> {
        // Check known rule name to file name mappings
        match rule_name {
            "EnvironmentVariableLeakage" => Some("secrets_leakage".to_string()),
            "CrossOriginEscalation" => Some("cross_origin_escalation".to_string()),
            "CrossDomainContamination" => Some("cross_origin_escalation".to_string()),
            "DomainOutlier" => Some("cross_origin_escalation".to_string()),
            "MixedSecuritySchemes" => Some("cross_origin_escalation".to_string()),
            // Add more mappings as needed for other rules
            _ => None, // Return None if no mapping found, caller will use rule_name as fallback
        }
    }
}

impl Scanner for YaraScanner {
    fn name(&self) -> &'static str {
        "yara"
    }

    fn phase(&self) -> ScanPhase {
        self.phase
    }

    fn run(&self, scan_data: &mut ScanData) -> anyhow::Result<()> {
        match self.phase {
            ScanPhase::PreScan => {
                let stats = self.scanner.stats();
                debug!("Running pre-scan with {} rules", stats.pre_scan_count);

                // Scan all item types using the generic scanner
                let tool_results =
                    self.scan_items_with_yara(&scan_data.tools, ScanPhase::PreScan)?;
                let prompt_results =
                    self.scan_items_with_yara(&scan_data.prompts, ScanPhase::PreScan)?;
                let resource_results =
                    self.scan_items_with_yara(&scan_data.resources, ScanPhase::PreScan)?;

                // Count total matches found
                let total_matches =
                    tool_results.len() + prompt_results.len() + resource_results.len();
                let total_items =
                    scan_data.tools.len() + scan_data.prompts.len() + scan_data.resources.len();

                // Collect triggered rule names from all results and map them to file names for consistency
                let mut triggered_file_names = std::collections::HashSet::new();
                let mut triggered_rules = std::collections::HashSet::new();

                for result in &tool_results {
                    triggered_rules.insert(result.rule_name.clone());
                    // Map YARA rule name back to file name for consistent comparison
                    if let Some(file_name) = self.map_rule_name_to_file_name(&result.rule_name) {
                        triggered_file_names.insert(file_name);
                    } else {
                        // Fallback: use the rule name itself if no mapping found
                        triggered_file_names.insert(result.rule_name.clone());
                    }
                }
                for result in &prompt_results {
                    triggered_rules.insert(result.rule_name.clone());
                    if let Some(file_name) = self.map_rule_name_to_file_name(&result.rule_name) {
                        triggered_file_names.insert(file_name);
                    } else {
                        triggered_file_names.insert(result.rule_name.clone());
                    }
                }
                for result in &resource_results {
                    triggered_rules.insert(result.rule_name.clone());
                    if let Some(file_name) = self.map_rule_name_to_file_name(&result.rule_name) {
                        triggered_file_names.insert(file_name);
                    } else {
                        triggered_file_names.insert(result.rule_name.clone());
                    }
                }

                // Add all results to scan data
                scan_data.yara_results.extend(tool_results);
                scan_data.yara_results.extend(prompt_results);
                scan_data.yara_results.extend(resource_results);

                // Add a summary result
                let summary_result = YaraScanResult {
                    target_type: "summary".to_string(),
                    target_name: "pre-scan".to_string(),
                    rule_name: "YARA_PRE_SCAN_SUMMARY".to_string(),
                    rule_file: None,
                    matched_text: None,
                    context: format!(
                        "Pre-scan completed: {} rules executed on {} items",
                        stats.pre_scan_count, total_items
                    ),
                    rule_metadata: None,
                    phase: Some("pre-scan".to_string()),
                    rules_executed: if !stats.pre_scan_rules.is_empty() {
                        Some(
                            stats
                                .pre_scan_rules
                                .iter()
                                .map(|f| format!("{f}:*"))
                                .collect(),
                        )
                    } else {
                        None
                    },
                    security_issues_detected: if total_matches > 0 {
                        // Show actual rules that triggered matches with filename:rulename format
                        let triggered_vec: Vec<String> = triggered_rules
                            .into_iter()
                            .map(|rule_name| {
                                // Get the file name for this rule
                                if let Some(file_name) = rule_name_to_file_name(&rule_name) {
                                    format!("{file_name}:{rule_name}")
                                } else {
                                    rule_name
                                }
                            })
                            .collect();
                        debug!("Pre-scan triggered rules: {:?}", triggered_vec);
                        Some(triggered_vec)
                    } else {
                        None
                    },
                    total_items_scanned: Some(total_items),
                    total_matches: Some(total_matches),
                    status: Some(if total_matches == 0 {
                        "passed".to_string()
                    } else {
                        "warning".to_string()
                    }),
                };
                scan_data.yara_results.push(summary_result);
            }
            ScanPhase::PostScan => {
                let stats = self.scanner.stats();
                debug!("Running post-scan with {} rules", stats.post_scan_count);

                // Scan all item types using the generic scanner
                let tool_results =
                    self.scan_items_with_yara(&scan_data.tools, ScanPhase::PostScan)?;
                let prompt_results =
                    self.scan_items_with_yara(&scan_data.prompts, ScanPhase::PostScan)?;
                let resource_results =
                    self.scan_items_with_yara(&scan_data.resources, ScanPhase::PostScan)?;

                // Count total matches found
                let total_matches =
                    tool_results.len() + prompt_results.len() + resource_results.len();
                let total_items =
                    scan_data.tools.len() + scan_data.prompts.len() + scan_data.resources.len();

                // Collect triggered rule names from all results and map them to file names for consistency
                let mut triggered_file_names = std::collections::HashSet::new();
                let mut triggered_rules = std::collections::HashSet::new();

                for result in &tool_results {
                    triggered_rules.insert(result.rule_name.clone());
                    if let Some(file_name) = self.map_rule_name_to_file_name(&result.rule_name) {
                        triggered_file_names.insert(file_name);
                    } else {
                        triggered_file_names.insert(result.rule_name.clone());
                    }
                }
                for result in &prompt_results {
                    triggered_rules.insert(result.rule_name.clone());
                    if let Some(file_name) = self.map_rule_name_to_file_name(&result.rule_name) {
                        triggered_file_names.insert(file_name);
                    } else {
                        triggered_file_names.insert(result.rule_name.clone());
                    }
                }
                for result in &resource_results {
                    triggered_rules.insert(result.rule_name.clone());
                    if let Some(file_name) = self.map_rule_name_to_file_name(&result.rule_name) {
                        triggered_file_names.insert(file_name);
                    } else {
                        triggered_file_names.insert(result.rule_name.clone());
                    }
                }

                // Add all results to scan data
                scan_data.yara_results.extend(tool_results);
                scan_data.yara_results.extend(prompt_results);
                scan_data.yara_results.extend(resource_results);

                // Add a summary result
                let summary_result = YaraScanResult {
                    target_type: "summary".to_string(),
                    target_name: "post-scan".to_string(),
                    rule_name: "YARA_POST_SCAN_SUMMARY".to_string(),
                    rule_file: None,
                    matched_text: None,
                    context: format!(
                        "Post-scan completed: {} rules executed on {} items",
                        stats.post_scan_count, total_items
                    ),
                    rule_metadata: None,
                    phase: Some("post-scan".to_string()),
                    rules_executed: if !stats.post_scan_rules.is_empty() {
                        Some(
                            stats
                                .post_scan_rules
                                .iter()
                                .map(|f| format!("{f}:*"))
                                .collect(),
                        )
                    } else {
                        None
                    },
                    security_issues_detected: if total_matches > 0 {
                        // Show actual rules that triggered matches with filename:rulename format
                        let triggered_vec: Vec<String> = triggered_rules
                            .into_iter()
                            .map(|rule_name| {
                                // Get the file name for this rule
                                if let Some(file_name) = rule_name_to_file_name(&rule_name) {
                                    format!("{file_name}:{rule_name}")
                                } else {
                                    rule_name
                                }
                            })
                            .collect();
                        debug!("Post-scan triggered rules: {:?}", triggered_vec);
                        Some(triggered_vec)
                    } else {
                        None
                    },
                    total_items_scanned: Some(total_items),
                    total_matches: Some(total_matches),
                    status: Some(if total_matches == 0 {
                        "passed".to_string()
                    } else {
                        "warning".to_string()
                    }),
                };
                scan_data.yara_results.push(summary_result);
            }
        }

        Ok(())
    }

    fn box_clone(&self) -> Box<dyn Scanner> {
        Box::new(Self {
            scanner: self.scanner.clone(),
            phase: self.phase,
        })
    }
}

// MCPScanner struct
pub struct MCPScanner {
    client: Client,
    http_timeout: u64,
    middleware_chain: ScannerChain, // New: pre/post scan hooks
}

// MCPScanner implementation
impl MCPScanner {
    /// Creates a new MCPScanner with the specified HTTP timeout.
    pub fn with_timeout(http_timeout: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(http_timeout))
            .user_agent(protocol::USER_AGENT)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        // Set up middleware chain with dynamic YARA capabilities
        let mut middleware_chain = ScannerChain::new();

        // Add dynamic YARA pre-scan capability
        if let Ok(pre_cap) = YaraScanner::new("rules", ScanPhase::PreScan) {
            middleware_chain.add(Box::new(pre_cap));
            info!("{}", messages::YARA_PRE_SCAN_LOADED);
        } else {
            warn!("{}", messages::YARA_PRE_SCAN_FAILED);
        }

        // Add dynamic YARA post-scan capability
        if let Ok(post_cap) = YaraScanner::new("rules", ScanPhase::PostScan) {
            middleware_chain.add(Box::new(post_cap));
            info!("{}", messages::YARA_POST_SCAN_LOADED);
        } else {
            warn!("{}", messages::YARA_POST_SCAN_FAILED);
        }

        // Add cross-origin escalation scanner (pre-scan)
        let cross_origin_scanner = CrossOriginScanner::new(ScanPhase::PreScan);
        middleware_chain.add(Box::new(cross_origin_scanner));
        debug!("Cross-origin scanner loaded");

        Ok(Self {
            client,
            http_timeout,
            middleware_chain,
        })
    }

    /// Scan a single MCP server
    pub async fn scan_single(&self, url: &str, options: ScanOptions) -> Result<ScanResult> {
        let mut result = ScanResult::new(url.to_string());

        info!("Scanning {}", url);

        // Detect transport type
        let transport_type = TransportType::from_url(url);
        debug!("Transport type: {:?}", transport_type);

        // Normalize URL with error context
        let normalized_url = error_utils::wrap_error(self.normalize_url(url), "URL normalization")?;
        result.url = normalized_url.clone();

        // Perform the scan with performance tracking
        let scan_result = track_performance("MCP server scan", || async {
            let scan_future = self.perform_scan(&normalized_url, &options, &transport_type);
            match timeout(Duration::from_secs(options.timeout), scan_future).await {
                Ok(result) => result,
                Err(_) => Err(anyhow!("Scan operation timed out")),
            }
        })
        .await;

        match scan_result {
            Ok(mut scan_data) => {
                // === PRE-SCAN HOOKS ===
                self.middleware_chain.run_pre_scan(&mut scan_data);

                result.status = ScanStatus::Success;
                result.server_info = scan_data.server_info.clone();
                result.tools = scan_data.tools.clone();
                result.resources = scan_data.resources.clone();
                result.prompts = scan_data.prompts.clone();
                result.yara_results = scan_data.yara_results.clone();

                // Add fetch errors to the result
                result.errors.extend(scan_data.fetch_errors.clone());

                // Load scanner configuration
                let config_manager = crate::config::ScannerConfigManager::new();
                let scanner_config = match config_manager.load_config() {
                    Ok(config) => config,
                    Err(e) => {
                        warn!("Failed to load scanner config, using defaults: {}", e);
                        result.errors.push(format!("Config loading failed: {e}"));
                        Default::default()
                    }
                };

                // Perform security scanning with configuration
                let security_scanner = if scanner_config.security.enabled {
                    SecurityScanner::with_config(scanner_config)
                } else {
                    SecurityScanner::default()
                };
                let mut security_result = SecurityScanResult::new();

                // Always perform the security scan (no enhanced/standard distinction)
                // Batch scan tools for security issues
                match security_scanner
                    .scan_tools_batch(&scan_data.tools, options.detailed)
                    .await
                {
                    Ok((tool_issues, analysis_details)) => {
                        security_result.add_tool_issues(tool_issues);
                        // Store the analysis details for each tool
                        for (tool_name, details) in analysis_details {
                            security_result.add_tool_analysis_details(tool_name, details);
                        }
                    }
                    Err(e) => warn!("Failed to batch scan tools for security issues: {}", e),
                }

                // Batch scan prompts for security issues
                if !scan_data.prompts.is_empty() {
                    match security_scanner
                        .scan_prompts_batch(&scan_data.prompts, options.detailed)
                        .await
                    {
                        Ok(prompt_issues) => security_result.add_prompt_issues(prompt_issues),
                        Err(e) => warn!("Failed to batch scan prompts for security issues: {}", e),
                    }
                }

                // Batch scan resources for security issues
                if !scan_data.resources.is_empty() {
                    match security_scanner
                        .scan_resources_batch(&scan_data.resources, options.detailed)
                        .await
                    {
                        Ok(resource_issues) => security_result.add_resource_issues(resource_issues),
                        Err(e) => {
                            warn!("Failed to batch scan resources for security issues: {}", e)
                        }
                    }
                }

                result.security_issues = Some(security_result);

                // === POST-SCAN HOOKS ===
                self.middleware_chain.run_post_scan(&mut scan_data);

                // Update result with any post-scan changes
                result.yara_results = scan_data.yara_results.clone();

                result.response_time_ms = Timer::start().elapsed_ms(); // Track actual scan time
                debug!("Scan completed in {}ms", result.response_time_ms);
            }
            Err(e) => {
                result.status = ScanStatus::Failed(e.to_string());
                result.add_error(error_utils::create_error_msg(
                    "Scan operation",
                    &e.to_string(),
                ));
                warn!("Scan failed: [\x1b[1m{}\x1b[0m]", e);
            }
        }

        Ok(result)
    }

    /// Scan MCP servers from IDE configuration files
    pub async fn scan_config(&self, options: ScanOptions) -> Result<Vec<ScanResult>> {
        let config_manager = MCPConfigManager::new();

        if !config_manager.has_config_files() {
            return Err(anyhow!("No MCP IDE configuration files found"));
        }

        let config = config_manager.load_config()?;
        let mut results = Vec::new();

        if let Some(servers) = config.servers {
            info!(
                "Found [\x1b[1m{}\x1b[0m] MCP servers in IDE configuration files",
                servers.len()
            );

            for server in servers {
                info!(
                    "Scanning MCP server from IDE config: [\x1b[1m{}\x1b[0m] ({})",
                    server.name.as_deref().unwrap_or("unnamed"),
                    server.url
                );

                // Merge server-specific options with global options
                let mut server_options = options.clone();

                // Apply global options from config
                if let Some(global_options) = &config.options {
                    if let Some(timeout) = global_options.timeout {
                        server_options.timeout = timeout;
                    }
                    if let Some(http_timeout) = global_options.http_timeout {
                        server_options.http_timeout = http_timeout;
                    }
                    if let Some(format) = &global_options.format {
                        server_options.format = format.clone();
                    }
                    if let Some(detailed) = global_options.detailed {
                        server_options.detailed = detailed;
                    }
                }

                // Apply server-specific options
                if let Some(server_specific_options) = &server.options {
                    if let Some(timeout) = server_specific_options.timeout {
                        server_options.timeout = timeout;
                    }
                    if let Some(http_timeout) = server_specific_options.http_timeout {
                        server_options.http_timeout = http_timeout;
                    }
                    if let Some(format) = &server_specific_options.format {
                        server_options.format = format.clone();
                    }
                    if let Some(detailed) = server_specific_options.detailed {
                        server_options.detailed = detailed;
                    }
                }

                // Merge authentication headers
                let mut auth_headers = options.auth_headers.clone();

                // Add global auth headers
                if let Some(global_auth_headers) = &config.auth_headers {
                    match &mut auth_headers {
                        Some(headers) => {
                            for (key, value) in global_auth_headers {
                                headers.insert(key.clone(), value.clone());
                            }
                        }
                        None => {
                            auth_headers = Some(global_auth_headers.clone());
                        }
                    }
                }

                // Add server-specific auth headers
                if let Some(server_auth_headers) = &server.auth_headers {
                    match &mut auth_headers {
                        Some(headers) => {
                            for (key, value) in server_auth_headers {
                                headers.insert(key.clone(), value.clone());
                            }
                        }
                        None => {
                            auth_headers = Some(server_auth_headers.clone());
                        }
                    }
                }

                server_options.auth_headers = auth_headers;

                // Scan the MCP server
                match self.scan_single(&server.url, server_options).await {
                    Ok(result) => {
                        results.push(result);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to scan MCP server [\x1b[1m{}\x1b[0m]: {}",
                            server.url, e
                        );
                        let mut failed_result = ScanResult::new(server.url.clone());
                        failed_result.status = ScanStatus::Failed(e.to_string());
                        failed_result.add_error(format!("IDE config scan failed: {e}"));
                        results.push(failed_result);
                    }
                }
            }
        } else {
            warn!("No MCP servers found in IDE configuration files");
        }

        Ok(results)
    }

    /// Perform the scan
    async fn perform_scan(
        &self,
        url: &str,
        options: &ScanOptions,
        transport_type: &TransportType,
    ) -> Result<ScanData> {
        let mut scan_data = ScanData::new();

        // Initialize MCP client session
        let session = self
            .initialize_mcp_session(url, options, transport_type)
            .await?;

        // Get server info from initialization
        if let Some(ref server_info) = session.server_info {
            scan_data.server_info = Some(server_info.clone());
        }

        // Fetch tools, resources, and prompts with proper error handling
        let mut fetch_errors = Vec::new();

        scan_data.tools = match self
            .fetch_tools(url, options, transport_type, &session)
            .await
        {
            Ok(tools) => tools,
            Err(e) => {
                let error_msg = format!("Failed to fetch tools: {e}");
                warn!("{}", error_msg);
                fetch_errors.push(error_msg);
                Vec::new()
            }
        };

        scan_data.resources = match self
            .fetch_resources(url, options, transport_type, &session)
            .await
        {
            Ok(resources) => resources,
            Err(e) => {
                let error_msg = format!("Failed to fetch resources: {e}");
                warn!("{}", error_msg);
                fetch_errors.push(error_msg);
                Vec::new()
            }
        };

        scan_data.prompts = match self
            .fetch_prompts(url, options, transport_type, &session)
            .await
        {
            Ok(prompts) => prompts,
            Err(e) => {
                let error_msg = format!("Failed to fetch prompts: {e}");
                warn!("{}", error_msg);
                fetch_errors.push(error_msg);
                Vec::new()
            }
        };

        // Store fetch errors in scan_data for later inclusion in final result
        // (We'll need to add this field to ScanData)
        scan_data.fetch_errors = fetch_errors;

        Ok(scan_data)
    }

    /// Initialize an MCP session
    async fn initialize_mcp_session(
        &self,
        url: &str,
        options: &ScanOptions,
        transport_type: &TransportType,
    ) -> Result<MCPSession> {
        // Try different protocol versions for compatibility, prioritizing latest
        let protocol_versions = protocol::MCP_PROTOCOL_VERSIONS;

        for protocol_version in protocol_versions {
            let request = json!({
                "jsonrpc": protocol::JSONRPC_VERSION,
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": protocol_version,
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "clientInfo": {
                        "name": protocol::CLIENT_NAME,
                        "version": protocol::CLIENT_VERSION
                    }
                }
            });

            match self
                .send_jsonrpc_request_with_headers_and_session(
                    url,
                    request,
                    options,
                    transport_type,
                    None,
                )
                .await
            {
                Ok((response, session_id)) => {
                    // Use utility function for debug logging
                    debug!(
                        "Initialize response: [\x1b[1m{}\x1b[0m]",
                        serde_json::to_string_pretty(&response).unwrap_or_default()
                    );

                    // Parse server info from response
                    let server_info = MCPServerInfo {
                        name: response["result"]["serverInfo"]["name"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                        version: response["result"]["serverInfo"]["version"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                        description: response["result"]["serverInfo"]["description"]
                            .as_str()
                            .map(|s| s.to_string()),
                        capabilities: self.extract_capabilities(&response),
                        metadata: HashMap::new(),
                    };

                    return Ok(MCPSession {
                        server_info: Some(server_info),
                        session_id,
                    });
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize with protocol version [\x1b[1m{}\x1b[0m]: {}",
                        protocol_version, e
                    );
                    continue;
                }
            }
        }

        // Try a simpler initialize request without capabilities
        let simple_request = json!({
            "jsonrpc": protocol::JSONRPC_VERSION,
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": protocol::MCP_PROTOCOL_VERSIONS[0],
                "clientInfo": {
                    "name": protocol::CLIENT_NAME,
                    "version": protocol::CLIENT_VERSION
                }
            }
        });

        match self
            .send_jsonrpc_request_with_headers_and_session(
                url,
                simple_request,
                options,
                transport_type,
                None,
            )
            .await
        {
            Ok((response, session_id)) => {
                debug!(
                    "Simple initialize response: [\x1b[1m{}\x1b[0m]",
                    serde_json::to_string_pretty(&response).unwrap_or_default()
                );

                let server_info = MCPServerInfo {
                    name: response["result"]["serverInfo"]["name"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    version: response["result"]["serverInfo"]["version"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    description: response["result"]["serverInfo"]["description"]
                        .as_str()
                        .map(|s| s.to_string()),
                    capabilities: self.extract_capabilities(&response),
                    metadata: HashMap::new(),
                };

                return Ok(MCPSession {
                    server_info: Some(server_info),
                    session_id,
                });
            }
            Err(e) => {
                warn!(
                    "Failed to initialize with simple request: [\x1b[1m{}\x1b[0m]",
                    e
                );
            }
        }

        Err(anyhow!(
            "Failed to initialize MCP session with any protocol version"
        ))
    }

    /// Send a JSON-RPC request with session ID support
    async fn send_jsonrpc_request_with_headers_and_session(
        &self,
        url: &str,
        request: Value,
        options: &ScanOptions,
        transport_type: &TransportType,
        session_id: Option<String>,
    ) -> Result<(Value, Option<String>)> {
        match transport_type {
            TransportType::Http => {
                retry_with_backoff(
                    || async {
                        // Debug: Log the exact request being sent
                        tracing::debug!(
                            "Sending JSON-RPC request to [\x1b[1m{}\x1b[0m]: {}",
                            url,
                            serde_json::to_string_pretty(&request).unwrap_or_default()
                        );

                        let mut req = self
                            .client
                            .post(url)
                            .header("Content-Type", "application/json")
                            .header("Accept", "application/json, text/event-stream")
                            .header("User-Agent", protocol::USER_AGENT)
                            .header("MCP-Protocol-Version", protocol::MCP_PROTOCOL_VERSIONS[0])
                            .json(&request);

                        // Add session ID header if provided
                        if let Some(ref session_id) = session_id {
                            req = req.header("Mcp-Session-Id", session_id);
                            tracing::debug!(
                                "Adding session header: Mcp-Session-Id: [\x1b[1m{}\x1b[0m]",
                                session_id
                            );
                        }

                        // Add authentication headers if provided
                        if let Some(ref auth_headers) = options.auth_headers {
                            for (key, value) in auth_headers {
                                req = req.header(key, value);
                                tracing::debug!(
                                    "Adding auth header: [\x1b[1m{}\x1b[0m]: {}",
                                    key,
                                    sanitize_header_for_logging(key, value)
                                );
                            }
                        }

                        let response = req.send().await?;

                        // Debug: Log response status and headers
                        let status = response.status();
                        tracing::debug!("Response status: [\x1b[1m{}\x1b[0m]", status);
                        tracing::debug!("Response headers: {:?}", response.headers());

                        // Extract session ID from response headers
                        let response_session_id = response
                            .headers()
                            .get("mcp-session-id")
                            .and_then(|h| h.to_str().ok())
                            .map(|s| s.to_string());

                        // Check for SSE/streaming response
                        if let Some(content_type) = response.headers().get("content-type") {
                            let content_type_str = content_type.to_str().unwrap_or("");
                            if content_type_str.contains("text/event-stream") {
                                let response_json = self.handle_sse_response(response).await?;
                                return Ok((response_json, response_session_id));
                            }
                        }

                        let response_body = response.text().await.unwrap_or_default();

                        if !status.is_success() {
                            // Debug: Log the response body for error cases
                            tracing::debug!(
                                "Error response body: [\x1b[1m{}\x1b[0m]",
                                response_body
                            );
                            return Err(anyhow!(
                                "HTTP request returned status: {} - Body: {}",
                                status,
                                response_body
                            ));
                        }

                        let response_json: Value = serde_json::from_str(&response_body)
                            .map_err(|e| anyhow!("Failed to parse JSON response: {}", e))?;

                        // Debug: Log the response
                        tracing::debug!(
                            "JSON-RPC response: {}",
                            serde_json::to_string_pretty(&response_json).unwrap_or_default()
                        );

                        if let Some(error) = response_json.get("error") {
                            return Err(anyhow!("JSON-RPC error: {}", error));
                        }

                        Ok((response_json, response_session_id))
                    },
                    3,   // max_retries
                    500, // initial_delay_ms
                )
                .await
            }
            TransportType::Stdio => {
                // Handle STDIO transport
                let mut stdio_transport = STDIOTransport::from_stdio_url(url)?;
                stdio_transport.start().await?;

                // Send request via STDIO
                let response = stdio_transport.send_request(request).await?;

                // STDIO doesn't support session IDs in the same way as HTTP
                // but we can extract from response if available
                let response_session_id = response
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Clean up STDIO transport
                let _ = stdio_transport.shutdown().await;

                if let Some(error) = response.get("error") {
                    return Err(anyhow!("JSON-RPC error from STDIO: {}", error));
                }

                Ok((response, response_session_id))
            }
        }
    }

    /// Handle an SSE response
    async fn handle_sse_response(&self, response: reqwest::Response) -> Result<Value> {
        use futures_util::StreamExt;

        info!("Detected SSE response, parsing stream...");

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut json_response = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| anyhow!("Failed to read SSE chunk: {}", e))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Parse SSE format: "data: {json}\n\n"
            for line in buffer.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if !json_str.trim().is_empty() {
                        match serde_json::from_str::<Value>(json_str) {
                            Ok(json) => {
                                json_response = Some(json);
                                break;
                            }
                            Err(e) => {
                                warn!("Failed to parse SSE JSON: {}", e);
                            }
                        }
                    }
                }
            }

            // If we found a valid JSON response, break
            if json_response.is_some() {
                break;
            }
        }

        match json_response {
            Some(json) => {
                if let Some(error) = json.get("error") {
                    return Err(anyhow!("JSON-RPC error from SSE: {}", error));
                }
                Ok(json)
            }
            None => Err(anyhow!("No valid JSON-RPC response found in SSE stream")),
        }
    }

    /// Extract capabilities from the response
    fn extract_capabilities(&self, response: &Value) -> Vec<String> {
        let mut capabilities = Vec::new();

        if let Some(caps) = response["result"]["capabilities"].as_object() {
            for (capability, _) in caps {
                capabilities.push(capability.clone());
            }
        }

        capabilities
    }

    /// Normalize a URL
    fn normalize_url(&self, url: &str) -> Result<String> {
        let transport_type = TransportType::from_url(url);

        match transport_type {
            TransportType::Http => {
                let mut url = url.to_string();

                // Add http:// if no scheme is provided
                if !url.contains("://") {
                    url = format!("http://{url}");
                }

                // Validate URL
                Url::parse(&url).map_err(|e| anyhow!("Invalid URL: {}", e))?;

                Ok(url)
            }
            TransportType::Stdio => {
                // For STDIO, just return the command as-is
                // Remove stdio:// prefix if present
                let normalized = if url.starts_with("stdio://") {
                    url.strip_prefix("stdio://").unwrap_or(url).to_string()
                } else {
                    url.to_string()
                };

                // Validate that the command exists or is executable
                let parts: Vec<&str> = normalized.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(anyhow!("Empty STDIO command"));
                }

                let command = parts[0];
                if !Path::new(command).exists() && !self.is_executable(command) {
                    warn!("STDIO command may not exist or be executable: {}", command);
                }

                Ok(normalized)
            }
        }
    }

    /// Check if a command is executable (basic check)
    fn is_executable(&self, command: &str) -> bool {
        // Check if it's in PATH
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                let executable_path = Path::new(dir).join(command);
                if executable_path.exists() {
                    return true;
                }
            }
        }

        // Check if it's an absolute path and exists
        if Path::new(command).exists() {
            return true;
        }

        false
    }
}

// Clone the MCPScanner
impl Clone for MCPScanner {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            http_timeout: self.http_timeout,
            middleware_chain: self.middleware_chain.clone(),
        }
    }
}

// Scan data
pub(crate) struct ScanData {
    pub server_info: Option<MCPServerInfo>,
    pub tools: Vec<MCPTool>,
    pub resources: Vec<MCPResource>,
    pub prompts: Vec<MCPPrompt>,
    pub yara_results: Vec<YaraScanResult>,
    pub fetch_errors: Vec<String>,
}

// Scan data implementation
impl ScanData {
    fn new() -> Self {
        Self {
            server_info: None,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            yara_results: Vec::new(),
            fetch_errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creates_threat_rules_with_and_without_yara_x() {
        // Use disabled YARA when feature is not available
        #[cfg(feature = "yara-x-scanning")]
        let scanner = ThreatRules::new("rules");
        #[cfg(not(feature = "yara-x-scanning"))]
        let scanner = ThreatRules::with_config("rules", false);

        assert!(
            scanner.is_ok(),
            "ThreatRules creation should succeed even when YARA is disabled"
        );

        let scanner = scanner.expect("Scanner creation should have succeeded");
        let stats = scanner.stats();

        // Should have loaded rules from the pre directory (only when YARA is enabled)
        #[cfg(feature = "yara-x-scanning")]
        assert!(stats.pre_scan_count > 0);
        #[cfg(not(feature = "yara-x-scanning"))]
        assert_eq!(stats.pre_scan_count, 0);

        println!("Loaded {} pre-scan rules", stats.pre_scan_count);
    }

    #[test]
    fn test_creates_yara_capability_with_correct_phase() {
        let scanner = YaraScanner::new("rules", ScanPhase::PreScan);
        assert!(
            scanner.is_ok(),
            "YaraScanner creation should succeed with valid rules directory"
        );

        let scanner_instance = scanner.expect("Scanner creation should have succeeded");
        assert_eq!(scanner_instance.name(), "yara");
        assert_eq!(scanner_instance.phase(), ScanPhase::PreScan);
    }

    #[test]
    fn test_reports_memory_usage_statistics() {
        #[cfg(feature = "yara-x-scanning")]
        let scanner = ThreatRules::new("rules")
            .expect("Should be able to create ThreatRules with rules directory");
        #[cfg(not(feature = "yara-x-scanning"))]
        let scanner = ThreatRules::with_config("rules", false)
            .expect("Should be able to create ThreatRules with YARA disabled");

        let memory_stats = scanner.memory_stats();

        #[cfg(feature = "yara-x-scanning")]
        assert!(memory_stats.pre_scan_count + memory_stats.post_scan_count > 0);
        #[cfg(not(feature = "yara-x-scanning"))]
        assert_eq!(
            memory_stats.pre_scan_count + memory_stats.post_scan_count,
            0
        );

        println!("Memory stats: {memory_stats:?}");
    }

    #[test]
    fn test_shares_rules_efficiently_via_arc_cloning() {
        // Create first scanner
        let scanner1 = ThreatRules::new("rules").unwrap();
        let memory1 = scanner1.memory_stats();

        // Clone should work without deadlocks
        let scanner2 = scanner1.clone();
        let memory2 = scanner2.memory_stats();

        // Both scanners should have same rule counts
        assert_eq!(memory1.pre_scan_count, memory2.pre_scan_count);
        assert_eq!(memory1.post_scan_count, memory2.post_scan_count);

        // Memory counts should match between cloned scanners
        assert_eq!(
            memory1.pre_scan_count + memory1.post_scan_count,
            memory2.pre_scan_count + memory2.post_scan_count
        );

        println!("Cache test passed - cloned scanner has identical memory usage");
    }

    #[test]
    fn test_validates_loaded_rules_successfully() {
        let scanner = ThreatRules::new("rules")
            .expect("Should be able to create ThreatRules with rules directory");
        let validation_result = scanner.validate();

        // Should not have validation errors
        assert!(
            validation_result.is_ok(),
            "YARA rule validation should pass for well-formed rules"
        );
        println!("Rule validation passed");
    }

    #[test]
    #[cfg(feature = "yara-x-scanning")]
    fn test_compiles_and_scans_with_yara_x_rules() {
        // Test actual YARA-X rule compilation from .yar files
        let test_rule = r#"
            rule TestRule {
                meta:
                    name = "Test Rule"
                    description = "A test rule for YARA-X integration"
                    severity = "MEDIUM"
                strings:
                    $test_string = "MALICIOUS_PATTERN"
                    $api_key = /[Aa][Pp][Ii].*[Kk][Ee][Yy]/
                condition:
                    $test_string or $api_key
            }
        "#;

        // Test compilation
        let mut compiler = yara_x::Compiler::new();
        assert!(
            compiler.add_source(test_rule).is_ok(),
            "Rule compilation should succeed"
        );

        let rules = compiler.build();

        // Test scanning with matches
        let mut scanner = yara_x::Scanner::new(&rules);
        let result = scanner.scan(b"This contains MALICIOUS_PATTERN text");
        assert!(result.is_ok(), "Scanning should succeed");

        let scan_results = result.expect("YARA-X scan should have succeeded");
        let matching_rules: Vec<_> = scan_results.matching_rules().collect();
        assert!(
            !matching_rules.is_empty(),
            "Should have matches for test pattern"
        );

        // Test scanning without matches
        let mut scanner2 = yara_x::Scanner::new(&rules);
        let result2 = scanner2.scan(b"Clean text with no malicious content");
        assert!(result2.is_ok(), "Scanning clean text should succeed");

        let scan_results2 = result2.expect("YARA-X scan on clean text should have succeeded");
        let matching_rules2: Vec<_> = scan_results2.matching_rules().collect();
        assert!(
            matching_rules2.is_empty(),
            "Should have no matches for clean text"
        );
    }

    #[test]
    #[cfg(feature = "yara-x-scanning")]
    fn test_extracts_metadata_from_yara_x_matches() {
        // Test metadata extraction from YARA-X rules
        let test_rule = r#"
            rule MetadataTest {
                meta:
                    name = "Metadata Test Rule"
                    author = "Test Author"
                    version = "2.0"
                    severity = "HIGH"
                    confidence = 0.95
                    tags = "test,metadata"
                strings:
                    $test = "test_pattern"
                condition:
                    $test
            }
        "#;

        let mut compiler = yara_x::Compiler::new();
        compiler
            .add_source(test_rule)
            .expect("Test rule should compile successfully with YARA-X");

        let rules = compiler.build();
        let mut scanner = yara_x::Scanner::new(&rules);
        let result = scanner.scan(b"test_pattern");
        assert!(
            result.is_ok(),
            "YARA-X scanning should succeed on test pattern"
        );

        let scan_results = result.expect("YARA-X scan should have succeeded");
        let matching_rules: Vec<_> = scan_results.matching_rules().collect();
        assert!(!matching_rules.is_empty());

        // Test metadata extraction
        let rule = &matching_rules[0];
        assert_eq!(rule.identifier(), "MetadataTest");

        let metadata: std::collections::HashMap<_, _> = rule.metadata().collect();
        assert!(metadata.contains_key("name"));
        assert!(metadata.contains_key("severity"));
        assert!(metadata.contains_key("confidence"));
    }

    #[test]
    #[cfg(feature = "yara-x-scanning")]
    fn test_rejects_malformed_yara_rules() {
        // Test error handling for malformed rules
        let malformed_rule = r#"
            rule MalformedRule {
                invalid_section:
                    this_is_not_valid_yara_syntax = "error"
                condition:
                    undefined_variable
            }
        "#;

        let mut compiler = yara_x::Compiler::new();
        let result = compiler.add_source(malformed_rule);
        assert!(
            result.is_err(),
            "Malformed rule should cause compilation error"
        );
    }

    #[test]
    fn test_loads_rules_from_filesystem() {
        // Test that the real YARA rules can be loaded
        let scanner = ThreatRules::new("rules")
            .expect("Should be able to create ThreatRules with rules directory");
        let stats = scanner.memory_stats();

        // Should have loaded the .yar rule files (only when YARA-X is enabled)
        #[cfg(feature = "yara-x-scanning")]
        {
            assert!(stats.pre_scan_count + stats.post_scan_count > 0);
            assert!(stats.pre_scan_count > 0);
        }

        #[cfg(not(feature = "yara-x-scanning"))]
        {
            // When YARA is disabled, rule counts should be 0
            assert_eq!(stats.pre_scan_count + stats.post_scan_count, 0);
        }

        // Test that the real Rules struct can be loaded (only when YARA-X is available)
        #[cfg(feature = "yara-x-scanning")]
        {
            // Load rules from .yar source file (YARA-X compiles on-the-fly)
            let rule_content = std::fs::read_to_string("rules/pre/secrets_leakage.yar")
                .expect("Should be able to read secrets_leakage.yar test rule file");

            let mut compiler = yara_x::Compiler::new();
            compiler
                .add_source(rule_content.as_str())
                .expect("Should be able to compile secrets_leakage.yar rule");

            let rules = compiler.build();

            // Test that the real scan method works
            let mut scanner = yara_x::Scanner::new(&rules);
            let scan_result = scanner.scan(b"test data");
            assert!(
                scan_result.is_ok(),
                "YARA-X scanning should succeed on test data"
            );

            // Real YARA-X may or may not have matches depending on rule content
            // Just verify we got a valid result (empty or with matches)
        }
    }

    #[test]
    fn test_runs_post_scan_capability_without_errors() {
        // Test that post-scan capability can be created and runs correctly
        let post_scanner = YaraScanner::new("rules", ScanPhase::PostScan);
        assert!(
            post_scanner.is_ok(),
            "YaraScanner creation should succeed for post-scan phase"
        );

        let scanner_instance =
            post_scanner.expect("Post-scan scanner creation should have succeeded");
        assert_eq!(scanner_instance.name(), "yara");
        assert_eq!(scanner_instance.phase(), ScanPhase::PostScan);

        // Create test scan data
        let mut scan_data = ScanData {
            server_info: None,
            tools: vec![],
            resources: vec![],
            prompts: vec![],
            yara_results: vec![],
            fetch_errors: vec![],
        };

        // Run post-scan scanner (should not error even with empty data)
        let result = scanner_instance.run(&mut scan_data);
        assert!(
            result.is_ok(),
            "Post-scan scanner should run successfully on scan data"
        );
        println!("Post-scan scanner test passed");
    }

    #[test]
    fn test_tracks_separate_pre_and_post_scan_statistics() {
        #[cfg(feature = "yara-x-scanning")]
        let scanner = ThreatRules::new("rules")
            .expect("Should be able to create ThreatRules with rules directory");
        #[cfg(not(feature = "yara-x-scanning"))]
        let scanner = ThreatRules::with_config("rules", false)
            .expect("Should be able to create ThreatRules with YARA disabled");

        let stats = scanner.stats();

        println!("Pre-scan rules: {}", stats.pre_scan_count);
        println!("Post-scan rules: {}", stats.post_scan_count);

        // Should have at least some pre-scan rules (only when YARA is enabled)
        #[cfg(feature = "yara-x-scanning")]
        assert!(stats.pre_scan_count > 0);
        #[cfg(not(feature = "yara-x-scanning"))]
        assert_eq!(stats.pre_scan_count, 0);

        // Should have post-scan rules if we created test rule
        if stats.post_scan_count > 0 {
            println!("Post-scan rules detected: {}", stats.post_scan_count);
        } else {
            println!("No post-scan rules found (this is expected if none were created)");
        }
    }

    #[test]
    fn test_collects_rule_names_from_loaded_files() {
        // Test that the dynamic rule names are being collected correctly
        let scanner = ThreatRules::new("rules")
            .expect("Should be able to create ThreatRules with rules directory");
        let stats = scanner.stats();

        // Should have the expected pre-scan rules
        #[cfg(feature = "yara-x-scanning")]
        {
            assert!(!stats.pre_scan_rules.is_empty());
            assert!(stats
                .pre_scan_rules
                .contains(&"command_injection".to_string()));
            assert!(stats.pre_scan_rules.contains(&"path_traversal".to_string()));
            assert!(stats
                .pre_scan_rules
                .contains(&"secrets_leakage".to_string()));
            println!("Pre-scan rules: {:?}", stats.pre_scan_rules);
        }

        #[cfg(not(feature = "yara-x-scanning"))]
        {
            assert!(stats.pre_scan_rules.is_empty());
        }

        // Post-scan rules should be empty for now
        assert!(stats.post_scan_rules.is_empty());
        println!("Post-scan rules: {:?}", stats.post_scan_rules);
    }
}
