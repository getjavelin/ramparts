use crate::security::SecurityScanResult;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// CORE TYPES - Main data structures for MCP scanning
// ============================================================================

/// Configuration builder for scan options
#[derive(Debug, Clone)]
pub struct ScanConfigBuilder {
    timeout: u64,
    http_timeout: u64,
    detailed: bool,
    format: String,
    auth_headers: Option<HashMap<String, String>>,
}

impl Default for ScanConfigBuilder {
    fn default() -> Self {
        Self {
            timeout: 60,
            http_timeout: 30,
            detailed: false,
            format: "text".to_string(),
            auth_headers: None,
        }
    }
}

impl ScanConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn http_timeout(mut self, http_timeout: u64) -> Self {
        self.http_timeout = http_timeout;
        self
    }

    pub fn detailed(mut self, detailed: bool) -> Self {
        self.detailed = detailed;
        self
    }

    pub fn format(mut self, format: String) -> Self {
        self.format = format;
        self
    }

    pub fn auth_headers(mut self, auth_headers: Option<HashMap<String, String>>) -> Self {
        self.auth_headers = auth_headers;
        self
    }

    pub fn build(self) -> ScanOptions {
        ScanOptions {
            timeout: self.timeout,
            http_timeout: self.http_timeout,
            detailed: self.detailed,
            format: self.format,
            auth_headers: self.auth_headers,
        }
    }
}

/// Configuration validation utilities
pub mod config_utils {
    use super::*;

    pub fn validate_scan_config(options: &ScanOptions) -> Result<()> {
        if options.timeout == 0 {
            return Err(anyhow::anyhow!("Timeout must be greater than 0"));
        }

        if options.http_timeout == 0 {
            return Err(anyhow::anyhow!("HTTP timeout must be greater than 0"));
        }

        if options.timeout < options.http_timeout {
            return Err(anyhow::anyhow!(
                "Total timeout must be greater than or equal to HTTP timeout"
            ));
        }

        Ok(())
    }
}

/// Trait for processing capability results
pub trait CapabilityResultProcessor {
    fn process_tool_scan(&mut self, data: serde_json::Value) -> Result<()>;
    fn process_resource_scan(&mut self, data: serde_json::Value) -> Result<()>;
    fn process_prompt_scan(&mut self, data: serde_json::Value) -> Result<()>;
}

/// Scan options configuration
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub timeout: u64,
    pub http_timeout: u64,
    pub detailed: bool,
    pub format: String,
    pub auth_headers: Option<HashMap<String, String>>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            timeout: 60,
            http_timeout: 30,
            detailed: false,
            format: "text".to_string(),
            auth_headers: None,
        }
    }
}

/// Main scan result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub url: String,
    pub status: ScanStatus,
    pub timestamp: DateTime<Utc>,
    pub response_time_ms: u64,
    pub server_info: Option<MCPServerInfo>,
    pub tools: Vec<MCPTool>,
    pub resources: Vec<MCPResource>,
    pub prompts: Vec<MCPPrompt>,
    pub security_issues: Option<SecurityScanResult>,
    pub errors: Vec<String>,
}

/// Scan status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    Success,
    Failed(String),
    Timeout,
    ConnectionError(String),
}

impl ScanResult {
    pub fn new(url: String) -> Self {
        Self {
            url,
            status: ScanStatus::Success,
            timestamp: Utc::now(),
            response_time_ms: 0,
            server_info: None,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            security_issues: None,
            errors: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error.clone());
        if matches!(self.status, ScanStatus::Success) {
            self.status = ScanStatus::Failed(error);
        }
    }
}

/// MCP server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub deprecated: bool,
    // Preserve the original JSON schema from the MCP server
    pub raw_json: Option<serde_json::Value>,
}

/// MCP session information
#[derive(Debug, Clone)]
pub struct MCPSession {
    pub server_info: Option<MCPServerInfo>,
    pub session_id: Option<String>,
}

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResource {
    #[serde(rename = "uri")]
    pub uri: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "size")]
    pub size: Option<u64>,
    #[serde(rename = "metadata")]
    pub metadata: HashMap<String, serde_json::Value>,
    // Preserve the original JSON schema from the MCP server
    #[serde(skip)]
    pub raw_json: Option<serde_json::Value>,
}

/// MCP prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

/// MCP prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<MCPPromptArgument>>,
    // Preserve the original JSON schema from the MCP server
    #[serde(skip)]
    pub raw_json: Option<serde_json::Value>,
}

/// Tool response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub tools: Vec<MCPTool>,
    pub total_count: usize,
    pub response_timestamp: DateTime<Utc>,
}

/// Prompt response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptResponse {
    pub prompts: Vec<MCPPrompt>,
    pub total_count: usize,
    pub response_timestamp: DateTime<Utc>,
}

impl ToolResponse {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            total_count: 0,
            response_timestamp: Utc::now(),
        }
    }

    pub fn add_tool(&mut self, tool: MCPTool) {
        self.tools.push(tool);
        self.total_count = self.tools.len();
    }

    pub fn from_json_response(response: &serde_json::Value) -> Result<Self> {
        let mut tool_response = ToolResponse::new();

        if let Some(tools_array) = response["result"]["tools"].as_array() {
            for tool_value in tools_array {
                let tool = Self::parse_tool_from_json(tool_value)?;
                tool_response.add_tool(tool);
            }
        }

        Ok(tool_response)
    }

    fn parse_tool_from_json(tool_value: &serde_json::Value) -> Result<MCPTool> {
        let name = tool_value["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Tool name is required"))?
            .to_string();

        let description = tool_value["description"].as_str().map(|s| s.to_string());

        let input_schema = tool_value["inputSchema"].clone();
        let output_schema = tool_value["outputSchema"].clone();

        let mut parameters = HashMap::new();
        if let Some(params) = tool_value["parameters"].as_object() {
            for (key, value) in params {
                parameters.insert(key.clone(), value.clone());
            }
        }

        let category = tool_value["category"].as_str().map(|s| s.to_string());

        let mut tags = Vec::new();
        if let Some(tags_array) = tool_value["tags"].as_array() {
            for tag in tags_array {
                if let Some(tag_str) = tag.as_str() {
                    tags.push(tag_str.to_string());
                }
            }
        }

        let deprecated = tool_value["deprecated"].as_bool().unwrap_or(false);

        Ok(MCPTool {
            name,
            description,
            input_schema: if input_schema.is_null() {
                None
            } else {
                Some(input_schema)
            },
            output_schema: if output_schema.is_null() {
                None
            } else {
                Some(output_schema)
            },
            parameters,
            category,
            tags,
            deprecated,
            raw_json: Some(tool_value.clone()),
        })
    }
}

impl PromptResponse {
    pub fn new() -> Self {
        Self {
            prompts: Vec::new(),
            total_count: 0,
            response_timestamp: Utc::now(),
        }
    }

    pub fn add_prompt(&mut self, prompt: MCPPrompt) {
        self.prompts.push(prompt);
        self.total_count = self.prompts.len();
    }

    pub fn from_json_response(response: &serde_json::Value) -> Result<Self> {
        let mut prompt_response = PromptResponse::new();

        if let Some(prompts_array) = response["result"]["prompts"].as_array() {
            for prompt_value in prompts_array {
                let prompt = Self::parse_prompt_from_json(prompt_value)?;
                prompt_response.add_prompt(prompt);
            }
        }

        Ok(prompt_response)
    }

    fn parse_prompt_from_json(prompt_value: &serde_json::Value) -> Result<MCPPrompt> {
        let name = prompt_value["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Prompt name is required"))?
            .to_string();

        let description = prompt_value["description"].as_str().map(|s| s.to_string());

        let mut arguments = None;
        if let Some(args_array) = prompt_value["arguments"].as_array() {
            let mut args = Vec::new();
            for arg_value in args_array {
                let arg_name = arg_value["name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Argument name is required"))?
                    .to_string();

                let arg_description = arg_value["description"].as_str().map(|s| s.to_string());
                let required = arg_value["required"].as_bool();

                args.push(MCPPromptArgument {
                    name: arg_name,
                    description: arg_description,
                    required,
                });
            }
            arguments = Some(args);
        }

        Ok(MCPPrompt {
            name,
            description,
            arguments,
            raw_json: Some(prompt_value.clone()),
        })
    }
}
