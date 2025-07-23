use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// MCP Configuration structure for reading from IDE config files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPConfig {
    /// List of MCP server configurations from IDE config files
    pub servers: Option<Vec<MCPServerConfig>>,
    /// Global configuration options
    pub options: Option<MCPGlobalOptions>,
    /// Authentication headers for all servers
    pub auth_headers: Option<HashMap<String, String>>,
}

/// Individual MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Server name or identifier
    pub name: Option<String>,
    /// Server URL
    pub url: String,
    /// Server description
    pub description: Option<String>,
    /// Authentication headers specific to this server
    pub auth_headers: Option<HashMap<String, String>>,
    /// Server-specific options
    pub options: Option<MCPServerOptions>,
}

/// Global configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPGlobalOptions {
    /// Default timeout in seconds
    pub timeout: Option<u64>,
    /// Default HTTP timeout in seconds
    pub http_timeout: Option<u64>,
    /// Default output format
    pub format: Option<String>,
    /// Whether to include detailed output by default
    pub detailed: Option<bool>,
}

/// Server-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerOptions {
    /// Server-specific timeout
    pub timeout: Option<u64>,
    /// Server-specific HTTP timeout
    pub http_timeout: Option<u64>,
    /// Server-specific output format
    pub format: Option<String>,
    /// Whether to include detailed output for this server
    pub detailed: Option<bool>,
}

fn is_verbose_or_debug() -> bool {
    // Check for MCP_DEBUG or RUST_LOG=debug/info
    std::env::var("MCP_DEBUG").ok().as_deref() == Some("1")
        || std::env::var("RUST_LOG")
            .map(|v| v.contains("debug") || v.contains("info"))
            .unwrap_or(false)
}

/// IDE configuration file manager for MCP scanner
pub struct MCPConfigManager {
    config_paths: Vec<PathBuf>,
}

impl MCPConfigManager {
    /// Create a new configuration manager with default IDE config paths
    pub fn new() -> Self {
        let mut paths = Vec::new();

        if let Some(home_dir) = dirs::home_dir() {
            // Cursor IDE MCP config
            let cursor_config = home_dir.join(".cursor").join("mcp.json");
            paths.push(cursor_config);

            // Codium/Windsurf MCP config
            let codium_config = home_dir
                .join(".codium")
                .join("windsurf")
                .join("mcp_config.json");
            paths.push(codium_config);

            // VS Code MCP config (if it exists)
            let vscode_config = home_dir.join(".vscode").join("mcp.json");
            paths.push(vscode_config);

            // Neovim MCP config (if it exists)
            let neovim_config = home_dir.join(".config").join("nvim").join("mcp.json");
            paths.push(neovim_config);

            // Helix editor MCP config (if it exists)
            let helix_config = home_dir.join(".config").join("helix").join("mcp.json");
            paths.push(helix_config);
        }

        Self {
            config_paths: paths,
        }
    }

    /// Load configuration from all available IDE config files
    pub fn load_config(&self) -> Result<MCPConfig> {
        let mut merged_config = MCPConfig::default();

        for path in &self.config_paths {
            if let Ok(config) = self.load_config_from_path(path) {
                self.merge_config(&mut merged_config, &config);
                info!(
                    "Loaded MCP configuration from IDE config: {}",
                    path.display()
                );
                if !is_verbose_or_debug() {
                    println!("Loaded MCP server config from: {}", path.display());
                }
            } else {
                debug!(
                    "No MCP configuration found at IDE config: {}",
                    path.display()
                );
            }
        }

        Ok(merged_config)
    }

    /// Load configuration from a specific IDE config path
    pub fn load_config_from_path(&self, path: &Path) -> Result<MCPConfig> {
        if !path.exists() {
            return Err(anyhow!(
                "IDE configuration file does not exist: {}",
                path.display()
            ));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read IDE config file {}: {}", path.display(), e))?;

        let config: MCPConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse IDE config file {}: {}", path.display(), e))?;

        Ok(config)
    }

    /// Merge two configurations, with the second one taking precedence
    fn merge_config(&self, base: &mut MCPConfig, other: &MCPConfig) {
        // Merge servers
        if let Some(other_servers) = &other.servers {
            match &mut base.servers {
                Some(base_servers) => {
                    base_servers.extend(other_servers.clone());
                }
                None => {
                    base.servers = Some(other_servers.clone());
                }
            }
        }

        // Merge global options
        if let Some(other_options) = &other.options {
            match &mut base.options {
                Some(base_options) => {
                    if other_options.timeout.is_some() {
                        base_options.timeout = other_options.timeout;
                    }
                    if other_options.http_timeout.is_some() {
                        base_options.http_timeout = other_options.http_timeout;
                    }
                    if other_options.format.is_some() {
                        base_options.format = other_options.format.clone();
                    }
                    if other_options.detailed.is_some() {
                        base_options.detailed = other_options.detailed;
                    }
                }
                None => {
                    base.options = Some(other_options.clone());
                }
            }
        }

        // Merge auth headers
        if let Some(other_auth_headers) = &other.auth_headers {
            match &mut base.auth_headers {
                Some(base_auth_headers) => {
                    for (key, value) in other_auth_headers {
                        base_auth_headers.insert(key.clone(), value.clone());
                    }
                }
                None => {
                    base.auth_headers = Some(other_auth_headers.clone());
                }
            }
        }
    }

    /// Check if any IDE configuration files exist
    pub fn has_config_files(&self) -> bool {
        self.config_paths.iter().any(|path| path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_load_config_from_path() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        let config_content = r#"{
            "servers": [
                {
                    "name": "test-server",
                    "url": "http://localhost:3000",
                    "description": "Test server"
                }
            ],
            "options": {
                "timeout": 60,
                "format": "json"
            }
        }"#;

        fs::write(&config_path, config_content).unwrap();

        let manager = MCPConfigManager::new();
        let config = manager.load_config_from_path(&config_path).unwrap();

        assert!(config.servers.is_some());
        assert_eq!(config.servers.unwrap().len(), 1);
        assert!(config.options.is_some());
    }

    #[test]
    fn test_merge_config() {
        let mut base = MCPConfig::default();
        let other = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("server1".to_string()),
                url: "http://localhost:3000".to_string(),
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: Some(MCPGlobalOptions {
                timeout: Some(60),
                http_timeout: None,
                format: Some("json".to_string()),
                detailed: None,
            }),
            auth_headers: None,
        };

        let manager = MCPConfigManager::new();
        manager.merge_config(&mut base, &other);

        assert!(base.servers.is_some());
        assert_eq!(base.servers.unwrap().len(), 1);
        assert!(base.options.is_some());
    }
}

/// Scanner Configuration structure for reading from config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// LLM configuration
    pub llm: LLMConfig,
    /// Scanner configuration
    pub scanner: ScannerSettings,
    /// Security configuration
    pub security: SecurityConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
}

/// LLM Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Model provider (openai, anthropic, local, etc.)
    pub provider: String,
    /// Model name/identifier
    pub model: String,
    /// Base URL for the API
    pub base_url: String,
    /// API key (can also be set via environment variable)
    pub api_key: String,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Maximum tokens for LLM responses
    pub max_tokens: u32,
    /// Temperature for LLM responses
    pub temperature: f32,
}

/// Scanner Settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerSettings {
    /// Default HTTP timeout for MCP server connections (seconds)
    pub http_timeout: u64,
    /// Default scan timeout (seconds)
    pub scan_timeout: u64,
    /// Enable/disable detailed output
    pub detailed: bool,
    /// Output format (json, table, text)
    pub format: String,
    /// Enable/disable parallel execution
    pub parallel: bool,
    /// Number of retries for failed requests
    pub max_retries: u32,
    /// Initial delay for retry backoff (milliseconds)
    pub retry_delay_ms: u64,
    /// Maximum number of tools to process in a single LLM batch
    pub llm_batch_size: u32,
}

/// Security Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable/disable security scanning
    pub enabled: bool,
    /// Minimum severity level to report
    pub min_severity: String,
    /// Enable/disable specific vulnerability checks
    pub checks: SecurityChecks,
}

/// Security Checks Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityChecks {
    pub tool_poisoning: bool,
    pub secrets_leakage: bool,
    pub sql_injection: bool,
    pub command_injection: bool,
    pub path_traversal: bool,
    pub auth_bypass: bool,
    pub prompt_injection: bool,
    pub pii_leakage: bool,
    pub jailbreak: bool,
}

/// Logging Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable/disable colored output
    pub colored: bool,
    /// Enable/disable timestamps in logs
    pub timestamps: bool,
}

/// Performance Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable/disable performance tracking
    pub tracking: bool,
    /// Threshold for slow execution warnings (milliseconds)
    pub slow_threshold_ms: u64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            llm: LLMConfig {
                provider: "openai".to_string(),
                model: "gpt-4o".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "".to_string(),
                timeout: 30,
                max_tokens: 4000,
                temperature: 0.1,
            },
            scanner: ScannerSettings {
                http_timeout: 30,
                scan_timeout: 60,
                detailed: false,
                format: "table".to_string(),
                parallel: true,
                max_retries: 3,
                retry_delay_ms: 1000,
                llm_batch_size: 10,
            },
            security: SecurityConfig {
                enabled: true,
                min_severity: "low".to_string(),
                checks: SecurityChecks {
                    tool_poisoning: true,
                    secrets_leakage: true,
                    sql_injection: true,
                    command_injection: true,
                    path_traversal: true,
                    auth_bypass: true,
                    prompt_injection: true,
                    pii_leakage: true,
                    jailbreak: true,
                },
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                colored: true,
                timestamps: true,
            },
            performance: PerformanceConfig {
                tracking: true,
                slow_threshold_ms: 5000,
            },
        }
    }
}

/// Scanner Configuration Manager
pub struct ScannerConfigManager {
    config_path: PathBuf,
}

impl ScannerConfigManager {
    /// Create a new scanner configuration manager
    pub fn new() -> Self {
        let config_path = PathBuf::from("config.yaml");
        Self { config_path }
    }

    /// Load configuration from config.yaml
    pub fn load_config(&self) -> Result<ScannerConfig> {
        if !self.config_path.exists() {
            info!("No config.yaml found, using default configuration");
            return Ok(ScannerConfig::default());
        }

        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| anyhow!("Failed to read config.yaml: {}", e))?;

        let config: ScannerConfig = serde_yaml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config.yaml: {}", e))?;

        info!("Loaded configuration from config.yaml");
        Ok(config)
    }

    /// Save configuration to config.yaml
    pub fn save_config(&self, config: &ScannerConfig) -> Result<()> {
        let content = serde_yaml::to_string(config)
            .map_err(|e| anyhow!("Failed to serialize configuration: {}", e))?;

        fs::write(&self.config_path, content)
            .map_err(|e| anyhow!("Failed to write config.yaml: {}", e))?;

        info!("Saved configuration to config.yaml");
        Ok(())
    }

    /// Check if config.yaml exists
    pub fn has_config_file(&self) -> bool {
        self.config_path.exists()
    }
}
