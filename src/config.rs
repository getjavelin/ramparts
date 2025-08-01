use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::{debug, info, warn};
use url::Url;

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

/// Cursor-specific MCP Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorMCPConfig {
    /// List of MCP server configurations using Cursor's format
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<HashMap<String, CursorMCPServerConfig>>,
    /// Cursor's settings (if any)
    pub settings: Option<HashMap<String, serde_json::Value>>,
}

/// Cursor-specific MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorMCPServerConfig {
    /// Server command
    pub command: Option<String>,
    /// Server arguments
    pub args: Option<Vec<String>>,
    /// Working directory
    pub cwd: Option<String>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Transport configuration
    pub transport: Option<CursorTransportConfig>,
    /// Server description
    pub description: Option<String>,
    /// Available tools
    pub tools: Option<Vec<String>>,
    /// Server URL (for HTTP transport)
    pub url: Option<String>,
}

/// Cursor transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorTransportConfig {
    /// Transport type
    #[serde(rename = "type")]
    pub transport_type: Option<String>,
    /// Host for HTTP transport
    pub host: Option<String>,
    /// Port for HTTP transport
    pub port: Option<u16>,
}

/// Claude Desktop configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeDesktopConfig {
    /// List of MCP server configurations using Claude Desktop's format
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<HashMap<String, ClaudeDesktopServerConfig>>,
    /// Global settings
    pub globalShortcut: Option<String>,
    /// Other settings
    #[serde(flatten)]
    pub other_settings: Option<HashMap<String, serde_json::Value>>,
}

/// Claude Desktop MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeDesktopServerConfig {
    /// Server command
    pub command: Option<String>,
    /// Server arguments
    pub args: Option<Vec<String>>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Working directory
    pub cwd: Option<String>,
    /// Disabled flag
    pub disabled: Option<bool>,
    /// Server URL (for HTTP servers)
    pub url: Option<String>,
}

/// VS Code settings structure (can contain MCP configuration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeSettings {
    /// MCP servers configuration in VS Code settings
    #[serde(rename = "mcp.servers")]
    pub mcp_servers: Option<HashMap<String, VSCodeMCPServerConfig>>,
    /// Other VS Code settings
    #[serde(flatten)]
    pub other_settings: Option<HashMap<String, serde_json::Value>>,
}

/// VS Code MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeMCPServerConfig {
    /// Server command
    pub command: Option<String>,
    /// Server arguments
    pub args: Option<Vec<String>>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Working directory
    pub cwd: Option<String>,
    /// Server URL
    pub url: Option<String>,
    /// Transport type (e.g., "http", "stdio")
    #[serde(rename = "type")]
    pub transport_type: Option<String>,
}

/// New VS Code MCP configuration structure (for mcp.json files)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeMCPConfig {
    /// MCP servers configuration in new VS Code format
    pub servers: Option<HashMap<String, VSCodeMCPServerConfig>>,
    /// Inputs configuration
    pub inputs: Option<Vec<serde_json::Value>>,
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

/// Supported MCP client types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MCPClient {
    Cursor,
    Windsurf,
    VSCode,
    Claude,
    Neovim,
    Helix,
    Zed,
}

impl MCPClient {
    pub fn name(&self) -> &'static str {
        match self {
            MCPClient::Cursor => "cursor",
            MCPClient::Windsurf => "windsurf",
            MCPClient::VSCode => "vscode",
            MCPClient::Claude => "claude",
            MCPClient::Neovim => "neovim",
            MCPClient::Helix => "helix",
            MCPClient::Zed => "zed",
        }
    }
}

/// Cache for discovered configuration paths to avoid repeated filesystem operations
static CONFIG_PATHS_CACHE: LazyLock<Vec<(PathBuf, MCPClient)>> =
    LazyLock::new(MCPConfigManager::discover_config_paths);

/// IDE configuration file manager for MCP scanner
pub struct MCPConfigManager {
    config_paths: Vec<(PathBuf, MCPClient)>,
}

impl MCPConfigManager {
    /// Normalize URL for consistent deduplication
    /// Removes trailing slashes, converts to lowercase, and handles localhost aliases
    fn normalize_url(url: &str) -> String {
        let mut normalized = url.trim().to_lowercase();

        // Remove trailing slashes
        if normalized.ends_with('/') && normalized != "http://" && normalized != "https://" {
            normalized = normalized.trim_end_matches('/').to_string();
        }

        // Normalize localhost variants
        normalized = normalized
            .replace("127.0.0.1", "localhost")
            .replace("0.0.0.0", "localhost");

        // Normalize port defaults - currently no-op, but placeholder for future enhancement
        if (normalized.starts_with("http://localhost")
            || normalized.starts_with("https://localhost"))
            && !normalized.contains(':')
        {
            // Don't add default port, keep as-is for now
        }

        normalized
    }
    /// Create a new configuration manager with platform-specific IDE config paths
    pub fn new() -> Self {
        Self {
            config_paths: CONFIG_PATHS_CACHE.clone(),
        }
    }

    /// Create a new configuration manager with fresh path discovery (bypasses cache)
    #[allow(dead_code)]
    pub fn new_uncached() -> Self {
        let paths = Self::discover_config_paths();
        Self {
            config_paths: paths,
        }
    }

    /// Discover MCP configuration paths based on platform and IDE
    fn discover_config_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();

        // First, add workspace-level configurations (highest priority)
        paths.extend(Self::get_workspace_paths());

        // Then add platform-specific global configurations
        let platform = env::consts::OS;

        match platform {
            "windows" => {
                paths.extend(Self::get_windows_paths());
            }
            "macos" | "darwin" => {
                paths.extend(Self::get_macos_paths());
            }
            _ => {
                // Linux and other Unix-like systems
                paths.extend(Self::get_unix_paths());
            }
        }

        paths
    }

    /// Get workspace-level MCP configuration paths (current working directory and workspace)
    fn get_workspace_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();
        
        // Current working directory workspace configurations
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        
        // VS Code workspace configurations
        paths.push((current_dir.join(".vscode").join("mcp.json"), MCPClient::VSCode));
        paths.push((current_dir.join(".vscode").join("settings.json"), MCPClient::VSCode));
        
        // Cursor workspace configurations  
        paths.push((current_dir.join(".cursor").join("mcp.json"), MCPClient::Cursor));
        paths.push((current_dir.join(".cursor").join("settings.json"), MCPClient::Cursor));
        
        // Claude Code workspace configurations
        paths.push((current_dir.join(".claude.json"), MCPClient::Claude));
        paths.push((current_dir.join(".claude").join("mcp.json"), MCPClient::Claude));
        
        // Windsurf workspace configurations  
        paths.push((current_dir.join(".windsurf").join("mcp.json"), MCPClient::Windsurf));
        paths.push((current_dir.join(".windsurf").join("mcp_config.json"), MCPClient::Windsurf));
        
        // Also check parent directories up to 3 levels for project root configurations
        let mut parent = current_dir.parent();
        let mut level = 0;
        while let Some(dir) = parent {
            if level >= 3 { break; }
            
            // Look for common project indicators
            if dir.join(".git").exists() || 
               dir.join("package.json").exists() || 
               dir.join("Cargo.toml").exists() ||
               dir.join("pyproject.toml").exists() ||
               dir.join("requirements.txt").exists() {
                
                // VS Code project root configurations
                paths.push((dir.join(".vscode").join("mcp.json"), MCPClient::VSCode));
                paths.push((dir.join(".vscode").join("settings.json"), MCPClient::VSCode));
                
                // Cursor project root configurations
                paths.push((dir.join(".cursor").join("mcp.json"), MCPClient::Cursor));
                
                // Claude Code project root configurations
                paths.push((dir.join(".claude.json"), MCPClient::Claude));
                paths.push((dir.join(".claude").join("mcp.json"), MCPClient::Claude));
                
                // Windsurf project root configurations
                paths.push((dir.join(".windsurf").join("mcp.json"), MCPClient::Windsurf));
                paths.push((dir.join(".windsurf").join("mcp_config.json"), MCPClient::Windsurf));
                
                break; // Stop at first project root found
            }
            
            parent = dir.parent();
            level += 1;
        }
        
        paths
    }

    /// Get Windows-specific MCP configuration paths
    fn get_windows_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();

        // Try APPDATA first, then fallback to home directory
        if let Ok(appdata) = env::var("APPDATA") {
            let appdata_path = PathBuf::from(appdata);

            // Cursor
            paths.push((
                appdata_path
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("rooveterinaryinc.cursor-mcp")
                    .join("mcp.json"),
                MCPClient::Cursor,
            ));
            paths.push((
                appdata_path.join("Cursor").join("User").join("mcp.json"),
                MCPClient::Cursor,
            ));

            // Windsurf
            paths.push((
                appdata_path.join("Windsurf").join("User").join("mcp.json"),
                MCPClient::Windsurf,
            ));
            paths.push((
                appdata_path
                    .join("Codeium")
                    .join("Windsurf")
                    .join("mcp_config.json"),
                MCPClient::Windsurf,
            ));

            // VS Code
            paths.push((
                appdata_path.join("Code").join("User").join("mcp.json"),
                MCPClient::VSCode,
            ));

            // Claude Desktop
            paths.push((
                appdata_path.join("Claude").join("mcp.json"),
                MCPClient::Claude,
            ));
        } else {
            // Fallback: try LOCALAPPDATA if APPDATA is missing
            if let Ok(localappdata) = env::var("LOCALAPPDATA") {
                let localappdata_path = PathBuf::from(localappdata);

                paths.push((
                    localappdata_path
                        .join("Cursor")
                        .join("User")
                        .join("mcp.json"),
                    MCPClient::Cursor,
                ));
                paths.push((
                    localappdata_path
                        .join("Programs")
                        .join("Windsurf")
                        .join("mcp.json"),
                    MCPClient::Windsurf,
                ));
                paths.push((
                    localappdata_path
                        .join("Programs")
                        .join("Microsoft VS Code")
                        .join("mcp.json"),
                    MCPClient::VSCode,
                ));
            }
        }

        // User home directory configs (Unix-style on Windows)
        if let Some(home_dir) = dirs::home_dir() {
            paths.push((home_dir.join(".cursor").join("mcp.json"), MCPClient::Cursor));
            paths.push((home_dir.join(".vscode").join("mcp.json"), MCPClient::VSCode));
            paths.push((home_dir.join(".claude").join("mcp.json"), MCPClient::Claude));

            // Windows-specific AppData fallback in user profile
            let user_appdata = home_dir.join("AppData").join("Roaming");
            if user_appdata.exists() {
                // Cursor
                paths.push((
                    user_appdata.join("Cursor").join("User").join("mcp.json"),
                    MCPClient::Cursor,
                ));
                
                // VS Code
                paths.push((
                    user_appdata.join("Code").join("User").join("mcp.json"),
                    MCPClient::VSCode,
                ));
                paths.push((
                    user_appdata.join("Code").join("User").join("settings.json"),
                    MCPClient::VSCode,
                ));
                
                // Claude Desktop
                paths.push((
                    user_appdata.join("Claude").join("claude_desktop_config.json"),
                    MCPClient::Claude,
                ));
                
                // Windsurf
                paths.push((
                    user_appdata.join("Windsurf").join("User").join("mcp.json"),
                    MCPClient::Windsurf,
                ));
                paths.push((
                    user_appdata.join("Codeium").join("Windsurf").join("mcp_config.json"),
                    MCPClient::Windsurf,
                ));
            }
        }

        paths
    }

    /// Get macOS-specific MCP configuration paths
    fn get_macos_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();

        if let Some(home_dir) = dirs::home_dir() {
            let app_support = home_dir.join("Library").join("Application Support");

            // Cursor
            paths.push((
                app_support
                    .join("Cursor")
                    .join("User")
                    .join("globalStorage")
                    .join("rooveterinaryinc.cursor-mcp")
                    .join("mcp.json"),
                MCPClient::Cursor,
            ));
            paths.push((
                app_support.join("Cursor").join("User").join("mcp.json"),
                MCPClient::Cursor,
            ));
            paths.push((home_dir.join(".cursor").join("mcp.json"), MCPClient::Cursor));

            // Windsurf
            paths.push((
                app_support.join("Windsurf").join("User").join("mcp.json"),
                MCPClient::Windsurf,
            ));
            paths.push((
                app_support
                    .join("Codeium")
                    .join("Windsurf")
                    .join("mcp_config.json"),
                MCPClient::Windsurf,
            ));
            paths.push((
                home_dir
                    .join(".codium")
                    .join("windsurf")
                    .join("mcp_config.json"),
                MCPClient::Windsurf,
            ));

            // VS Code - multiple configuration locations
            paths.push((
                app_support.join("Code").join("User").join("mcp.json"),
                MCPClient::VSCode,
            ));
            paths.push((
                app_support.join("Code").join("User").join("settings.json"),
                MCPClient::VSCode,
            ));
            paths.push((home_dir.join(".vscode").join("mcp.json"), MCPClient::VSCode));
            paths.push((home_dir.join(".vscode").join("settings.json"), MCPClient::VSCode));

            // Claude Desktop - uses claude_desktop_config.json
            paths.push((
                app_support.join("Claude").join("claude_desktop_config.json"),
                MCPClient::Claude,
            ));
            paths.push((
                app_support.join("Claude").join("User").join("claude_desktop_config.json"),
                MCPClient::Claude,
            ));
            
            // Claude Code - multiple scopes and file formats
            paths.push((home_dir.join(".claude.json"), MCPClient::Claude)); // User/Global scope
            paths.push((home_dir.join(".claude").join("mcp.json"), MCPClient::Claude));

            // Zed
            paths.push((app_support.join("Zed").join("mcp.json"), MCPClient::Zed));

            // Unix-style configs in home directory
            let config_dir = home_dir.join(".config");
            paths.push((config_dir.join("nvim").join("mcp.json"), MCPClient::Neovim));
            paths.push((config_dir.join("helix").join("mcp.json"), MCPClient::Helix));
        }

        paths
    }

    /// Get Unix/Linux-specific MCP configuration paths
    fn get_unix_paths() -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();

        if let Some(home_dir) = dirs::home_dir() {
            let config_dir = home_dir.join(".config");

            // Cursor
            paths.push((home_dir.join(".cursor").join("mcp.json"), MCPClient::Cursor));
            paths.push((
                config_dir.join("Cursor").join("User").join("mcp.json"),
                MCPClient::Cursor,
            ));

            // Windsurf
            paths.push((
                home_dir
                    .join(".codium")
                    .join("windsurf")
                    .join("mcp_config.json"),
                MCPClient::Windsurf,
            ));
            paths.push((
                config_dir.join("Windsurf").join("User").join("mcp.json"),
                MCPClient::Windsurf,
            ));

            // VS Code
            paths.push((home_dir.join(".vscode").join("mcp.json"), MCPClient::VSCode));
            paths.push((home_dir.join(".vscode").join("settings.json"), MCPClient::VSCode));
            paths.push((
                config_dir.join("Code").join("User").join("mcp.json"),
                MCPClient::VSCode,
            ));
            paths.push((
                config_dir.join("Code").join("User").join("settings.json"),
                MCPClient::VSCode,
            ));

            // Claude Desktop - uses claude_desktop_config.json
            paths.push((home_dir.join(".claude").join("claude_desktop_config.json"), MCPClient::Claude));
            paths.push((
                config_dir.join("claude").join("claude_desktop_config.json"),
                MCPClient::Claude,
            ));
            
            // Claude Code - multiple scopes and formats
            paths.push((home_dir.join(".claude.json"), MCPClient::Claude)); // User/Global scope
            paths.push((home_dir.join(".claude").join("mcp.json"), MCPClient::Claude));

            // Neovim
            paths.push((config_dir.join("nvim").join("mcp.json"), MCPClient::Neovim));

            // Helix
            paths.push((config_dir.join("helix").join("mcp.json"), MCPClient::Helix));

            // Zed
            paths.push((config_dir.join("zed").join("mcp.json"), MCPClient::Zed));
        }

        paths
    }

    /// Get client type from a configuration file path using component-based matching
    #[allow(dead_code)]
    pub fn get_client_from_path<P: AsRef<Path>>(path: P) -> Option<MCPClient> {
        let path = path.as_ref();
        let components: Vec<_> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .map(str::to_lowercase)
            .collect();

        // Check path components in order of specificity to avoid false matches
        for component in &components {
            match component.as_str() {
                // Exact matches first
                "cursor" | ".cursor" => return Some(MCPClient::Cursor),
                "windsurf" => return Some(MCPClient::Windsurf),
                "claude" | ".claude" => return Some(MCPClient::Claude),
                "zed" => return Some(MCPClient::Zed),
                "helix" => return Some(MCPClient::Helix),
                "nvim" | "neovim" => return Some(MCPClient::Neovim),
                "code" | "vscode" | ".vscode" => return Some(MCPClient::VSCode),

                // Partial matches with disambiguation
                c if c.contains("cursor") && !c.contains("vscode") => {
                    return Some(MCPClient::Cursor)
                }
                c if c.contains("windsurf") => return Some(MCPClient::Windsurf),
                c if c.contains("codium") => return Some(MCPClient::Windsurf), // Codium usually means Windsurf context
                c if c.contains("microsoft") && c.contains("code") => {
                    return Some(MCPClient::VSCode)
                }

                _ => {} // Keep looking
            }
        }

        // Check specific file names for client detection
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            match filename {
                "claude_desktop_config.json" => return Some(MCPClient::Claude),
                ".claude.json" => return Some(MCPClient::Claude),
                "settings.json" => {
                    // Check if it's in a VS Code directory
                    if components.iter().any(|c| c.contains("code") || c.contains("vscode")) {
                        return Some(MCPClient::VSCode);
                    }
                }
                _ => {}
            }
        }

        // Fallback: check full path string for edge cases
        let path_str = path.to_string_lossy().to_lowercase();
        if path_str.contains("rooveterinaryinc.cursor-mcp") {
            return Some(MCPClient::Cursor);
        }

        None
    }

    /// Convert client shorthand names to full configuration paths
    #[allow(dead_code)]
    pub fn client_shorthands_to_paths(clients: &[&str]) -> Vec<(PathBuf, MCPClient)> {
        let mut paths = Vec::new();
        let all_paths = &*CONFIG_PATHS_CACHE;

        for client_name in clients {
            let client_type = match client_name.to_lowercase().as_str() {
                "cursor" => Some(MCPClient::Cursor),
                "windsurf" => Some(MCPClient::Windsurf),
                "vscode" | "code" => Some(MCPClient::VSCode),
                "claude" => Some(MCPClient::Claude),
                "neovim" | "nvim" => Some(MCPClient::Neovim),
                "helix" => Some(MCPClient::Helix),
                "zed" => Some(MCPClient::Zed),
                _ => None,
            };

            if let Some(client) = client_type {
                paths.extend(all_paths.iter().filter(|(_, c)| *c == client).cloned());
            }
        }

        paths
    }

    /// Get all discovered configuration file paths
    #[allow(dead_code)]
    pub fn get_all_config_paths() -> Vec<(PathBuf, MCPClient)> {
        CONFIG_PATHS_CACHE.clone()
    }

    /// Load configuration from all available IDE config files
    pub fn load_config(&self) -> MCPConfig {
        let mut merged_config = MCPConfig::default();
        let mut loaded_configs = 0;
        let mut failed_configs = Vec::new();

        for (path, client) in &self.config_paths {
            match Self::load_config_from_path(path) {
                Ok(config) => {
                    // Validate configuration before merging
                    if let Err(validation_error) = Self::validate_config(&config) {
                        warn!(
                            "Invalid MCP configuration in {} ({}): {}",
                            client.name(),
                            path.display(),
                            validation_error
                        );
                        failed_configs.push((path.clone(), validation_error));
                        continue;
                    }

                    Self::merge_config(&mut merged_config, &config);
                    loaded_configs += 1;
                    info!(
                        "Loaded MCP configuration from {} IDE: {}",
                        client.name(),
                        path.display()
                    );
                }
                Err(e) => {
                    if path.exists() {
                        // File exists but couldn't be parsed - this is an error
                        warn!(
                            "Failed to parse MCP configuration from {} IDE at {}: {}",
                            client.name(),
                            path.display(),
                            e
                        );
                        failed_configs.push((path.clone(), e));
                    } else {
                        // File doesn't exist - this is normal
                        debug!(
                            "No MCP configuration found for {} IDE at: {}",
                            client.name(),
                            path.display()
                        );
                    }
                }
            }
        }

        if loaded_configs == 0 && failed_configs.is_empty() {
            info!("No MCP configuration files found in any supported IDE locations");
        } else if !failed_configs.is_empty() {
            warn!(
                "Found {} configuration files with errors",
                failed_configs.len()
            );
        }

        merged_config
    }

    /// Load configuration from a specific IDE config path
    pub fn load_config_from_path(path: &Path) -> Result<MCPConfig> {
        if !path.exists() {
            return Err(anyhow!(
                "IDE configuration file does not exist: {}",
                path.display()
            ));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read IDE config file {}: {}", path.display(), e))?;

        // Detect IDE type by checking the client type from path
        let client = Self::get_client_from_path(path);
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        // Try parsing based on client type and file name
        match client {
            Some(MCPClient::Cursor) => {
                if let Ok(cursor_config) = serde_json::from_str::<CursorMCPConfig>(&content) {
                    debug!("Parsed as Cursor MCP configuration format");
                    return Ok(Self::convert_cursor_config(cursor_config));
                }
            }
            Some(MCPClient::Claude) => {
                // Claude Desktop uses claude_desktop_config.json
                if filename == "claude_desktop_config.json" {
                    if let Ok(claude_config) = serde_json::from_str::<ClaudeDesktopConfig>(&content) {
                        debug!("Parsed as Claude Desktop configuration format");
                        return Ok(Self::convert_claude_desktop_config(claude_config));
                    }
                }
                // Claude Code uses .claude.json
                if filename == ".claude.json" {
                    if let Ok(cursor_config) = serde_json::from_str::<CursorMCPConfig>(&content) {
                        debug!("Parsed as Claude Code configuration format");
                        return Ok(Self::convert_cursor_config(cursor_config));
                    }
                }
            }
            Some(MCPClient::VSCode) => {
                // VS Code settings.json may contain MCP configuration
                if filename == "settings.json" {
                    if let Ok(vscode_config) = serde_json::from_str::<VSCodeSettings>(&content) {
                        debug!("Parsed as VS Code settings configuration format");
                        return Ok(Self::convert_vscode_config(vscode_config));
                    }
                }
                // VS Code mcp.json uses the new format
                else if filename == "mcp.json" {
                    if let Ok(vscode_mcp_config) = serde_json::from_str::<VSCodeMCPConfig>(&content) {
                        debug!("Parsed as VS Code MCP configuration format");
                        return Ok(Self::convert_vscode_mcp_config(vscode_mcp_config));
                    }
                }
            }
            _ => {}
        }

        // Try parsing as standard format
        match serde_json::from_str::<MCPConfig>(&content) {
            Ok(config) => Ok(config),
            Err(e) => {
                // Try fallback parsing for different formats
                if let Ok(cursor_config) = serde_json::from_str::<CursorMCPConfig>(&content) {
                    debug!("Parsed as Cursor MCP configuration format (fallback)");
                    Ok(Self::convert_cursor_config(cursor_config))
                } else if let Ok(claude_config) = serde_json::from_str::<ClaudeDesktopConfig>(&content) {
                    debug!("Parsed as Claude Desktop configuration format (fallback)");
                    Ok(Self::convert_claude_desktop_config(claude_config))
                } else if let Ok(vscode_config) = serde_json::from_str::<VSCodeSettings>(&content) {
                    debug!("Parsed as VS Code settings configuration format (fallback)");
                    Ok(Self::convert_vscode_config(vscode_config))
                } else if let Ok(vscode_mcp_config) = serde_json::from_str::<VSCodeMCPConfig>(&content) {
                    debug!("Parsed as VS Code MCP configuration format (fallback)");
                    Ok(Self::convert_vscode_mcp_config(vscode_mcp_config))
                } else {
                    Err(anyhow!("Failed to parse IDE config file {}: {}", path.display(), e))
                }
            }
        }
    }

    /// Convert Cursor MCP configuration to standard format
    fn convert_cursor_config(cursor_config: CursorMCPConfig) -> MCPConfig {
        let servers = if let Some(mcp_servers) = cursor_config.mcp_servers {
            Some(
                mcp_servers
                    .into_iter()
                    .map(|(name, server_config)| {
                        // Use explicit URL first, then build from transport config
                        let url = if let Some(url) = server_config.url {
                            url
                        } else if let Some(transport) = &server_config.transport {
                            let host = transport.host.as_deref().unwrap_or("localhost");
                            let port = transport.port.unwrap_or(8080);
                            let scheme = match transport.transport_type.as_deref() {
                                Some("http") | Some("streamable-http") => "http",
                                Some("https") => "https",
                                _ => "http",
                            };
                            format!("{}://{}:{}", scheme, host, port)
                        } else {
                            // Default URL for servers without transport config
                            "http://localhost:8123".to_string()
                        };

                        MCPServerConfig {
                            name: Some(name),
                            url,
                            description: server_config.description,
                            auth_headers: None, // Cursor format doesn't specify auth headers at server level
                            options: None, // Could be extended to convert any server-specific options
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        MCPConfig {
            servers,
            options: None, // Could convert cursor settings to options if needed
            auth_headers: None,
        }
    }

    /// Convert Claude Desktop configuration to standard format
    fn convert_claude_desktop_config(claude_config: ClaudeDesktopConfig) -> MCPConfig {
        let servers = if let Some(mcp_servers) = claude_config.mcp_servers {
            Some(
                mcp_servers
                    .into_iter()
                    .filter_map(|(name, server_config)| {
                        // Skip disabled servers
                        if server_config.disabled.unwrap_or(false) {
                            return None;
                        }

                        // Use explicit URL if provided, otherwise build from command
                        let url = if let Some(url) = server_config.url {
                            url
                        } else if server_config.command.is_some() {
                            // For command-based servers, create a placeholder URL
                            // This represents a local server that will be started by the command
                            format!("stdio://{}", name)
                        } else {
                            // Default URL for servers without explicit configuration
                            "http://localhost:8123".to_string()
                        };

                        Some(MCPServerConfig {
                            name: Some(name),
                            url,
                            description: None, // Claude Desktop format doesn't include descriptions
                            auth_headers: None,
                            options: None,
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        MCPConfig {
            servers,
            options: None,
            auth_headers: None,
        }
    }

    /// Convert VS Code settings configuration to standard format
    fn convert_vscode_config(vscode_config: VSCodeSettings) -> MCPConfig {
        let servers = if let Some(mcp_servers) = vscode_config.mcp_servers {
            Some(
                mcp_servers
                    .into_iter()
                    .map(|(name, server_config)| {
                        // Use explicit URL if provided, otherwise build from command
                        let url = if let Some(url) = server_config.url {
                            url
                        } else if server_config.command.is_some() {
                            // For command-based servers, create a placeholder URL
                            format!("stdio://{}", name)
                        } else {
                            // Default URL for servers without explicit configuration
                            "http://localhost:8123".to_string()
                        };

                        MCPServerConfig {
                            name: Some(name),
                            url,
                            description: None, // VS Code settings don't typically include descriptions
                            auth_headers: None,
                            options: None,
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        MCPConfig {
            servers,
            options: None,
            auth_headers: None,
        }
    }

    /// Convert VS Code MCP configuration (new mcp.json format) to standard format
    fn convert_vscode_mcp_config(vscode_mcp_config: VSCodeMCPConfig) -> MCPConfig {
        let servers = if let Some(servers) = vscode_mcp_config.servers {
            Some(
                servers
                    .into_iter()
                    .map(|(name, server_config)| {
                        // Use explicit URL if provided, otherwise build from command
                        let url = if let Some(url) = server_config.url {
                            url
                        } else if server_config.command.is_some() {
                            // For command-based servers, create a placeholder URL
                            format!("stdio://{}", name)
                        } else {
                            // Default URL for servers without explicit configuration
                            "http://localhost:8123".to_string()
                        };

                        MCPServerConfig {
                            name: Some(name),
                            url,
                            description: None, // VS Code MCP format doesn't include descriptions
                            auth_headers: None,
                            options: None,
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        MCPConfig {
            servers,
            options: None,
            auth_headers: None,
        }
    }

    /// Merge two configurations, with the second one taking precedence
    /// Handles server deduplication based on URL
    fn merge_config(base: &mut MCPConfig, other: &MCPConfig) {
        // Merge servers with deduplication
        if let Some(other_servers) = &other.servers {
            match &mut base.servers {
                Some(base_servers) => {
                    // Create a map of existing servers by URL for deduplication
                    let mut server_map: HashMap<String, MCPServerConfig> = HashMap::new();

                    // Move existing servers to the map using drain() to avoid cloning
                    for server in base_servers.drain(..) {
                        let normalized_url = Self::normalize_url(&server.url);
                        server_map.insert(normalized_url, server);
                    }

                    // Add new servers, replacing duplicates based on normalized URLs
                    for server in other_servers {
                        let normalized_url = Self::normalize_url(&server.url);
                        server_map.insert(normalized_url, server.clone());
                    }

                    // Convert back to vector
                    *base_servers = server_map.into_values().collect();
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
                        base_options.format.clone_from(&other_options.format);
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

    /// Validate a loaded MCP configuration with comprehensive checks
    fn validate_server_count(servers: &[MCPServerConfig]) -> Result<()> {
        if servers.len() > 100 {
            return Err(anyhow!(
                "Too many servers configured ({}). Maximum recommended: 100",
                servers.len()
            ));
        }
        Ok(())
    }

    fn validate_server_url(url_str: &str, server_index: usize) -> Result<()> {
        if url_str.is_empty() {
            return Err(anyhow!("Server {} has empty URL", server_index));
        }

        let url_str = url_str.trim();
        if url_str.len() > 2048 {
            return Err(anyhow!(
                "Server {} URL too long ({}), maximum 2048 characters",
                server_index,
                url_str.len()
            ));
        }

        // Handle stdio: URLs separately as they're not standard URLs
        if url_str.starts_with("stdio:") {
            return Ok(());
        }

        // Parse HTTP/HTTPS URLs using the url crate
        if url_str.starts_with("http://") || url_str.starts_with("https://") {
            match Url::parse(url_str) {
                Ok(parsed_url) => {
                    // Validate port if present
                    if let Some(port) = parsed_url.port() {
                        if port == 0 {
                            return Err(anyhow!(
                                "Server {} has invalid port 0: {}",
                                server_index,
                                url_str
                            ));
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(anyhow!(
                    "Server {} has malformed URL '{}': {}",
                    server_index,
                    url_str,
                    e
                )),
            }
        } else {
            Err(anyhow!(
                "Server {} has invalid URL scheme: {}. Supported: http://, https://, stdio:",
                server_index,
                url_str
            ))
        }
    }

    fn validate_server_name(name: &str, server_index: usize) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("Server {} has empty name", server_index));
        }

        if name.len() > 255 {
            return Err(anyhow!(
                "Server {} name too long ({}), maximum 255 characters",
                server_index,
                name.len()
            ));
        }

        Ok(())
    }

    fn validate_auth_headers(
        auth_headers: &HashMap<String, String>,
        server_index: usize,
    ) -> Result<()> {
        for (header_name, header_value) in auth_headers {
            if header_name.trim().is_empty() {
                return Err(anyhow!(
                    "Server {} has empty auth header name",
                    server_index
                ));
            }
            if header_value.trim().is_empty() {
                return Err(anyhow!(
                    "Server {} has empty auth header value for '{}'",
                    server_index,
                    header_name
                ));
            }
            if header_name.len() > 1024 || header_value.len() > 4096 {
                return Err(anyhow!(
                    "Server {} has auth header that's too long",
                    server_index
                ));
            }
        }
        Ok(())
    }

    fn validate_server_description(description: &str, server_index: usize) -> Result<()> {
        if description.len() > 1000 {
            return Err(anyhow!(
                "Server {} description too long ({}), maximum 1000 characters",
                server_index,
                description.len()
            ));
        }
        Ok(())
    }

    fn validate_global_auth_headers(global_auth_headers: &HashMap<String, String>) -> Result<()> {
        for (header_name, header_value) in global_auth_headers {
            if header_name.trim().is_empty() || header_value.trim().is_empty() {
                return Err(anyhow!(
                    "Global auth headers cannot have empty names or values"
                ));
            }
        }
        Ok(())
    }

    fn validate_config(config: &MCPConfig) -> Result<()> {
        if let Some(servers) = &config.servers {
            Self::validate_server_count(servers)?;

            let mut seen_urls = HashMap::new();
            let mut seen_names = HashMap::new();

            for (i, server) in servers.iter().enumerate() {
                Self::validate_server_url(&server.url, i)?;

                let normalized_url = Self::normalize_url(&server.url);
                if let Some(existing_index) = seen_urls.get(&normalized_url) {
                    return Err(anyhow!(
                        "Duplicate server URL detected: server {} and server {} both use URL '{}' (normalized: '{}')",
                        existing_index, i, server.url, normalized_url
                    ));
                }
                seen_urls.insert(normalized_url, i);

                if let Some(name) = &server.name {
                    Self::validate_server_name(name, i)?;
                    let name = name.trim();
                    if let Some(existing_index) = seen_names.get(name) {
                        return Err(anyhow!(
                            "Duplicate server name '{}': server {} and server {} both use this name",
                            name, existing_index, i
                        ));
                    }
                    seen_names.insert(name.to_string(), i);
                }

                if let Some(auth_headers) = &server.auth_headers {
                    Self::validate_auth_headers(auth_headers, i)?;
                }

                if let Some(description) = &server.description {
                    Self::validate_server_description(description, i)?;
                }
            }
        }

        if let Some(global_auth_headers) = &config.auth_headers {
            Self::validate_global_auth_headers(global_auth_headers)?;
        }

        Ok(())
    }

    /// Check if any IDE configuration files exist
    pub fn has_config_files(&self) -> bool {
        self.config_paths.iter().any(|(path, _)| path.exists())
    }

    /// Get statistics about discovered configuration files
    #[allow(dead_code)]
    pub fn get_config_stats(&self) -> (usize, usize, HashMap<MCPClient, usize>) {
        let total_paths = self.config_paths.len();
        let existing_files = self
            .config_paths
            .iter()
            .filter(|(path, _)| path.exists())
            .count();

        let mut client_counts = HashMap::new();
        for (path, client) in &self.config_paths {
            if path.exists() {
                *client_counts.entry(client.clone()).or_insert(0) += 1;
            }
        }

        (total_paths, existing_files, client_counts)
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

        let config = MCPConfigManager::load_config_from_path(&config_path).unwrap();

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

        MCPConfigManager::merge_config(&mut base, &other);

        assert!(base.servers.is_some());
        assert_eq!(base.servers.unwrap().len(), 1);
        assert!(base.options.is_some());
    }

    #[test]
    fn test_merge_config_deduplication() {
        let mut base = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("server1".to_string()),
                url: "http://localhost:3000".to_string(),
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };

        let other = MCPConfig {
            servers: Some(vec![
                MCPServerConfig {
                    name: Some("server1-updated".to_string()),
                    url: "http://localhost:3000".to_string(), // Same URL - should replace
                    description: Some("Updated server".to_string()),
                    auth_headers: None,
                    options: None,
                },
                MCPServerConfig {
                    name: Some("server2".to_string()),
                    url: "http://localhost:4000".to_string(), // Different URL - should add
                    description: None,
                    auth_headers: None,
                    options: None,
                },
            ]),
            options: None,
            auth_headers: None,
        };

        MCPConfigManager::merge_config(&mut base, &other);

        let servers = base.servers.unwrap();
        assert_eq!(servers.len(), 2); // Should have 2 servers, not 3

        // Find the server with URL localhost:3000 - should be the updated one
        let updated_server = servers
            .iter()
            .find(|s| s.url == "http://localhost:3000")
            .unwrap();
        assert_eq!(updated_server.name.as_ref().unwrap(), "server1-updated");
        assert_eq!(
            updated_server.description.as_ref().unwrap(),
            "Updated server"
        );

        // Should also have the new server
        let new_server = servers
            .iter()
            .find(|s| s.url == "http://localhost:4000")
            .unwrap();
        assert_eq!(new_server.name.as_ref().unwrap(), "server2");
    }

    #[test]
    fn test_get_client_from_path() {
        assert_eq!(
            MCPConfigManager::get_client_from_path("/home/user/.cursor/mcp.json"),
            Some(MCPClient::Cursor)
        );
        assert_eq!(
            MCPConfigManager::get_client_from_path("/home/user/.codium/windsurf/mcp_config.json"),
            Some(MCPClient::Windsurf)
        );
        assert_eq!(
            MCPConfigManager::get_client_from_path("/home/user/.vscode/mcp.json"),
            Some(MCPClient::VSCode)
        );
        assert_eq!(
            MCPConfigManager::get_client_from_path("/home/user/.claude/mcp.json"),
            Some(MCPClient::Claude)
        );
        assert_eq!(
            MCPConfigManager::get_client_from_path("/home/user/.config/nvim/mcp.json"),
            Some(MCPClient::Neovim)
        );
        assert_eq!(
            MCPConfigManager::get_client_from_path("/some/unknown/path.json"),
            None
        );
    }

    #[test]
    fn test_client_shorthands_to_paths() {
        let paths = MCPConfigManager::client_shorthands_to_paths(&["cursor", "claude"]);

        // Should contain paths for both cursor and claude
        let cursor_paths: Vec<_> = paths
            .iter()
            .filter(|(_, client)| *client == MCPClient::Cursor)
            .collect();
        let claude_paths: Vec<_> = paths
            .iter()
            .filter(|(_, client)| *client == MCPClient::Claude)
            .collect();

        assert!(!cursor_paths.is_empty());
        assert!(!claude_paths.is_empty());
    }

    #[test]
    fn test_validate_config() {
        // Valid config
        let valid_config = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("test".to_string()),
                url: "http://localhost:3000".to_string(),
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&valid_config).is_ok());

        // Invalid config - empty URL
        let invalid_config = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("test".to_string()),
                url: String::new(),
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&invalid_config).is_err());

        // Invalid config - bad URL format
        let invalid_config2 = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("test".to_string()),
                url: "not-a-url".to_string(),
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
    fn test_mcp_client_enum() {
        assert_eq!(MCPClient::Cursor.name(), "cursor");
        assert_eq!(MCPClient::Windsurf.name(), "windsurf");
        assert_eq!(MCPClient::VSCode.name(), "vscode");
        assert_eq!(MCPClient::Claude.name(), "claude");
        assert_eq!(MCPClient::Neovim.name(), "neovim");
        assert_eq!(MCPClient::Helix.name(), "helix");
        assert_eq!(MCPClient::Zed.name(), "zed");
    }

    #[test]
    fn test_url_normalization() {
        assert_eq!(
            MCPConfigManager::normalize_url("HTTP://LOCALHOST:3000/"),
            "http://localhost:3000"
        );
        assert_eq!(
            MCPConfigManager::normalize_url("http://127.0.0.1:3000"),
            "http://localhost:3000"
        );
        assert_eq!(
            MCPConfigManager::normalize_url("https://0.0.0.0:8080/"),
            "https://localhost:8080"
        );
        assert_eq!(MCPConfigManager::normalize_url("stdio:test"), "stdio:test");
        assert_eq!(
            MCPConfigManager::normalize_url("http://example.com/path/"),
            "http://example.com/path"
        );
    }

    #[test]
    fn test_enhanced_client_detection() {
        use std::path::PathBuf;

        // Test exact component matches
        assert_eq!(
            MCPConfigManager::get_client_from_path(PathBuf::from(
                "/Applications/Cursor.app/Contents/mcp.json"
            )),
            Some(MCPClient::Cursor)
        );

        // Test Windows paths - use forward slashes for cross-platform compatibility
        assert_eq!(
            MCPConfigManager::get_client_from_path(PathBuf::from(
                "C:/Users/test/AppData/Roaming/Code/User/mcp.json"
            )),
            Some(MCPClient::VSCode)
        );

        // Test extension ID path
        assert_eq!(
            MCPConfigManager::get_client_from_path(PathBuf::from(
                "/home/user/.config/rooveterinaryinc.cursor-mcp/config.json"
            )),
            Some(MCPClient::Cursor)
        );

        // Test disambiguation - should not match generic "code" in paths
        assert_eq!(
            MCPConfigManager::get_client_from_path(PathBuf::from(
                "/home/user/my-code-project/config.json"
            )),
            None
        );
    }

    #[test]
    fn test_enhanced_validation() {
        // Test duplicate URL detection with normalization
        let config_with_duplicate_urls = MCPConfig {
            servers: Some(vec![
                MCPServerConfig {
                    name: Some("server1".to_string()),
                    url: "http://localhost:3000".to_string(),
                    description: None,
                    auth_headers: None,
                    options: None,
                },
                MCPServerConfig {
                    name: Some("server2".to_string()),
                    url: "HTTP://LOCALHOST:3000/".to_string(), // Different case and trailing slash
                    description: None,
                    auth_headers: None,
                    options: None,
                },
            ]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&config_with_duplicate_urls).is_err());

        // Test duplicate names
        let config_with_duplicate_names = MCPConfig {
            servers: Some(vec![
                MCPServerConfig {
                    name: Some("same-name".to_string()),
                    url: "http://localhost:3000".to_string(),
                    description: None,
                    auth_headers: None,
                    options: None,
                },
                MCPServerConfig {
                    name: Some("same-name".to_string()),
                    url: "http://localhost:4000".to_string(),
                    description: None,
                    auth_headers: None,
                    options: None,
                },
            ]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&config_with_duplicate_names).is_err());

        // Test invalid port
        let config_with_invalid_port = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("test".to_string()),
                url: "http://localhost:0".to_string(),
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };
        assert!(MCPConfigManager::validate_config(&config_with_invalid_port).is_err());
    }

    #[test]
    fn test_platform_detection() {
        // Test that platform detection works without panicking
        let paths = MCPConfigManager::discover_config_paths();
        assert!(
            !paths.is_empty(),
            "Should discover some configuration paths"
        );

        // Test that caching works
        let cached_paths = MCPConfigManager::get_all_config_paths();
        assert_eq!(
            paths.len(),
            cached_paths.len(),
            "Cached paths should match discovered paths"
        );
    }

    #[test]
    fn test_merge_config_with_normalization() {
        let mut base = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("server1".to_string()),
                url: "http://localhost:3000/".to_string(), // With trailing slash
                description: None,
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };

        let other = MCPConfig {
            servers: Some(vec![MCPServerConfig {
                name: Some("server1-updated".to_string()),
                url: "HTTP://LOCALHOST:3000".to_string(), // Different case, no trailing slash
                description: Some("Updated".to_string()),
                auth_headers: None,
                options: None,
            }]),
            options: None,
            auth_headers: None,
        };

        MCPConfigManager::merge_config(&mut base, &other);

        let servers = base.servers.unwrap();
        assert_eq!(servers.len(), 1); // Should be deduplicated due to URL normalization
        assert_eq!(servers[0].name.as_ref().unwrap(), "server1-updated");
        assert_eq!(servers[0].description.as_ref().unwrap(), "Updated");
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
    /// Enable/disable YARA rule scanning
    pub enable_yara: bool,
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
#[allow(clippy::struct_excessive_bools)]
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
                api_key: String::new(),
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
                enable_yara: true,
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

        debug!("Loaded configuration from config.yaml");
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
