use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// Re-export commonly used types
pub use anyhow;
pub use chrono;
pub use serde;
pub use serde_json;
pub use tokio;
pub use tracing;
pub use uuid;

// ============================================================================
// CORE MCP TYPES - Shared between scan and proxy components
// ============================================================================

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
    #[serde(skip)]
    pub raw_json: Option<serde_json::Value>,
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

/// MCP session information
#[derive(Debug, Clone)]
pub struct MCPSession {
    pub server_info: Option<MCPServerInfo>,
    pub endpoint_url: String, // Store the successful endpoint URL for reuse
    pub auth_headers: Option<HashMap<String, String>>, // Store auth headers for reuse
    pub session_id: Option<String>, // Store session ID for stateful MCP servers (e.g., GitHub Copilot)
}

// ============================================================================
// CONFIGURATION TYPES
// ============================================================================

/// Base configuration for MCP operations
#[derive(Debug, Clone)]
pub struct MCPConfig {
    pub timeout: u64,
    pub http_timeout: u64,
    pub auth_headers: Option<HashMap<String, String>>,
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            timeout: 60,
            http_timeout: 30,
            auth_headers: None,
        }
    }
}

// ============================================================================
// UTILITY FUNCTIONS
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
    pub fn wrap_error<T>(result: Result<T>, context: &str) -> Result<T> {
        result.map_err(|e| anyhow::anyhow!("{context}: {e}"))
    }
}

/// Performance tracking utilities
pub mod performance {
    use super::{Result, Timer};
    use tracing::debug;

    pub struct PerformanceTracker {
        operation_name: String,
        timer: Timer,
    }

    impl PerformanceTracker {
        pub fn start(operation_name: &str) -> Self {
            debug!("Starting operation: {}", operation_name);
            Self {
                operation_name: operation_name.to_string(),
                timer: Timer::start(),
            }
        }

        pub fn finish(self) -> u64 {
            let elapsed = self.timer.elapsed_ms();
            debug!("Completed operation '{}' in {}ms", self.operation_name, elapsed);
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

/// Configuration validation utilities
pub mod config_utils {
    use super::{anyhow, Result, MCPConfig};

    pub fn validate_mcp_config(config: &MCPConfig) -> Result<()> {
        if config.timeout == 0 {
            return Err(anyhow::anyhow!("Timeout must be greater than 0"));
        }

        if config.http_timeout == 0 {
            return Err(anyhow::anyhow!("HTTP timeout must be greater than 0"));
        }

        if config.timeout < config.http_timeout {
            return Err(anyhow::anyhow!(
                "Total timeout must be greater than or equal to HTTP timeout"
            ));
        }

        Ok(())
    }
}

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default timeout values
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
pub const DEFAULT_HTTP_TIMEOUT_SECONDS: u64 = 30;

/// MCP protocol constants
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
pub const MCP_JSONRPC_VERSION: &str = "2.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_functionality() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_config_validation() {
        let config = MCPConfig::default();
        assert!(config_utils::validate_mcp_config(&config).is_ok());

        let invalid_config = MCPConfig {
            timeout: 0,
            http_timeout: 30,
            auth_headers: None,
        };
        assert!(config_utils::validate_mcp_config(&invalid_config).is_err());
    }
}
