<div align="center">

# Ramparts: mcp (model context protocol) scanner

<img src="assets/ramparts.png" alt="Ramparts Banner" width="250" />

*A fast, lightweight security scanner for Model Context Protocol (MCP) servers with built-in vulnerability detection.*

[![Crates.io](https://img.shields.io/crates/v/ramparts)](https://crates.io/crates/ramparts)
[![GitHub stars](https://img.shields.io/github/stars/getjavelin/ramparts?style=social)](https://github.com/getjavelin/ramparts)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/github/actions/workflow/status/getjavelin/ramparts/pr-check.yml?label=tests)](https://github.com/getjavelin/ramparts/actions)
[![Clippy](https://img.shields.io/github/actions/workflow/status/getjavelin/ramparts/pr-check.yml?label=lint)](https://github.com/getjavelin/ramparts/actions)
[![Release](https://img.shields.io/github/release/getjavelin/ramparts)](https://github.com/getjavelin/ramparts/releases)

</div>

## Overview

**Ramparts** is a scanner designed for the **Model Context Protocol (MCP)** ecosystem. As AI agents and LLMs increasingly rely on external tools and resources through MCP servers, ensuring the security of these connections has become critical.   

The Model Context Protocol (MCP) is an open standard that enables AI assistants to securely connect to external data sources and tools. It allows AI agents to access databases, file systems, and APIs through toolcalling to retrieve real-time information and interact with external or internal services.

Ramparts is under active development. Read our [launch blog](https://www.getjavelin.com/blogs/ramparts-mcp-scan).

### The Security Challenge

MCP servers can expose powerful capabilities to AI agents, including:
- **File system access** (read/write files, directory traversal)
- **Database operations** (SQL queries, data manipulation)
- **API integrations** (external/internal/local service calls, authentication)
- **System commands** (process execution, system administration)

Without proper security analysis, these capabilities can become attack vectors for:
- **Tool Poisoning** - bypassing AI safety measures
- **MCP Rug Pulls** - unauthorized changes to MCP tool descriptions after initial user approval
- **Cross-Origin Escalation** - exploiting tools across multiple domains to hijack context or inject malicious content
- **Data exfiltration** - leaking sensitive information
- **Privilege escalation** - gaining unauthorized access
- **Path traversal attacks** - accessing files outside intended directories
- **Command injection** - executing unauthorized system commands
- **SQL injection** - manipulating database queries

📚 **[Detailed Security Features Documentation](docs/security-features.md)** - Complete guide to all attack vectors and detection methods

### What Ramparts Does

Ramparts provides **security scanning** of MCP servers by:

1. **Discovering Capabilities**: Scans all MCP endpoints to identify available tools, resources, and prompts
2. **Static Analysis**: Performs yara-based checks for common vulnerabilities
3. **Cross-Origin Analysis**: Detects when tools span multiple domains, which could enable context hijacking or injection attacks
4. **LLM-Powered Analysis**: Uses AI models to detect sophisticated security issues
5. **Risk Assessment**: Categorizes findings by severity and provides actionable recommendations


## Who Ramparts is For

Ramparts is designed for developers using local, remote MCP servers or building their own MCP servers and interested in scanning it for any vulnerabilities it may expose. Developers may use Ramparts locally to scan the MCP servers they use in their local development environment (e.g., Cursor, Windsurf, Claude Code etc.,). 

**If you're using MCP servers** - whether they're running locally on your machine or hosted remotely - Ramparts helps you understand what security risks they might pose. You can scan third-party MCP servers before connecting to them, or validate your own local MCP servers before deploying them to production.

**If you're building MCP servers** - whether you're creating tools, resources, or prompts - Ramparts gives you confidence that your implementation doesn't expose vulnerabilities to AI agents. It's especially useful for developers who want to ensure their MCP tools are secure by design.

## Why Rust?

The Ramparts mcp scanner is implemented in Rust to prioritize performance, reliability, and broad portability. Rust offers native execution speed with minimal memory overhead, making it well-suited for analyzing large prompt contexts, tool manifests, or server topologies—without the need for a heavyweight runtime. Ramparts was built with a view of operating in CI pipelines, agent sandboxes, or constrained edge environments which made the ability to compile to a single, compact binary essential.

## Features

- **Comprehensive MCP Coverage**: Analyzes all MCP endpoints (server/info, tools/list, resources/list, prompts/list) and evaluates each tool, resource, and prompt
- **Advanced Security Detection**: Detects path traversal, command injection, SQL injection, prompt injection, secret leakage, auth bypass, cross-origin escalation, and more using both static checks and LLM-assisted analysis
- **Optional YARA-X Integration**: Advanced pattern-based scanning with configurable YARA rules using the modern YARA-X engine for enhanced security analysis
- **High Performance**: Built in Rust for fast, efficient scanning of large MCP servers with minimal memory overhead
- **Flexible Installation**: Install with or without YARA-X dependency based on your security requirements
- **Multiple Transport Support**: HTTP and STDIO transport mechanisms for various MCP server configurations
- **Rich Output Formats**: Choose from tree-style text, JSON, or raw formats for easy integration with scripts and dashboards
- **Configuration Management**: Load settings from IDE configuration files and custom YAML configs
- **Modular & Extensible**: Add custom rules or tweak severity thresholds via a simple configuration file  

## Use Cases

- **Security Audits**: Comprehensive assessment of MCP server security posture
- **Development**: Testing MCP servers during development and testing phases
- **Compliance**: Meeting security requirements for AI agent deployments

## Caution

- **Adopt a layered approach** consider a layered approach to security. **ramparts** scanner is designed to work on the mcp server & tool _metadata_. It can catch **Tool Poisoning** or other static vulnerabilities in MCP server but you need to continually run the scans AND implement runtime MCP guardrails. For runtime attack detection of MCP tools, please contact support@getjavelin.com for Javelin's runtime MCP guardrails. 
- **Evolving standards & threats** both the MCP standard as well as the AI/MCP threat landscape is evolving rapidly and there may be several threats or attack vectors that ramparts may fail to catch (until it catches up with the specific attack/threat)

## Quick Start

**From crates.io (Recommended)**
```bash
cargo install ramparts
```

**From source**
```bash
git clone https://github.com/getjavelin/ramparts.git
cd ramparts
cargo install --path .
```

**Scan with verbose output**
```bash
ramparts scan <url> 
```

### Example Output

```
================================================================================
MCP Server Scan Result
================================================================================
URL: https://api.githubcopilot.com/mcp/
Status: Success
Response Time: 1234ms
Timestamp: 2024-01-01T12:00:00.000Z

Server Information:
  Name: GitHub Copilot MCP Server
  Version: 1.0.0
  Description: GitHub Copilot MCP server for code assistance
  Capabilities: tools, resources, prompts

Tools: 74
Resources: 0
Prompts: 0

Security Assessment Results
================================================================================
🌐 GitHub Copilot MCP Server
  ✅ All tools passed security checks

  └── push_files passed
  └── create_or_update_file warning
      📋 Analysis: Standard GitHub file creation/update functionality
      ├── HIGH: Tool allowing directory traversal attacks: Potential Path Traversal Vulnerability
      │   Details: The tool accepts a 'path' parameter without proper validation, allowing potential path traversal attacks.
  └── delete_file warning
      📋 Analysis: Standard GitHub file deletion functionality
      ├── HIGH: Tool allowing directory traversal attacks: Potential Path Traversal Vulnerability
      │   Details: The tool allows the deletion of a file from a GitHub repository and accepts parameters like branch, message, owner, path, and repo. If path validation is not implemented properly, an attacker could manipulate the path to access files outside the intended directory.

YARA Scan Results
================================================================================
⚠️ PRE-SCAN - WARNING
  Context: Pre-scan completed: 2 rules executed on 74 items
  Items scanned: 74
  Security matches: 1
  Rules executed: secrets_leakage:*, cross_origin_escalation:*
  Security issues detected: cross_origin_escalation:CrossDomainContamination

🔍 Detailed Results:
⚠️ domain-analysis (domain-analysis)
  Rule: CrossDomainContamination (HIGH)
  Description: Detected tools and resources spanning multiple domains, indicating potential cross-origin escalation risk
  Matched: Cross-domain contamination detected across 2 domains: api.github.com, webhooks.github.com
  Context: Found tools and resources spanning 2 different root domains

Summary:
  • Tools scanned: 74
  • Warnings found: 2 tools with 2 total warnings
================================================================================
```

## Examples

### Scan Different MCP Servers

**GitHub Copilot**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"
```

**Local MCP server**
```bash
ramparts scan http://localhost:3000/mcp/
```

**Custom MCP server with API key**
```bash
ramparts scan https://api.example.com/mcp/ --auth-headers "X-API-Key: $API_KEY"
```

**With custom timeout**
```bash
ramparts scan <url> --timeout 60
```

### Advanced Scanning Options

**Scan with custom severity threshold**
```bash
ramparts scan <url> --min-severity HIGH
```

**Scan with specific output format**
```bash
ramparts scan <url> --output json --pretty
```

**Scan with custom configuration**
```bash
ramparts scan <url> --config custom-ramparts.yaml
```

## Advanced Usage

### Server Mode

Ramparts can run as a REST API server for continuous monitoring and integration with other systems:

```bash
# Start server on default port 3000
ramparts server

# Start server on custom port and host
ramparts server --port 8080 --host 0.0.0.0
```

📚 **[Complete API Documentation](docs/api.md)** - Detailed API reference with all endpoints, request/response formats, and examples

### Batch Scanning

Scan multiple servers from a file:

```bash
# Create a servers list
echo "https://server1.com/mcp/
https://server2.com/mcp/
https://server3.com/mcp/" > servers.txt

# Run batch scan
ramparts scan --batch servers.txt
```

## CLI Reference

### Basic Commands

```bash
# Scan an MCP server
ramparts scan <url> [options]

# Start Ramparts server mode
ramparts server [options]

# Initialize configuration file
ramparts init-config

# Show help
ramparts --help
ramparts scan --help
```

### Scan Options

```bash
Options:
  -a, --auth-headers <HEADERS>    Authentication headers
  -o, --output <FORMAT>           Output format (text, json, raw) [default: text]
  -t, --timeout <SECONDS>         Request timeout in seconds [default: 30]
  -v, --verbose                   Enable verbose output
  --min-severity <LEVEL>          Minimum severity level (LOW, MEDIUM, HIGH, CRITICAL)
  --config <FILE>                 Custom configuration file
  --pretty                        Pretty print JSON output
```

### Server Options

```bash
Options:
  -p, --port <PORT>               Server port [default: 3000]
  -h, --host <HOST>               Server host [default: 0.0.0.0]
  --config <FILE>                 Configuration file
```

## Configuration

Ramparts uses a `ramparts.yaml` configuration file for customizing security rules and thresholds:

### Initialize Configuration

Create a custom configuration file:

```bash
ramparts init-config
```

This creates a `ramparts.yaml` file with the following structure:

```yaml
# Example ramparts.yaml
llm:
  provider: "openai"
  model: "gpt-4o"
  base_url: "https://api.openai.com/v1"
  api_key: ""
  timeout: 30
  max_tokens: 4000
  temperature: 0.1

scanner:
  http_timeout: 30
  scan_timeout: 60
  detailed: false
  format: "table"
  parallel: true
  max_retries: 3
  retry_delay_ms: 1000
  llm_batch_size: 10
  enable_yara: true  # Set to false to disable YARA scanning

security:
  enabled: true
  min_severity: "low"
  checks:
    tool_poisoning: true
    secrets_leakage: true
    sql_injection: true
    command_injection: true
    path_traversal: true
    auth_bypass: true
    cross_origin_escalation: true
    prompt_injection: true
    pii_leakage: true
    jailbreak: true

logging:
  level: "info"
  colored: true
  timestamps: true

performance:
  tracking: true
  slow_threshold_ms: 5000
```

### YARA-X Configuration

Control YARA-X scanning via config file:
```yaml
scanner:
  enable_yara: true   # Enable/disable YARA-X scanning
```

**Rules Directory**: `rules/pre/` (auto-loaded `.yar` files)  
**Built-in Rules**: secrets_leakage, command_injection, path_traversal, sql_injection, cross_origin_escalation
**Custom Rules**: Create `.yar` files and place directly in `rules/pre/` or `rules/post/` - no compilation needed with YARA-X!

## Output Formats

### Text Format (Default)
```bash
ramparts scan <url>
```

### JSON Format
```bash
ramparts scan <url> --output json
```

```json
{
  "url": "https://api.githubcopilot.com/mcp/",
  "status": "success",
  "response_time": 1234,
  "server_info": {
    "name": "GitHub Copilot MCP Server",
    "version": "1.0.0"
  },
  "security_issues": [
    {
      "tool": "create_or_update_file",
      "severity": "HIGH",
      "type": "path_traversal",
      "description": "Potential path traversal vulnerability"
    }
  ]
}
```

### Raw Format
```bash
ramparts scan <url> --output raw
```

## Troubleshooting

### Common Issues

**Connection Timeout**
```bash
# Increase timeout
ramparts scan <url> --timeout 60
```

**Authentication Errors**
```bash
# Check your auth headers format
ramparts scan <url> --auth-headers "Authorization: Bearer $TOKEN"
```

**Permission Denied**
```bash
# Check file permissions
chmod +x $(which ramparts)
```

**Configuration File Not Found**
```bash
# Initialize configuration
ramparts init-config
```

### YARA-X Related Issues

**YARA-X Rules Not Loading**
```bash
# Check if rules directory exists
ls -la rules/
ls -la rules/pre/

# List .yar files (no compilation needed)
ls rules/pre/*.yar
```

**Rule Syntax Errors**
```bash
# YARA-X provides better error messages for rule syntax issues
# Check rule syntax in your .yar files:
# 1. Ensure proper rule structure
# 2. Verify string escaping (especially { characters in regex)
# 3. Check metadata format
```

**Performance Issues with YARA-X**
```bash
# If YARA-X scanning is slow, you can:
# 1. Disable YARA-X temporarily
echo "scanner:
  enable_yara: false" > ramparts.yaml

# 2. Or use --no-default-features installation
cargo install ramparts --no-default-features --force
```

**YARA-X Rules Directory Permissions**
```bash
# Error: Permission denied accessing rules
# Solution: Check directory permissions
chmod -R 755 rules/
chmod 644 rules/pre/*.yar
```

**Custom Rules Not Working**
```bash
# Debug rule loading
# 1. Check file extension (.yar files directly)
ls rules/pre/*.yar

# 2. Verify rule syntax (YARA-X will show errors during compilation)
# 3. Check for 99% compatibility issues with original YARA rules
```

## Contributing

We welcome contributions to Ramparts mcp scan. If you have suggestions, bug reports, or feature requests, please open an issue on our GitHub repository.

## Support

- **Issues**: [GitHub Issues](https://github.com/getjavelin/ramparts/issues)

## Documentation

- 📚 **[Security Features & Attack Vectors](docs/security-features.md)** - Detailed guide to all security vulnerabilities detected
- 📚 **[Complete API Documentation](docs/api.md)** - REST API reference with endpoints and examples  
- 🔧 **[Integration Patterns](docs/integration.md)** - CI/CD, Docker, Kubernetes, and monitoring examples
- ⚙️ **[Configuration Reference](docs/configuration.md)** - Complete configuration file documentation
- 📖 **[CLI Reference](docs/cli.md)** - All commands, options, and usage examples
- 🔍 **[Troubleshooting Guide](docs/troubleshooting.md)** - Solutions to common issues

## Additional Resources

- [MCP Protocol Documentation](https://modelcontextprotocol.io/)
- [Configuration Examples](examples/config_example.json)

