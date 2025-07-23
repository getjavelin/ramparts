use crate::config::MCPConfigManager;
use crate::security::{SecurityScanResult, SecurityScanner};
use crate::types::*;
use crate::utils::{
    error_utils, parse_jsonrpc_array_response, performance::track_performance, retry_with_backoff,
    Timer,
};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use url::Url;

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
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

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
            .write_all(format!("{}\n", request_str).as_bytes())
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
            "jsonrpc": "2.0",
            "id": self.id,
            "method": self.method,
            "params": self.params
        })
    }
}

// Optimized capability configuration
#[derive(Debug, Clone, Default)]
pub struct CapabilityConfig {
    pub timeout_ms: Option<u64>,
}

// Enum for different scan capabilities with optimized structure
#[derive(Debug, Clone)]
pub enum ScanCapability {
    Tool(CapabilityConfig),
    Resource(CapabilityConfig),
    Prompt(CapabilityConfig),
    ServerInfo(CapabilityConfig),
}

impl ScanCapability {
    // Use const for capability names to avoid string allocations
    const TOOL_SCAN_NAME: &'static str = "tool_scan";
    const RESOURCE_SCAN_NAME: &'static str = "resource_scan";
    const PROMPT_SCAN_NAME: &'static str = "prompt_scan";
    const SERVER_INFO_SCAN_NAME: &'static str = "server_info_scan";

    pub fn name(&self) -> &'static str {
        match self {
            ScanCapability::Tool(_) => Self::TOOL_SCAN_NAME,
            ScanCapability::Resource(_) => Self::RESOURCE_SCAN_NAME,
            ScanCapability::Prompt(_) => Self::PROMPT_SCAN_NAME,
            ScanCapability::ServerInfo(_) => Self::SERVER_INFO_SCAN_NAME,
        }
    }

    // Optimized execution with generic request handling
    pub async fn execute(
        &self,
        scanner: &MCPScanner,
        url: &str,
        options: &ScanOptions,
        session: &MCPSession,
        transport_type: &TransportType,
    ) -> Result<CapabilityResult> {
        let timer = Timer::start();
        let capability_name = self.name();

        // Build request based on capability type
        let request = self.build_request();

        // Execute with capability-specific timeout
        let (response, _) = if let Some(timeout_ms) = self.get_config().timeout_ms {
            let timeout_duration = Duration::from_millis(timeout_ms);
            match tokio::time::timeout(
                timeout_duration,
                scanner.send_jsonrpc_request_with_headers_and_session(
                    url,
                    request,
                    options,
                    transport_type,
                    session.session_id.clone(),
                ),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    return Err(anyhow!(
                        "Capability {} timed out after {}ms",
                        capability_name,
                        timeout_ms
                    ))
                }
            }
        } else {
            scanner
                .send_jsonrpc_request_with_headers_and_session(
                    url,
                    request,
                    options,
                    transport_type,
                    session.session_id.clone(),
                )
                .await?
        };

        // Use tracing for debug logging
        debug!(
            "{} response: {}",
            capability_name,
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );

        // Parse response based on capability type
        let (data, count) = self.parse_response(&response)?;

        let execution_time = timer.elapsed_ms();
        info!(
            "{} capability completed in {}ms, found {} items",
            capability_name, execution_time, count
        );

        let result = CapabilityResult::success(capability_name.to_string(), data, execution_time);

        // Log execution time for monitoring
        if result.is_slow_execution() {
            warn!(
                "{} capability took {}ms (slow execution)",
                capability_name,
                result.execution_time_ms()
            );
        }

        Ok(result)
    }

    // Generic request builder to reduce redundancy
    fn build_request(&self) -> serde_json::Value {
        match self {
            ScanCapability::Tool(_) => {
                let request = JsonRpcRequest::new("tools/list", 2).to_json();
                debug!(
                    "tool_scan request: {}",
                    serde_json::to_string_pretty(&request).unwrap_or_default()
                );
                request
            }
            ScanCapability::Resource(_) => {
                let request = JsonRpcRequest::new("resources/list", 3).to_json();
                debug!(
                    "resource_scan request: {}",
                    serde_json::to_string_pretty(&request).unwrap_or_default()
                );
                request
            }
            ScanCapability::Prompt(_) => {
                let request = JsonRpcRequest::new("prompts/list", 4).to_json();
                debug!(
                    "prompt_scan request: {}",
                    serde_json::to_string_pretty(&request).unwrap_or_default()
                );
                request
            }
            ScanCapability::ServerInfo(_) => JsonRpcRequest::new("server/info", 5).to_json(),
        }
    }

    // Generic response parser to reduce redundancy
    fn parse_response(&self, response: &serde_json::Value) -> Result<(serde_json::Value, usize)> {
        match self {
            ScanCapability::Tool(_) => {
                let tool_response = ToolResponse::from_json_response(response)?;
                let count = tool_response.total_count;
                info!("Tool scan found {} tools", count);
                Ok((serde_json::to_value(tool_response)?, count))
            }
            ScanCapability::Resource(_) => {
                let resources = parse_jsonrpc_array_response::<MCPResource>(response, "resources")?;
                let count = resources.len();
                info!("Resource scan found {} resources", count);
                for (i, resource) in resources.iter().enumerate() {
                    info!("Resource {}: {} ({})", i + 1, resource.name, resource.uri);
                }
                Ok((serde_json::to_value(resources)?, count))
            }
            ScanCapability::Prompt(_) => {
                let prompt_response = PromptResponse::from_json_response(response)?;
                let count = prompt_response.total_count;
                info!("Prompt scan found {} prompts", count);
                for (i, prompt) in prompt_response.prompts.iter().enumerate() {
                    info!(
                        "Prompt {}: {} ({})",
                        i + 1,
                        prompt.name,
                        prompt.description.as_deref().unwrap_or("No description")
                    );
                }
                Ok((serde_json::to_value(prompt_response)?, count))
            }
            ScanCapability::ServerInfo(_) => {
                // Just return the server info as-is
                let server_info = response["result"]["serverInfo"].clone();
                info!(
                    "Server info scan found server: {}",
                    server_info["name"].as_str().unwrap_or("Unknown")
                );
                Ok((server_info, 1))
            }
        }
    }

    // Get capability-specific configuration
    fn get_config(&self) -> &CapabilityConfig {
        match self {
            ScanCapability::Tool(config) => config,
            ScanCapability::Resource(config) => config,
            ScanCapability::Prompt(config) => config,
            ScanCapability::ServerInfo(config) => config,
        }
    }

    // Factory methods for creating capabilities with default config
    pub fn tool_scan() -> Self {
        Self::Tool(CapabilityConfig::default())
    }

    pub fn resource_scan() -> Self {
        Self::Resource(CapabilityConfig::default())
    }

    pub fn prompt_scan() -> Self {
        Self::Prompt(CapabilityConfig::default())
    }

    pub fn server_info_scan() -> Self {
        Self::ServerInfo(CapabilityConfig::default())
    }
}

// Optimized result structure with better memory layout
#[derive(Debug, Clone)]
pub struct CapabilityResult {
    pub capability_name: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl CapabilityResult {
    pub fn success(
        capability_name: String,
        data: serde_json::Value,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            capability_name,
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms,
        }
    }

    pub fn failure(capability_name: String, error: String, execution_time_ms: u64) -> Self {
        Self {
            capability_name,
            success: false,
            data: None,
            error: Some(error),
            execution_time_ms,
        }
    }

    /// Get the execution time in milliseconds
    pub fn execution_time_ms(&self) -> u64 {
        self.execution_time_ms
    }

    /// Check if the capability execution was slow (over 1 second)
    pub fn is_slow_execution(&self) -> bool {
        self.execution_time_ms > 1000
    }
}

// Optimized capability chain with parallel execution support
pub struct CapabilityChain {
    capabilities: Vec<ScanCapability>,
    parallel_execution: bool,
}

impl CapabilityChain {
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
            parallel_execution: false,
        }
    }

    pub fn add_capability(mut self, capability: ScanCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_default_capabilities() -> Self {
        Self::new()
            .add_capability(ScanCapability::server_info_scan())
            .add_capability(ScanCapability::tool_scan())
            .add_capability(ScanCapability::resource_scan())
            .add_capability(ScanCapability::prompt_scan())
    }

    // Optimized execution with parallel support
    pub async fn execute(
        &self,
        scanner: &MCPScanner,
        url: &str,
        options: &ScanOptions,
        session: &MCPSession,
        transport_type: &TransportType,
    ) -> Vec<CapabilityResult> {
        if self.parallel_execution {
            self.execute_parallel(scanner, url, options, session, transport_type)
                .await
        } else {
            self.execute_sequential(scanner, url, options, session, transport_type)
                .await
        }
    }

    // Sequential execution (original behavior)
    async fn execute_sequential(
        &self,
        scanner: &MCPScanner,
        url: &str,
        options: &ScanOptions,
        session: &MCPSession,
        transport_type: &TransportType,
    ) -> Vec<CapabilityResult> {
        let mut results = Vec::with_capacity(self.capabilities.len());

        for capability in &self.capabilities {
            info!(
                "Executing capability: [\x1b[1m{}\x1b[0m]",
                capability.name()
            );
            match capability
                .execute(scanner, url, options, session, transport_type)
                .await
            {
                Ok(result) => {
                    let result_clone = result.clone();
                    results.push(result);
                    if !result_clone.success {
                        warn!(
                            "Capability [\x1b[1m{}\x1b[0m] failed: {:?}",
                            capability.name(),
                            result_clone.error
                        );
                    }
                }
                Err(e) => {
                    let failure_result = CapabilityResult::failure(
                        capability.name().to_string(),
                        e.to_string(),
                        0, // Execution time not available in error case
                    );
                    results.push(failure_result);
                    warn!(
                        "Capability [\x1b[1m{}\x1b[0m] failed with error: {}",
                        capability.name(),
                        e
                    );
                }
            }
        }

        results
    }

    // Parallel execution for better performance
    async fn execute_parallel(
        &self,
        scanner: &MCPScanner,
        url: &str,
        options: &ScanOptions,
        session: &MCPSession,
        transport_type: &TransportType,
    ) -> Vec<CapabilityResult> {
        use futures_util::future::join_all;

        let futures: Vec<_> = self
            .capabilities
            .iter()
            .map(|capability| {
                let scanner = scanner.clone();
                let url = url.to_string();
                let options = options.clone();
                let session = session.clone();
                let capability = capability.clone();
                let transport_type = transport_type.clone();

                async move {
                    info!(
                        "Executing capability: [\x1b[1m{}\x1b[0m]",
                        capability.name()
                    );
                    match capability
                        .execute(&scanner, &url, &options, &session, &transport_type)
                        .await
                    {
                        Ok(result) => {
                            if !result.success {
                                warn!(
                                    "Capability [\x1b[1m{}\x1b[0m] failed: {:?}",
                                    capability.name(),
                                    result.error
                                );
                            }
                            result
                        }
                        Err(e) => {
                            let failure_result = CapabilityResult::failure(
                                capability.name().to_string(),
                                e.to_string(),
                                0,
                            );
                            warn!(
                                "Capability [\x1b[1m{}\x1b[0m] failed with error: {}",
                                capability.name(),
                                e
                            );
                            failure_result
                        }
                    }
                }
            })
            .collect();

        join_all(futures).await
    }
}

// MCPScanner struct
pub struct MCPScanner {
    client: Client,
    http_timeout: u64,
    capability_chain: CapabilityChain,
}

// MCPScanner implementation
impl MCPScanner {
    // Create a new MCPScanner with a timeout
    pub fn new_with_timeout(http_timeout: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(http_timeout))
            .user_agent("rampart/0.2.0")
            .build()
            .unwrap_or_default();
        Self {
            client,
            http_timeout,
            capability_chain: CapabilityChain::with_default_capabilities(),
        }
    }

    // Scan a single MCP server
    pub async fn scan_single(&self, url: &str, options: ScanOptions) -> Result<ScanResult> {
        let mut result = ScanResult::new(url.to_string());

        info!("Scanning MCP server: [\x1b[1m{}\x1b[0m]", url);

        // Detect transport type
        let transport_type = TransportType::from_url(url);
        info!("Detected transport type: {:?}", transport_type);

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
            Ok(scan_data) => {
                result.status = ScanStatus::Success;
                result.server_info = scan_data.server_info;
                result.tools = scan_data.tools.clone();
                result.resources = scan_data.resources.clone();
                result.prompts = scan_data.prompts.clone();

                // Load scanner configuration
                let config_manager = crate::config::ScannerConfigManager::new();
                let scanner_config = config_manager.load_config().unwrap_or_default();

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
                result.response_time_ms = Timer::start().elapsed_ms(); // Track actual scan time
                info!(
                    "Scan completed successfully in [\x1b[1m{}\x1b[0m]ms",
                    result.response_time_ms
                );
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

    // Scan MCP servers from IDE configuration files
    pub async fn scan_from_config(&self, options: ScanOptions) -> Result<Vec<ScanResult>> {
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
                        failed_result.add_error(format!("IDE config scan failed: {}", e));
                        results.push(failed_result);
                    }
                }
            }
        } else {
            warn!("No MCP servers found in IDE configuration files");
        }

        Ok(results)
    }

    // Perform the scan
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

        // Execute capability chain
        let capability_results = self
            .capability_chain
            .execute(self, url, options, &session, transport_type)
            .await;

        // Process capability results using trait
        let mut processor = ScanDataProcessor {
            scan_data: &mut scan_data,
        };
        for result in capability_results {
            if result.success {
                match result.capability_name.as_str() {
                    "tool_scan" => {
                        let _ = processor.process_tool_scan(result.data.unwrap());
                    }
                    "resource_scan" => {
                        let _ = processor.process_resource_scan(result.data.unwrap());
                    }
                    "prompt_scan" => {
                        let _ = processor.process_prompt_scan(result.data.unwrap());
                    }
                    _ => {
                        info!(
                            "Unknown capability result: [\x1b[1m{}\x1b[0m]",
                            result.capability_name
                        );
                    }
                }
            } else {
                warn!(
                    "Capability [\x1b[1m{}\x1b[0m] failed: {:?}",
                    result.capability_name, result.error
                );
            }
        }

        // Send shutdown notification
        let _ = self
            .shutdown_session(url, &session, options, transport_type)
            .await;

        Ok(scan_data)
    }

    // Initialize an MCP session
    async fn initialize_mcp_session(
        &self,
        url: &str,
        options: &ScanOptions,
        transport_type: &TransportType,
    ) -> Result<MCPSession> {
        // Try different protocol versions for compatibility, prioritizing latest
        let protocol_versions = ["2025-06-18", "2024-11-05", "2024-11-01", "2024-10-01"];

        for protocol_version in protocol_versions {
            let request = json!({
                "jsonrpc": "2.0",
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
                        "name": "rampart",
                        "version": "0.2.0"
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
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": "rampart",
                    "version": "0.2.0"
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

    // Send a shutdown notification to the MCP server
    async fn shutdown_session(
        &self,
        url: &str,
        session: &MCPSession,
        options: &ScanOptions,
        transport_type: &TransportType,
    ) -> Result<()> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "notifications/shutdown",
            "params": {}
        });

        // We don't care about the response for shutdown
        let _ = self
            .send_jsonrpc_request_with_headers_and_session(
                url,
                request,
                options,
                transport_type,
                session.session_id.clone(),
            )
            .await;
        Ok(())
    }

    // Send a JSON-RPC request with session ID support
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
                            .header("User-Agent", "rampart/0.2.0")
                            .header("MCP-Protocol-Version", "2025-06-18")
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
                                    if key.to_lowercase().contains("bearer") {
                                        "[REDACTED]"
                                    } else {
                                        value
                                    }
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

    // Handle an SSE response
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

    // Extract capabilities from the response
    fn extract_capabilities(&self, response: &Value) -> Vec<String> {
        let mut capabilities = Vec::new();

        if let Some(caps) = response["result"]["capabilities"].as_object() {
            for (capability, _) in caps {
                capabilities.push(capability.clone());
            }
        }

        capabilities
    }

    // Normalize a URL
    fn normalize_url(&self, url: &str) -> Result<String> {
        let transport_type = TransportType::from_url(url);

        match transport_type {
            TransportType::Http => {
                let mut url = url.to_string();

                // Add http:// if no scheme is provided
                if !url.contains("://") {
                    url = format!("http://{}", url);
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

    // Check if a command is executable (basic check)
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
            capability_chain: CapabilityChain::with_default_capabilities(),
        }
    }
}

// Scan data processor implementing the trait
struct ScanDataProcessor<'a> {
    scan_data: &'a mut ScanData,
}

impl CapabilityResultProcessor for ScanDataProcessor<'_> {
    fn process_tool_scan(&mut self, data: serde_json::Value) -> Result<()> {
        if let Ok(tool_response) = serde_json::from_value::<ToolResponse>(data) {
            self.scan_data.tools = tool_response.tools;
        }
        Ok(())
    }

    fn process_resource_scan(&mut self, data: serde_json::Value) -> Result<()> {
        if let Ok(resources) = serde_json::from_value::<Vec<MCPResource>>(data) {
            self.scan_data.resources = resources;
        }
        Ok(())
    }

    fn process_prompt_scan(&mut self, data: serde_json::Value) -> Result<()> {
        if let Ok(prompt_response) = serde_json::from_value::<PromptResponse>(data) {
            self.scan_data.prompts = prompt_response.prompts;
        }
        Ok(())
    }
}

// Scan data
#[derive(Debug)]
struct ScanData {
    server_info: Option<MCPServerInfo>,
    tools: Vec<MCPTool>,
    resources: Vec<MCPResource>,
    prompts: Vec<MCPPrompt>,
}

// Scan data implementation
impl ScanData {
    fn new() -> Self {
        Self {
            server_info: None,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        }
    }
}
