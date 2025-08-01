use clap::{Parser, Subcommand};
use tracing::{debug, error, warn, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::ScannerConfig;

mod banner;
mod config;
mod constants;
mod core;
mod mcp_client;
mod scanner;
mod security;
mod server;
mod types;
mod utils;

use banner::display_banner;
use scanner::MCPScanner;
use server::MCPScannerServer;
use types::{config_utils, ScanConfigBuilder, ScanOptions};
use utils::error_utils;

#[derive(Parser)]
#[command(
    name = "ramparts",
    about = "A CLI tool for scanning Model Context Protocol (MCP) servers",
    version,
    long_about = "Scans MCP servers to discover available tools, resources, and capabilities with comprehensive security analysis.

SECURITY ASSESSMENTS:

Tool Security Assessments:
  • Tool Poisoning: Detects tools with destructive or malicious intent that could harm the system or data
  • SQL Injection: Identifies tools allowing SQL injection attacks that could compromise databases
  • Command Injection: Detects tools that may execute system commands, posing critical security risks
  • Path Traversal: Finds tools allowing directory traversal attacks to access unauthorized files
  • Authentication Bypass: Identifies tools that could allow unauthorized access to protected resources
  • Secrets Leakage: Detects tools processing sensitive credentials like API keys, passwords, tokens

Prompt Security Assessments:
  • Prompt Injection: Identifies prompts vulnerable to injection attacks that could override safety measures
  • Jailbreak: Detects prompts that could bypass AI safety measures and restrictions
  • PII Leakage: Finds prompts handling personal information like emails, addresses, SSNs, credit cards

Resource Security Assessments:
  • Path Traversal: Detects resources with directory traversal vulnerabilities in URIs
  • Sensitive Data Exposure: Identifies resources containing sensitive information or credentials

IMPACT LEVELS:
  • CRITICAL: Immediate security risk requiring immediate attention
  • HIGH: Significant security vulnerability that should be addressed promptly
  • MEDIUM: Moderate security concern that should be reviewed
  • LOW: Minor security issue that may need monitoring

EXAMPLES:
  • Basic scan: ramparts scan http://localhost:3000
  • Security scan: ramparts scan http://localhost:3000
  • From IDE config: ramparts scan-config
  • Initialize config: ramparts init-config"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging for detailed operation tracking
    ///
    /// This provides detailed logs about:
    ///   • HTTP requests and responses
    ///   • Security assessment progress
    ///   • Tool, resource, and prompt discovery
    ///   • Error details and debugging information
    ///
    /// Useful for troubleshooting connection issues or understanding scan behavior.
    /// Note: Use --debug for JSON-RPC protocol debugging.
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug output for detailed operation tracking
    ///
    /// This provides detailed logs about:
    ///   • HTTP requests and responses
    ///   • Security assessment progress
    ///   • Tool, resource, and prompt discovery
    ///   • Error details and debugging information
    ///   • JSON-RPC protocol communication
    ///
    /// Useful for troubleshooting connection issues or understanding scan behavior.
    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a single MCP server for tools, resources, and security vulnerabilities
    Scan {
        /// MCP server URL or endpoint to scan
        #[arg(value_name = "URL")]
        url: String,

        /// Authentication headers for the MCP server (format: "Header: Value")
        #[arg(long, value_delimiter = ',')]
        auth_headers: Vec<String>,

        /// Output format (json, raw, table, text)
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
    },

    /// Scan MCP servers from IDE configuration files (~/.cursor/mcp.json, ~/.`codium/windsurf/mcp_config.json`)
    ScanConfig {
        /// Authentication headers for the MCP servers (format: "Header: Value")
        #[arg(long, value_delimiter = ',')]
        auth_headers: Vec<String>,

        /// Output format (json, raw, table, text)
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
    },

    /// Generate a default config.yaml file
    InitConfig {
        /// Overwrite existing config.yaml if it exists
        #[arg(short, long)]
        force: bool,
    },

    /// Start the MCP Scanner microservice
    Server {
        /// Port to run the server on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind the server to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    display_banner();

    let scanner_config = load_scanner_config();
    setup_logging(&cli, &scanner_config);
    debug!("Starting MCP Scanner");

    let scanner = create_scanner_if_needed(&cli, &scanner_config);
    execute_command(cli, scanner_config, scanner).await?;

    Ok(())
}

/// Loads the scanner configuration, using defaults if loading fails
fn load_scanner_config() -> ScannerConfig {
    let config_manager = config::ScannerConfigManager::new();
    match config_manager.load_config() {
        Ok(config) => config,
        Err(e) => {
            warn!("Failed to load scanner config, using defaults: {}", e);
            ScannerConfig::default()
        }
    }
}

/// Sets up logging based on CLI arguments and configuration
fn setup_logging(cli: &Cli, scanner_config: &ScannerConfig) {
    let level = determine_log_level(cli, scanner_config);

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();
}

/// Determines the appropriate log level from CLI args and config
fn determine_log_level(cli: &Cli, scanner_config: &ScannerConfig) -> Level {
    if cli.debug || cli.verbose {
        Level::DEBUG
    } else {
        match scanner_config.logging.level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        }
    }
}

/// Creates an MCP scanner instance if needed for the given command
fn create_scanner_if_needed(cli: &Cli, scanner_config: &ScannerConfig) -> Option<MCPScanner> {
    match &cli.command {
        Commands::Scan { .. } | Commands::ScanConfig { .. } => {
            match MCPScanner::with_timeout(scanner_config.scanner.http_timeout) {
                Ok(scanner) => Some(scanner),
                Err(e) => {
                    error!("Failed to create scanner: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => None,
    }
}

/// Executes the specified command with the given configuration and scanner
async fn execute_command(
    cli: Cli,
    scanner_config: ScannerConfig,
    scanner: Option<MCPScanner>,
) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan {
            url,
            auth_headers,
            format,
        } => handle_scan_command(url, auth_headers, format, &scanner_config, scanner).await,
        Commands::ScanConfig {
            auth_headers,
            format,
        } => handle_scan_config_command(auth_headers, format, &scanner_config, scanner).await,
        Commands::InitConfig { force } => {
            handle_init_config_command(force);
            Ok(())
        }
        Commands::Server { port, host } => handle_server_command(port, host).await,
    }
}

/// Handles the scan command for a single URL
async fn handle_scan_command(
    url: String,
    auth_headers: Vec<String>,
    format: Option<String>,
    scanner_config: &ScannerConfig,
    scanner: Option<MCPScanner>,
) -> Result<(), Box<dyn std::error::Error>> {
    let auth_headers_map = parse_auth_headers(&auth_headers);
    let output_format = format.unwrap_or(scanner_config.scanner.format.clone());
    let options = build_scan_options(scanner_config, &output_format, auth_headers_map);

    validate_scan_config(&options);

    let scanner = scanner
        .as_ref()
        .expect("Scanner should be initialized for scan command");

    match scanner.scan_single(&url, options.clone()).await {
        Ok(result) => {
            utils::print_result(&result, &output_format, options.detailed);
            Ok(())
        }
        Err(e) => {
            error!(
                "{}",
                error_utils::format_error("Scan operation", &e.to_string())
            );
            std::process::exit(1);
        }
    }
}

/// Handles the scan-config command for IDE configurations
async fn handle_scan_config_command(
    auth_headers: Vec<String>,
    format: Option<String>,
    scanner_config: &ScannerConfig,
    scanner: Option<MCPScanner>,
) -> Result<(), Box<dyn std::error::Error>> {
    let auth_headers_map = parse_auth_headers(&auth_headers);
    let output_format = format.unwrap_or(scanner_config.scanner.format.clone());
    let options = build_scan_options(scanner_config, &output_format, auth_headers_map);

    validate_scan_config(&options);

    let scanner = scanner
        .as_ref()
        .expect("Scanner should be initialized for scan-config command");

    match scanner.scan_config(options).await {
        Ok(results) => {
            for result in results {
                utils::print_result(&result, &output_format, scanner_config.scanner.detailed);
            }
            Ok(())
        }
        Err(e) => {
            error!(
                "{}",
                error_utils::format_error("IDE configuration scan operation", &e.to_string())
            );
            std::process::exit(1);
        }
    }
}

/// Handles the init-config command
fn handle_init_config_command(force: bool) {
    let config_manager = config::ScannerConfigManager::new();

    if config_manager.has_config_file() && !force {
        println!("config.yaml already exists. Use --force to overwrite.");
        std::process::exit(1);
    }

    match config_manager.save_config(&config::ScannerConfig::default()) {
        Ok(()) => {
            println!("Created config.yaml with default settings");
            println!(
                "📝 Edit the file to customize LLM settings, security checks, and other options"
            );
        }
        Err(e) => {
            error!("Failed to create config.yaml: {}", e);
            std::process::exit(1);
        }
    }
}

/// Handles the server command
async fn handle_server_command(port: u16, host: String) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Starting MCP Scanner microservice on {}:{}", host, port);

    match MCPScannerServer::new() {
        Ok(server) => {
            let server = server.with_port(port).with_host(host);
            if let Err(e) = server.start().await {
                error!("Server failed: {}", e);
                std::process::exit(1);
            }
            Ok(())
        }
        Err(e) => {
            error!("Failed to create server: {}", e);
            std::process::exit(1);
        }
    }
}

/// Builds scan options from configuration and parameters
fn build_scan_options(
    scanner_config: &ScannerConfig,
    output_format: &str,
    auth_headers_map: Option<std::collections::HashMap<String, String>>,
) -> ScanOptions {
    ScanConfigBuilder::new()
        .timeout(scanner_config.scanner.scan_timeout)
        .http_timeout(scanner_config.scanner.http_timeout)
        .detailed(scanner_config.scanner.detailed)
        .format(output_format.to_string())
        .auth_headers(auth_headers_map)
        .build()
}

/// Validates scan configuration and exits on error
fn validate_scan_config(options: &ScanOptions) {
    if let Err(e) = config_utils::validate_scan_config(options) {
        error!("Invalid configuration: {}", e);
        std::process::exit(1);
    }
}

fn parse_auth_headers(headers: &[String]) -> Option<std::collections::HashMap<String, String>> {
    if headers.is_empty() {
        return None;
    }

    let mut map = std::collections::HashMap::new();
    for header in headers {
        if let Some((key, value)) = header.split_once(':') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Some(map)
}
