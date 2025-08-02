use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::{debug, warn};

/// Unified MCP server configuration that handles all IDE formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedServerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    // Transport-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<TransportConfig>,
}

/// Transport configuration for HTTP-based servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    #[serde(rename = "type")]
    pub transport_type: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
}

/// Main MCP Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPConfig {
    pub servers: Option<Vec<MCPServerConfig>>,
    pub options: Option<MCPGlobalOptions>,
    pub auth_headers: Option<HashMap<String, String>>,
}

/// Individual MCP server configuration (normalized format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub name: Option<String>,
    pub url: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub description: Option<String>,
    pub auth_headers: Option<HashMap<String, String>>,
    pub options: Option<MCPServerOptions>,
}

impl MCPServerConfig {
    /// Get the URL to display for this server
    pub fn to_display_url(&self) -> String {
        if let Some(url) = &self.url {
            url.clone()
        } else if let Some(command) = &self.command {
            format!("stdio://{}", command)
        } else {
            "unknown".to_string()
        }
    }

    /// Get the URL to scan for this server (only for HTTP servers)
    pub fn scan_url(&self) -> Option<String> {
        self.url.clone()
    }
}

/// Global MCP options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPGlobalOptions {
    pub timeout: Option<u64>,
    pub format: Option<String>,
    pub max_connections: Option<u32>,
    pub retry_attempts: Option<u32>,
    pub retry_delay: Option<u64>,
    pub http_timeout: Option<u64>,
    pub detailed: Option<bool>,
}

/// Server-specific options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPServerOptions {
    pub timeout: Option<u64>,
    pub retry_attempts: Option<u32>,
    pub retry_delay: Option<u64>,
    pub max_response_size: Option<u64>,
    pub http_timeout: Option<u64>,
    pub format: Option<String>,
    pub detailed: Option<bool>,
}

/// Supported MCP client types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MCPClient {
    Cursor,
    Windsurf,
    VSCode,
    Claude,
    ClaudeCode,
    Gemini,
    Neovim,
    Helix,
    Zed,
    Zencoder,
}

impl MCPClient {
    pub fn name(self) -> &'static str {
        match self {
            MCPClient::Cursor => "cursor",
            MCPClient::Windsurf => "windsurf",
            MCPClient::VSCode => "vscode",
            MCPClient::Claude => "claude",
            MCPClient::ClaudeCode => "claude-code",
            MCPClient::Gemini => "gemini",
            MCPClient::Neovim => "neovim",
            MCPClient::Helix => "helix",
            MCPClient::Zed => "zed",
            MCPClient::Zencoder => "zencoder",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            MCPClient::Cursor => "CURSOR",
            MCPClient::Windsurf => "WINDSURF",
            MCPClient::VSCode => "VS CODE",
            MCPClient::Claude => "CLAUDE DESKTOP",
            MCPClient::ClaudeCode => "CLAUDE CODE",
            MCPClient::Gemini => "GEMINI",
            MCPClient::Neovim => "NEOVIM",
            MCPClient::Helix => "HELIX",
            MCPClient::Zed => "ZED",
            MCPClient::Zencoder => "ZENCODER",
        }
    }
}

/// IDE-specific configuration formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDEConfigFormat<T> {
    #[serde(rename = "mcpServers", skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<HashMap<String, T>>,
    #[serde(flatten)]
    pub other_fields: HashMap<String, serde_json::Value>,
}

/// VS Code specific configuration (for settings.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeSettings {
    #[serde(rename = "mcp.servers")]
    pub mcp_servers: Option<HashMap<String, UnifiedServerConfig>>,
    #[serde(flatten)]
    pub other_settings: HashMap<String, serde_json::Value>,
}

/// Trait for converting IDE-specific configurations to MCP format
pub trait ToMCPConfig {
    fn to_mcp_config(self) -> Result<MCPConfig>;
}

impl ToMCPConfig for IDEConfigFormat<UnifiedServerConfig> {
    fn to_mcp_config(self) -> Result<MCPConfig> {
        let servers_map = self.mcp_servers.or(self.servers);

        let servers = servers_map.map(|map| {
            map.into_iter()
                .filter_map(|(name, config)| {
                    // Skip disabled servers
                    if config.disabled.unwrap_or(false) {
                        return None;
                    }

                    Some(convert_unified_server_config(name, config))
                })
                .collect()
        });

        Ok(MCPConfig {
            servers,
            options: None,
            auth_headers: None,
        })
    }
}

impl ToMCPConfig for VSCodeSettings {
    fn to_mcp_config(self) -> Result<MCPConfig> {
        let servers = self.mcp_servers.map(|servers_map| {
            servers_map
                .into_iter()
                .map(|(name, config)| convert_unified_server_config(name, config))
                .collect()
        });

        Ok(MCPConfig {
            servers,
            options: None,
            auth_headers: None,
        })
    }
}

/// Convert unified server config to normalized MCP server config
fn convert_unified_server_config(name: String, config: UnifiedServerConfig) -> MCPServerConfig {
    let url = determine_server_url(&name, &config);

    MCPServerConfig {
        name: Some(name),
        url,
        command: config.command,
        args: config.args,
        env: config.env,
        description: config.description,
        auth_headers: None,
        options: None,
    }
}

/// Determine the correct URL for a server configuration
fn determine_server_url(name: &str, config: &UnifiedServerConfig) -> Option<String> {
    // If explicit URL is provided, use it
    if let Some(url) = &config.url {
        return Some(normalize_url(url));
    }

    // If transport config is provided, build URL from it
    if let Some(transport) = &config.transport {
        let host = transport.host.as_deref().unwrap_or("localhost");
        let port = transport.port.unwrap_or(8080);
        let scheme = match transport.transport_type.as_deref() {
            Some("http" | "streamable-http") => "http",
            Some("https") => "https",
            _ => "http",
        };
        return Some(format!("{}://{}:{}", scheme, host, port));
    }

    // If command is provided, this is a STDIO server - no URL needed
    if config.command.is_some() {
        return None;
    }

    // Default fallback for HTTP servers
    Some(format!("http://localhost:8080/{}", name))
}

/// Normalize URL format
fn normalize_url(url: &str) -> String {
    if url.starts_with("localhost:") {
        format!("http://{}", url)
    } else if url.starts_with("127.0.0.1:") {
        format!("http://{}", url)
    } else if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("http://{}", url)
    } else {
        url.to_string()
    }
}

/// Configuration path cache
static CONFIG_PATHS_CACHE: LazyLock<Vec<(PathBuf, MCPClient)>> =
    LazyLock::new(MCPConfigManager::discover_config_paths);

/// IDE configuration file manager for MCP scanner
pub struct MCPConfigManager {
    config_paths: Vec<(PathBuf, MCPClient)>,
}

impl Default for MCPConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPConfigManager {
    /// Creates a new MCPConfigManager using the cached configuration paths
    pub fn new() -> Self {
        Self {
            config_paths: CONFIG_PATHS_CACHE.clone(),
        }
    }

    /// Creates a new MCPConfigManager without using the cache (for testing)
    pub fn new_uncached() -> Self {
        Self {
            config_paths: Self::discover_config_paths(),
        }
    }

    /// Check if any configuration files exist
    pub fn has_config_files(&self) -> bool {
        self.config_paths.iter().any(|(path, _)| path.exists())
    }

    /// Discover MCP configuration paths based on platform and IDE
    fn discover_config_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();

        // Add workspace-level paths first (highest priority)
        paths.extend(Self::get_workspace_paths());

        // Add platform-specific paths
        #[cfg(target_os = "windows")]
        paths.extend(Self::get_windows_paths());

        #[cfg(target_os = "macos")]
        paths.extend(Self::get_macos_paths());

        #[cfg(target_os = "linux")]
        paths.extend(Self::get_unix_paths());

        paths
    }

    /// Get workspace-level MCP configuration paths
    fn get_workspace_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Workspace configurations (highest priority)
        let workspace_configs = [
            (".vscode/mcp.json", MCPClient::VSCode),
            (".vscode/settings.json", MCPClient::VSCode),
            (".cursor/mcp.json", MCPClient::Cursor),
            (".cursor/settings.json", MCPClient::Cursor),
            (".claude.json", MCPClient::ClaudeCode),
            (".claude/mcp.json", MCPClient::Claude),
            (".windsurf/mcp.json", MCPClient::Windsurf),
            (".windsurf/mcp_config.json", MCPClient::Windsurf),
        ];

        for (path, client) in workspace_configs {
            paths.push((current_dir.join(path), client));
        }

        // Check parent directories up to git root or home
        let mut dir = current_dir.as_path();
        while let Some(parent) = dir.parent() {
            if parent == dir || Self::is_project_root(parent) {
                break;
            }

            for (path, client) in workspace_configs {
                paths.push((parent.join(path), client));
            }

            dir = parent;
        }

        paths
    }

    /// Check if directory is a project root
    fn is_project_root(dir: &Path) -> bool {
        dir.join(".git").exists()
            || dir.join("package.json").exists()
            || dir.join("Cargo.toml").exists()
            || dir.join("pyproject.toml").exists()
    }

    /// Get platform-specific paths using a unified approach
    fn get_platform_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();

        if let Some(home_dir) = dirs::home_dir() {
            // User-level configs
            let user_configs = [
                (".cursor/mcp.json", MCPClient::Cursor),
                (".vscode/mcp.json", MCPClient::VSCode),
                (".vscode/settings.json", MCPClient::VSCode),
                (".claude/mcp.json", MCPClient::Claude),
                (".claude.json", MCPClient::ClaudeCode),
                (".windsurf/mcp.json", MCPClient::Windsurf),
                (".gemini/settings.json", MCPClient::Gemini),
            ];

            for (path, client) in user_configs {
                paths.push((home_dir.join(path), client));
            }
        }

        if let Some(config_dir) = dirs::config_dir() {
            // Application-specific configs
            let app_configs = [
                ("Code/User/mcp.json", MCPClient::VSCode),
                ("Code/User/settings.json", MCPClient::VSCode),
                ("Cursor/User/mcp.json", MCPClient::Cursor),
                ("Windsurf/User/mcp.json", MCPClient::Windsurf),
                ("nvim/mcp.json", MCPClient::Neovim),
                ("helix/mcp.json", MCPClient::Helix),
                ("zed/mcp.json", MCPClient::Zed),
            ];

            for (path, client) in app_configs {
                paths.push((config_dir.join(path), client));
            }
        }

        paths
    }

    /// Windows-specific paths
    #[cfg(target_os = "windows")]
    fn get_windows_paths() -> Vec<(PathBuf, MCPClient)> {
        Self::get_platform_paths()
    }

    /// macOS-specific paths
    #[cfg(target_os = "macos")]
    fn get_macos_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Self::get_platform_paths();

        if let Some(home_dir) = dirs::home_dir() {
            let app_support = home_dir.join("Library/Application Support");

            // macOS-specific Claude Desktop path
            paths.push((
                app_support.join("Claude/claude_desktop_config.json"),
                MCPClient::Claude,
            ));
        }

        paths
    }

    /// Unix/Linux-specific paths
    #[cfg(target_os = "linux")]
    fn get_unix_paths() -> Vec<(PathBuf, MCPClient)> {
        Self::get_platform_paths()
    }

    /// Detect client type from path
    pub fn detect_client<P: AsRef<Path>>(path: P) -> Option<MCPClient> {
        let path = path.as_ref();
        let _path_str = path.to_string_lossy().to_lowercase();

        // Check for specific filenames first
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            match filename {
                ".claude.json" => return Some(MCPClient::ClaudeCode),
                "claude_desktop_config.json" => return Some(MCPClient::Claude),
                _ => {}
            }
        }

        // Check path components
        for component in path.components() {
            if let Some(comp_str) = component.as_os_str().to_str() {
                let comp_lower = comp_str.to_lowercase();
                match comp_lower.as_str() {
                    "cursor" | ".cursor" => return Some(MCPClient::Cursor),
                    "windsurf" | ".windsurf" => return Some(MCPClient::Windsurf),
                    "claude" | ".claude" => return Some(MCPClient::Claude),
                    "code" | "vscode" | ".vscode" => return Some(MCPClient::VSCode),
                    "gemini" | ".gemini" => return Some(MCPClient::Gemini),
                    "zed" => return Some(MCPClient::Zed),
                    "nvim" | "neovim" => return Some(MCPClient::Neovim),
                    "helix" => return Some(MCPClient::Helix),
                    _ => {}
                }
            }
        }

        None
    }

    /// Load configurations grouped by IDE
    pub fn load_config_by_ide(&self) -> Vec<(String, MCPConfig)> {
        let mut configs_by_ide = Vec::new();

        for (path, client) in &self.config_paths {
            if !path.exists() {
                continue;
            }

            match Self::load_config_from_path(path) {
                Ok(config) => {
                    if let Err(validation_error) = Self::validate_config(&config) {
                        warn!(
                            "Invalid MCP configuration in {} ({}): {}",
                            client.name(),
                            path.display(),
                            validation_error
                        );
                        continue;
                    }

                    configs_by_ide.push((client.display_name().to_string(), config));
                    debug!(
                        "Loaded MCP configuration from {}: {}",
                        client.name(),
                        path.display()
                    );
                }
                Err(e) => {
                    debug!(
                        "Failed to load MCP configuration from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        configs_by_ide
    }

    /// Load and merge all configurations
    pub fn load_config(&self) -> MCPConfig {
        let mut merged_config = MCPConfig::default();
        let mut loaded_configs = 0;

        for (path, client) in &self.config_paths {
            if !path.exists() {
                continue;
            }

            match Self::load_config_from_path(path) {
                Ok(config) => {
                    if let Err(validation_error) = Self::validate_config(&config) {
                        warn!(
                            "Invalid MCP configuration in {} ({}): {}",
                            client.name(),
                            path.display(),
                            validation_error
                        );
                        continue;
                    }

                    Self::merge_config(&mut merged_config, &config);
                    loaded_configs += 1;
                    debug!(
                        "Loaded MCP configuration from {}: {}",
                        client.name(),
                        path.display()
                    );
                }
                Err(e) => {
                    debug!(
                        "Failed to load MCP configuration from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        if loaded_configs == 0 {
            debug!("No MCP configuration files found in any supported IDE locations");
        }

        merged_config
    }

    /// Load configuration from a specific path
    pub fn load_config_from_path(path: &Path) -> Result<MCPConfig> {
        if !path.exists() {
            return Err(anyhow!(
                "Configuration file does not exist: {}",
                path.display()
            ));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file {}: {}", path.display(), e))?;

        let client = Self::detect_client(path);
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Try parsing based on detected client and filename
        match (client, filename) {
            (Some(MCPClient::VSCode), "settings.json") => {
                if let Ok(config) = serde_json::from_str::<VSCodeSettings>(&content) {
                    debug!("Parsed as VS Code settings format");
                    return config.to_mcp_config();
                }
            }
            (Some(MCPClient::ClaudeCode), ".claude.json") => {
                // Try Claude Code specific format first
                if let Ok(config) =
                    serde_json::from_str::<IDEConfigFormat<UnifiedServerConfig>>(&content)
                {
                    debug!("Parsed as Claude Code configuration format");
                    return config.to_mcp_config();
                }
            }
            (Some(MCPClient::Claude), "claude_desktop_config.json") => {
                if let Ok(config) =
                    serde_json::from_str::<IDEConfigFormat<UnifiedServerConfig>>(&content)
                {
                    debug!("Parsed as Claude Desktop configuration format");
                    return config.to_mcp_config();
                }
            }
            _ => {}
        }

        // Try generic IDE format
        if let Ok(config) = serde_json::from_str::<IDEConfigFormat<UnifiedServerConfig>>(&content) {
            debug!("Parsed as generic IDE configuration format");
            return config.to_mcp_config();
        }

        // Try standard MCP format
        if let Ok(config) = serde_json::from_str::<MCPConfig>(&content) {
            debug!("Parsed as standard MCP configuration format");
            return Ok(config);
        }

        Err(anyhow!(
            "Failed to parse configuration file: {}",
            path.display()
        ))
    }

    /// Validate configuration
    fn validate_config(config: &MCPConfig) -> Result<()> {
        if let Some(servers) = &config.servers {
            for server in servers {
                if server.name.is_none() || server.name.as_ref().unwrap().is_empty() {
                    return Err(anyhow!("Server name cannot be empty"));
                }

                if server.url.is_none() && server.command.is_none() {
                    return Err(anyhow!("Server must have either URL or command"));
                }
            }
        }
        Ok(())
    }

    /// Merge two configurations
    fn merge_config(base: &mut MCPConfig, other: &MCPConfig) {
        // Merge servers
        let mut server_map: HashMap<String, MCPServerConfig> = HashMap::new();

        // Add existing servers
        if let Some(base_servers) = &base.servers {
            for server in base_servers {
                if let Some(name) = &server.name {
                    server_map.insert(name.clone(), server.clone());
                }
            }
        }

        // Add new servers (overwrites duplicates)
        if let Some(other_servers) = &other.servers {
            for server in other_servers {
                if let Some(name) = &server.name {
                    server_map.insert(name.clone(), server.clone());
                }
            }
        }

        // Convert back to vector
        base.servers = if server_map.is_empty() {
            None
        } else {
            Some(server_map.into_values().collect())
        };

        // Merge options (other takes precedence)
        if other.options.is_some() {
            base.options = other.options.clone();
        }

        // Merge auth headers
        if let Some(other_headers) = &other.auth_headers {
            if let Some(base_headers) = &mut base.auth_headers {
                base_headers.extend(other_headers.clone());
            } else {
                base.auth_headers = Some(other_headers.clone());
            }
        }
    }
}

/// Scanner-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScannerConfig {
    pub scanner: ScannerSettings,
    pub security: SecuritySettings,
    pub llm: LlmSettings,
    pub logging: LoggingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerSettings {
    #[serde(default = "default_http_timeout")]
    pub http_timeout: u64,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub detailed: bool,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
    #[serde(default = "default_scan_timeout")]
    pub scan_timeout: u64,
    #[serde(default = "default_llm_batch_size")]
    pub llm_batch_size: u32,
}

impl Default for ScannerSettings {
    fn default() -> Self {
        Self {
            http_timeout: default_http_timeout(),
            format: default_format(),
            detailed: false,
            max_connections: default_max_connections(),
            retry_attempts: default_retry_attempts(),
            retry_delay: default_retry_delay(),
            scan_timeout: default_scan_timeout(),
            llm_batch_size: default_llm_batch_size(),
        }
    }
}

fn default_http_timeout() -> u64 {
    30
}
fn default_format() -> String {
    "table".to_string()
}
fn default_max_connections() -> u32 {
    10
}
fn default_retry_attempts() -> u32 {
    3
}
fn default_retry_delay() -> u64 {
    1000
}
fn default_scan_timeout() -> u64 {
    300
}
fn default_llm_batch_size() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecuritySettings {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_timeout")]
    pub timeout: u64,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: default_base_url(),
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            timeout: default_llm_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}
fn default_model() -> String {
    "gpt-3.5-turbo".to_string()
}
fn default_temperature() -> f32 {
    0.1
}
fn default_max_tokens() -> u32 {
    4000
}
fn default_llm_timeout() -> u64 {
    30
}
fn default_log_level() -> String {
    "info".to_string()
}

/// Scanner configuration manager
pub struct ScannerConfigManager {
    config_path: PathBuf,
}

impl Default for ScannerConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScannerConfigManager {
    pub fn new() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ramparts")
            .join("config.json");

        Self { config_path }
    }

    pub fn load_config(&self) -> Result<ScannerConfig> {
        if !self.config_path.exists() {
            return Ok(ScannerConfig::default());
        }

        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| anyhow!("Failed to read scanner config: {}", e))?;

        serde_json::from_str(&content).map_err(|e| anyhow!("Failed to parse scanner config: {}", e))
    }

    pub fn save_config(&self, config: &ScannerConfig) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| anyhow!("Failed to serialize scanner config: {}", e))?;

        fs::write(&self.config_path, content)
            .map_err(|e| anyhow!("Failed to write scanner config: {}", e))
    }

    pub fn has_config_file(&self) -> bool {
        self.config_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_unified_server_config_conversion() {
        // Test STDIO server (command-based)
        let stdio_config = UnifiedServerConfig {
            command: Some("python".to_string()),
            args: Some(vec!["-m".to_string(), "server".to_string()]),
            env: None,
            url: None,
            description: Some("Test STDIO server".to_string()),
            disabled: Some(false),
            transport: None,
        };
        let mcp_config = convert_unified_server_config("stdio-test".to_string(), stdio_config);
        assert_eq!(mcp_config.name, Some("stdio-test".to_string()));
        assert_eq!(mcp_config.command, Some("python".to_string()));
        assert_eq!(mcp_config.url, None); // STDIO server should have no URL

        // Test HTTP server (URL-based)
        let http_config = UnifiedServerConfig {
            command: None,
            args: None,
            env: None,
            url: Some("http://localhost:8080".to_string()),
            description: Some("Test HTTP server".to_string()),
            disabled: Some(false),
            transport: None,
        };
        let mcp_config = convert_unified_server_config("http-test".to_string(), http_config);
        assert_eq!(mcp_config.name, Some("http-test".to_string()));
        assert_eq!(mcp_config.url, Some("http://localhost:8080".to_string()));
        assert_eq!(mcp_config.command, None);

        // Test transport-based server
        let transport_config = UnifiedServerConfig {
            command: None,
            args: None,
            env: None,
            url: None,
            description: None,
            disabled: Some(false),
            transport: Some(TransportConfig {
                transport_type: Some("http".to_string()),
                host: Some("example.com".to_string()),
                port: Some(9000),
            }),
        };
        let mcp_config =
            convert_unified_server_config("transport-test".to_string(), transport_config);
        assert_eq!(mcp_config.url, Some("http://example.com:9000".to_string()));
    }

    #[test]
    fn test_url_normalization() {
        assert_eq!(normalize_url("localhost:8080"), "http://localhost:8080");
        assert_eq!(
            normalize_url("http://localhost:8080"),
            "http://localhost:8080"
        );
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
        assert_eq!(normalize_url("127.0.0.1:3000"), "http://127.0.0.1:3000");
        assert_eq!(normalize_url("example.com:5000"), "http://example.com:5000");
    }

    #[test]
    fn test_client_detection() {
        // Test specific filenames
        assert_eq!(
            MCPConfigManager::detect_client("/home/user/.claude.json"),
            Some(MCPClient::ClaudeCode)
        );
        assert_eq!(
            MCPConfigManager::detect_client("/home/user/claude_desktop_config.json"),
            Some(MCPClient::Claude)
        );

        // Test path components
        assert_eq!(
            MCPConfigManager::detect_client("/home/user/.vscode/settings.json"),
            Some(MCPClient::VSCode)
        );
        assert_eq!(
            MCPConfigManager::detect_client("/home/user/.cursor/mcp.json"),
            Some(MCPClient::Cursor)
        );
        assert_eq!(
            MCPConfigManager::detect_client("/home/user/.windsurf/mcp.json"),
            Some(MCPClient::Windsurf)
        );

        // Test unknown paths
        assert_eq!(
            MCPConfigManager::detect_client("/home/user/random.json"),
            None
        );
    }

    #[test]
    fn test_load_config_from_path() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        // Test IDE format (servers object)
        let config_content = r#"{
            "servers": {
                "test-server": {
                    "url": "http://localhost:3000",
                    "description": "Test server"
                },
                "stdio-server": {
                    "command": "python",
                    "args": ["-m", "server"]
                }
            }
        }"#;

        fs::write(&config_path, config_content).unwrap();
        let config = MCPConfigManager::load_config_from_path(&config_path).unwrap();
        assert!(config.servers.is_some());
        assert_eq!(config.servers.unwrap().len(), 2);

        // Test mcpServers format
        let mcp_config_content = r#"{
            "mcpServers": {
                "test-server-2": {
                    "url": "http://localhost:4000"
                }
            }
        }"#;

        fs::write(&config_path, mcp_config_content).unwrap();
        let config = MCPConfigManager::load_config_from_path(&config_path).unwrap();
        assert!(config.servers.is_some());
        assert_eq!(config.servers.unwrap().len(), 1);
    }

    #[test]
    fn test_merge_config() {
        let mut base = MCPConfig::default();
        let other = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("server1".to_string()),
                url: Some("http://localhost:3000".to_string()),
                command: None,
                args: None,
                env: None,
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };

        MCPConfigManager::merge_config(&mut base, &other);
        assert!(base.servers.is_some());
        assert_eq!(base.servers.unwrap().len(), 1);
    }

    #[test]
    fn test_config_validation() {
        // Valid config
        let valid_config = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("valid-server".to_string()),
                url: Some("http://localhost:3000".to_string()),
                command: None,
                args: None,
                env: None,
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&valid_config).is_ok());

        // Invalid config - empty server name
        let invalid_config = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("".to_string()),
                url: Some("http://localhost:3000".to_string()),
                command: None,
                args: None,
                env: None,
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&invalid_config).is_err());

        // Invalid config - no URL or command
        let invalid_config2 = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("invalid-server".to_string()),
                url: None,
                command: None,
                args: None,
                env: None,
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&invalid_config2).is_err());
    }

    #[test]
    fn test_mcp_server_config_methods() {
        // Test STDIO server display URL
        let stdio_server = MCPServerConfig {
            name: Some("stdio-test".to_string()),
            url: None,
            command: Some("python".to_string()),
            args: Some(vec!["-m".to_string(), "server".to_string()]),
            env: None,
            description: None,
            auth_headers: None,
            options: None,
        };
        assert_eq!(stdio_server.to_display_url(), "stdio://python");
        assert!(stdio_server.scan_url().is_none());

        // Test HTTP server
        let http_server = MCPServerConfig {
            name: Some("http-test".to_string()),
            url: Some("http://localhost:8080".to_string()),
            command: None,
            args: None,
            env: None,
            description: None,
            auth_headers: None,
            options: None,
        };
        assert_eq!(http_server.to_display_url(), "http://localhost:8080");
        assert_eq!(
            http_server.scan_url(),
            Some("http://localhost:8080".to_string())
        );
    }
}
