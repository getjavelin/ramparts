use crate::security::SecurityScanResult;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Original YARA rule metadata from the rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaraRuleMetadata {
    pub name: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub confidence: Option<String>,
    pub tags: Vec<String>,
}

/// Result of a YARA scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaraScanResult {
    pub target_type: String, // "tool", "prompt", "resource"
    pub target_name: String,
    pub rule_name: String,
    pub matched_text: Option<String>,
    pub context: String,
    pub rule_metadata: Option<YaraRuleMetadata>,
    // Execution summary fields (when target_type is "summary")
    pub phase: Option<String>,
    pub rules_executed: Option<Vec<String>>,
    pub rules_passed: Option<Vec<String>>,
    pub rules_failed: Option<Vec<String>>,
    pub total_items_scanned: Option<usize>,
    pub total_matches: Option<usize>,
    pub status: Option<String>, // "success", "warning", "error"
}

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
    pub yara_results: Vec<YaraScanResult>,
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
            yara_results: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_scan_config_builder() {
        let config = ScanConfigBuilder::new()
            .timeout(120)
            .http_timeout(60)
            .detailed(true)
            .format("json".to_string())
            .auth_headers(Some(HashMap::from([(
                "Authorization".to_string(),
                "Bearer token".to_string(),
            )])))
            .build();

        assert_eq!(config.timeout, 120);
        assert_eq!(config.http_timeout, 60);
        assert_eq!(config.detailed, true);
        assert_eq!(config.format, "json");
        assert!(config.auth_headers.is_some());
    }

    #[test]
    fn test_scan_config_builder_default() {
        let config = ScanConfigBuilder::new().build();
        assert_eq!(config.timeout, 60);
        assert_eq!(config.http_timeout, 30);
        assert_eq!(config.detailed, false);
        assert_eq!(config.format, "text");
        assert!(config.auth_headers.is_none());
    }

    #[test]
    fn test_validate_scan_config_valid() {
        let options = ScanOptions {
            timeout: 60,
            http_timeout: 30,
            detailed: false,
            format: "json".to_string(),
            auth_headers: None,
        };

        let result = config_utils::validate_scan_config(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_scan_config_invalid_timeout() {
        let options = ScanOptions {
            timeout: 0,
            http_timeout: 30,
            detailed: false,
            format: "json".to_string(),
            auth_headers: None,
        };

        let result = config_utils::validate_scan_config(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Timeout must be greater than 0"));
    }

    #[test]
    fn test_validate_scan_config_invalid_http_timeout() {
        let options = ScanOptions {
            timeout: 60,
            http_timeout: 0,
            detailed: false,
            format: "json".to_string(),
            auth_headers: None,
        };

        let result = config_utils::validate_scan_config(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HTTP timeout must be greater than 0"));
    }

    #[test]
    fn test_validate_scan_config_timeout_less_than_http_timeout() {
        let options = ScanOptions {
            timeout: 10,
            http_timeout: 30,
            detailed: false,
            format: "json".to_string(),
            auth_headers: None,
        };

        let result = config_utils::validate_scan_config(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Total timeout must be greater than or equal to HTTP timeout"));
    }

    #[test]
    fn test_scan_result_new() {
        let result = ScanResult::new("http://example.com".to_string());
        assert_eq!(result.url, "http://example.com");
        assert!(matches!(result.status, ScanStatus::Success));
        assert_eq!(result.tools.len(), 0);
        assert_eq!(result.resources.len(), 0);
        assert_eq!(result.prompts.len(), 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_scan_result_add_error() {
        let mut result = ScanResult::new("http://example.com".to_string());
        result.add_error("Test error".to_string());

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Test error");
    }

    #[test]
    fn test_tool_response_from_json() {
        let response = json!({
            "result": {
                "tools": [
                    {
                        "name": "test_tool",
                        "description": "A test tool",
                        "inputSchema": {"type": "object"},
                        "outputSchema": {"type": "string"}
                    }
                ]
            }
        });

        let tool_response = ToolResponse::from_json_response(&response);
        assert!(tool_response.is_ok());

        let tool_response = tool_response.unwrap();
        assert_eq!(tool_response.tools.len(), 1);
        assert_eq!(tool_response.tools[0].name, "test_tool");
        assert_eq!(
            tool_response.tools[0].description,
            Some("A test tool".to_string())
        );
    }

    #[test]
    fn test_tool_response_from_json_missing_result() {
        let response = json!({
            "other_key": {
                "tools": []
            }
        });

        let tool_response = ToolResponse::from_json_response(&response);
        assert!(tool_response.is_ok());
        assert_eq!(tool_response.unwrap().tools.len(), 0);
    }

    #[test]
    fn test_prompt_response_from_json() {
        let response = json!({
            "result": {
                "prompts": [
                    {
                        "name": "test_prompt",
                        "description": "A test prompt",
                        "arguments": [
                            {
                                "name": "arg1",
                                "description": "First argument",
                                "required": true
                            }
                        ]
                    }
                ]
            }
        });

        let prompt_response = PromptResponse::from_json_response(&response);
        assert!(prompt_response.is_ok());

        let prompt_response = prompt_response.unwrap();
        assert_eq!(prompt_response.prompts.len(), 1);
        assert_eq!(prompt_response.prompts[0].name, "test_prompt");
        assert_eq!(
            prompt_response.prompts[0].description,
            Some("A test prompt".to_string())
        );
        assert!(prompt_response.prompts[0].arguments.is_some());
        assert_eq!(
            prompt_response.prompts[0].arguments.as_ref().unwrap().len(),
            1
        );
    }

    #[test]
    fn test_prompt_response_from_json_missing_result() {
        let response = json!({
            "other_key": {
                "prompts": []
            }
        });

        let prompt_response = PromptResponse::from_json_response(&response);
        assert!(prompt_response.is_ok());
        assert_eq!(prompt_response.unwrap().prompts.len(), 0);
    }

    #[test]
    fn test_yara_rule_metadata() {
        let metadata = YaraRuleMetadata {
            name: Some("test_rule".to_string()),
            author: Some("Test Author".to_string()),
            date: Some("2024-01-01".to_string()),
            version: Some("1.0".to_string()),
            description: Some("A test rule".to_string()),
            severity: Some("HIGH".to_string()),
            category: Some("security".to_string()),
            confidence: Some("high".to_string()),
            tags: vec!["security".to_string(), "test".to_string()],
        };

        assert_eq!(metadata.name, Some("test_rule".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.severity, Some("HIGH".to_string()));
        assert_eq!(metadata.tags.len(), 2);
    }

    #[test]
    fn test_yara_scan_result() {
        let result = YaraScanResult {
            target_type: "tool".to_string(),
            target_name: "test_tool".to_string(),
            rule_name: "test_rule".to_string(),
            matched_text: Some("matched content".to_string()),
            context: "test context".to_string(),
            rule_metadata: None,
            phase: Some("pre-scan".to_string()),
            rules_executed: Some(vec!["rule1".to_string(), "rule2".to_string()]),
            rules_passed: Some(vec!["rule1".to_string()]),
            rules_failed: Some(vec!["rule2".to_string()]),
            total_items_scanned: Some(10),
            total_matches: Some(2),
            status: Some("warning".to_string()),
        };

        assert_eq!(result.target_type, "tool");
        assert_eq!(result.target_name, "test_tool");
        assert_eq!(result.rule_name, "test_rule");
        assert_eq!(result.matched_text, Some("matched content".to_string()));
        assert_eq!(result.context, "test context");
        assert_eq!(result.phase, Some("pre-scan".to_string()));
        assert_eq!(result.total_items_scanned, Some(10));
        assert_eq!(result.total_matches, Some(2));
        assert_eq!(result.status, Some("warning".to_string()));
    }
}
