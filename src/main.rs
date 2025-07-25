use clap::{Parser, Subcommand};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod constants;
mod core;
mod scanner;
mod security;
mod server;
mod types;
mod utils;

use scanner::MCPScanner;
use server::MCPScannerServer;
use types::{config_utils, ScanConfigBuilder};
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

    /// Scan MCP servers from IDE configuration files (~/.cursor/mcp.json, ~/.codium/windsurf/mcp_config.json)
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

    // Load configuration and setup logging
    let config_manager = config::ScannerConfigManager::new();
    let scanner_config = config_manager.load_config().unwrap_or_default();

    // Determine logging level (CLI args take precedence over config)
    let level = if cli.debug || cli.verbose {
        Level::DEBUG
    } else {
        // Use config level if available, otherwise default to INFO
        match scanner_config.logging.level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        }
    };

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();

    info!("Starting MCP Scanner");

    match cli.command {
        Commands::Scan {
            url,
            auth_headers,
            format,
        } => {
            let auth_headers_map = parse_auth_headers(&auth_headers);
            let output_format = format.unwrap_or(scanner_config.scanner.format.clone());
            let options = ScanConfigBuilder::new()
                .timeout(scanner_config.scanner.scan_timeout)
                .http_timeout(scanner_config.scanner.http_timeout)
                .detailed(scanner_config.scanner.detailed)
                .format(output_format.clone())
                .auth_headers(auth_headers_map)
                .build();

            // Validate configuration
            if let Err(e) = config_utils::validate_scan_config(&options) {
                error!("Invalid configuration: {}", e);
                std::process::exit(1);
            }

            let options_clone = options.clone();
            let scanner = MCPScanner::new_with_timeout(scanner_config.scanner.http_timeout);

            match scanner.scan_single(&url, options).await {
                Ok(result) => {
                    utils::print_result(&result, &output_format, options_clone.detailed);
                }
                Err(e) => {
                    error!(
                        "{}",
                        error_utils::create_error_msg("Scan operation", &e.to_string())
                    );
                    std::process::exit(1);
                }
            }
        }

        Commands::ScanConfig {
            auth_headers,
            format,
        } => {
            let auth_headers_map = parse_auth_headers(&auth_headers);
            let output_format = format.unwrap_or(scanner_config.scanner.format.clone());
            let options = ScanConfigBuilder::new()
                .timeout(scanner_config.scanner.scan_timeout)
                .http_timeout(scanner_config.scanner.http_timeout)
                .detailed(scanner_config.scanner.detailed)
                .format(output_format.clone())
                .auth_headers(auth_headers_map)
                .build();

            // Validate configuration
            if let Err(e) = config_utils::validate_scan_config(&options) {
                error!("Invalid configuration: {}", e);
                std::process::exit(1);
            }

            let scanner = MCPScanner::new_with_timeout(scanner_config.scanner.http_timeout);

            match scanner.scan_from_config(options).await {
                Ok(results) => {
                    for result in results {
                        utils::print_result(
                            &result,
                            &output_format,
                            scanner_config.scanner.detailed,
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "{}",
                        error_utils::create_error_msg(
                            "IDE configuration scan operation",
                            &e.to_string()
                        )
                    );
                    std::process::exit(1);
                }
            }
        }

        Commands::InitConfig { force } => {
            let config_manager = config::ScannerConfigManager::new();

            if config_manager.has_config_file() && !force {
                println!("config.yaml already exists. Use --force to overwrite.");
                std::process::exit(1);
            }

            match config_manager.save_config(&config::ScannerConfig::default()) {
                Ok(_) => {
                    println!("Created config.yaml with default settings");
                    println!("📝 Edit the file to customize LLM settings, security checks, and other options");
                }
                Err(e) => {
                    error!("Failed to create config.yaml: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Server { port, host } => {
            info!("Starting MCP Scanner microservice on {}:{}", host, port);

            match MCPScannerServer::new() {
                Ok(server) => {
                    let server = server.with_port(port).with_host(host);
                    if let Err(e) = server.start().await {
                        error!("Server failed: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    error!("Failed to create server: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
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
