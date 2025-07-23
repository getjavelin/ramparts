<div align="center">

# Rampart: mcp (model context protocol) scanner

<img src="assets/rampart.png" alt="Rampart Banner" width="250" />

*A fast, lightweight security scanner for Model Context Protocol (MCP) servers with built-in vulnerability detection.*

[![Crates.io](https://img.shields.io/crates/v/rampart)](https://crates.io/crates/rampart)
[![GitHub stars](https://img.shields.io/github/stars/getjavelin/rampart?style=social)](https://github.com/getjavelin/rampart)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/github/actions/workflow/status/getjavelin/rampart/ci.yml?label=tests)](https://github.com/getjavelin/rampart/actions)
[![Clippy](https://img.shields.io/github/actions/workflow/status/getjavelin/rampart/ci.yml?label=lint)](https://github.com/getjavelin/rampart/actions)
[![Release](https://img.shields.io/github/release/getjavelin/rampart)](https://github.com/getjavelin/rampart/releases)

</div>

## Overview

**Rampart** is a scanner designed for the **Model Context Protocol (MCP)** ecosystem. As AI agents and LLMs increasingly rely on external tools and resources through MCP servers, ensuring the security of these connections has become critical.   

The Model Context Protocol (MCP) is an open standard that enables AI assistants to securely connect to external data sources and tools. It allows AI agents to access databases, file systems, and APIs through toolcalling to retrieve real-time information and interact with external or internal services.

Rampart is under active development.

### The Security Challenge

MCP servers can expose powerful capabilities to AI agents, including:
- **File system access** (read/write files, directory traversal)
- **Database operations** (SQL queries, data manipulation)
- **API integrations** (external/internal/local service calls, authentication)
- **System commands** (process execution, system administration)

Without proper security analysis, these capabilities can become attack vectors for:
- **Path traversal attacks** - accessing files outside intended directories
- **Command injection** - executing unauthorized system commands
- **SQL injection** - manipulating database queries
- **Prompt injection** - bypassing AI safety measures
- **Data exfiltration** - leaking sensitive information
- **Privilege escalation** - gaining unauthorized access

### What Rampart Does

Rampart provides **security scanning** of MCP servers by:

1. **Discovering Capabilities**: Scans all MCP endpoints to identify available tools, resources, and prompts
2. **Static Analysis**: Performs rule-based checks for common vulnerabilities
3. **LLM-Powered Analysis**: Uses AI models to detect sophisticated security issues
4. **Risk Assessment**: Categorizes findings by severity and provides actionable recommendations

## Who Rampart is For

Rampart is designed for developers using local, remote MCP servers or building their own MCP servers and interested in scanning it for any vulnerabilities it may expose. Developers may use Rampart locally to scan the MCP servers they use in their local development environment (e.g., Cursor, Windsurf, Claude Code etc.,). 

**If you're using MCP servers** - whether they're running locally on your machine or hosted remotely - Rampart helps you understand what security risks they might pose. You can scan third-party MCP servers before connecting to them, or validate your own local MCP servers before deploying them to production.

**If you're building MCP servers** - whether you're creating tools, resources, or prompts - Rampart gives you confidence that your implementation doesn't expose vulnerabilities to AI agents. It's especially useful for developers who want to ensure their MCP tools are secure by design.


## Why Rust?

The Rampart mcp scanner is implemented in Rust to prioritize performance, reliability, and broad portability. Rust offers native execution speed with minimal memory overhead, making it well-suited for analyzing large prompt contexts, tool manifests, or server topologies—without the need for a heavyweight runtime. Rampart was built with a view of operating in CI pipelines, agent sandboxes, or constrained edge environments which made the ability to compile to a single, compact binary essential.

## Key Features

**Coverage**: Analyzes all MCP endpoints (server/info, tools/list, resources/list, prompts/list) and evaluates each tool, resource, and prompt 

**Detection**: Detects path traversal, command injection, SQL injection, prompt injection, secret leakage, auth bypass, and more—using both static checks and LLM-assisted analysis  

**Performance**: Built in Rust for fast, efficient scanning of large MCP servers  

**Output**: Choose from tree-style text, JSON, or raw formats for easy integration with scripts and dashboards  

**Modular & Extensible**: Add custom rules or tweak severity thresholds via a simple configuration file  

## Use Cases

- **Security Audits**: Comprehensive assessment of MCP server security posture
- **Development**: Testing MCP servers during development and testing phases
- **Compliance**: Meeting security requirements for AI agent deployments

## Quick Start

### Installation

**From crates.io (Recommended)**
```bash
cargo install rampart
```

**From source**
```bash
git clone https://github.com/getjavelin/rampart.git
cd rampart
cargo install --path .
```

### Basic Usage

**Scan an MCP server**
```bash
rampart scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"
```

**Scan with custom output format**
```bash
rampart scan <url> --output json
rampart scan <url> --output raw
```

**Scan with verbose output**
```bash
rampart scan <url> --verbose
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

Summary:
  • Tools scanned: 74
  • Warnings found: 2 tools with 2 total warnings
================================================================================
```

## Examples

### Scan Different MCP Servers

**GitHub Copilot**
```bash
rampart scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"
```

**Local MCP server**
```bash
rampart scan http://localhost:3000/mcp/
```

**Custom MCP server with API key**
```bash
rampart scan https://api.example.com/mcp/ --auth-headers "X-API-Key: $API_KEY"
```

**With custom timeout**
```bash
rampart scan <url> --timeout 60
```

### Advanced Scanning Options

**Scan with custom severity threshold**
```bash
rampart scan <url> --min-severity HIGH
```

**Scan with specific output format**
```bash
rampart scan <url> --output json --pretty
```

**Scan with custom configuration**
```bash
rampart scan <url> --config custom-rampart.yaml
```

## Advanced Usage

### Server Mode

Start Rampart as a server for continuous monitoring:

```bash
rampart server --port 8080
```

### Batch Scanning

Scan multiple servers from a file:

```bash
# Create a servers list
echo "https://server1.com/mcp/
https://server2.com/mcp/
https://server3.com/mcp/" > servers.txt

# Run batch scan
rampart scan --batch servers.txt
```

### Scan from IDE Configuration

Scan MCP servers configured in your IDE:

```bash
rampart scan-config
```

## CLI Reference

### Basic Commands

```bash
# Scan an MCP server
rampart scan <url> [options]

# Start Rampart server mode
rampart server [options]

# Scan from IDE configuration
rampart scan-config

# Initialize configuration file
rampart init-config

# Show help
rampart --help
rampart scan --help
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
  -p, --port <PORT>               Server port [default: 8080]
  -h, --host <HOST>               Server host [default: 127.0.0.1]
  --config <FILE>                 Configuration file
```

## Configuration

Rampart uses a `rampart.yaml` configuration file for customizing security rules and thresholds:

### Initialize Configuration

Create a custom configuration file:

```bash
rampart init-config
```

This creates a `rampart.yaml` file:

```yaml
# Example rampart.yaml
security:
  # LLM analysis settings
  llm_batch_size: 10
  llm_timeout: 30
  
  # Severity thresholds
  min_severity: "MEDIUM"
  
  # Custom security rules
  custom_rules:
    - name: "custom_path_check"
      pattern: "path.*\\.\\./"
      severity: "HIGH"
      description: "Custom path traversal detection"
  
  # Vulnerability detection settings
  vulnerability_detection:
    enable_path_traversal: true
    enable_command_injection: true
    enable_sql_injection: true
    enable_prompt_injection: true
    enable_secret_leakage: true
    enable_auth_bypass: true

# Output settings
output:
  format: "text"  # text, json, raw
  pretty_print: false
  include_details: true
  include_recommendations: true

# Performance settings
performance:
  max_concurrent_requests: 10
  request_timeout: 30
  retry_attempts: 3
```

## Output Formats

### Text Format (Default)
```bash
rampart scan <url>
```

### JSON Format
```bash
rampart scan <url> --output json
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
rampart scan <url> --output raw
```

## Troubleshooting

### Common Issues

**Connection Timeout**
```bash
# Increase timeout
rampart scan <url> --timeout 60
```

**Authentication Errors**
```bash
# Check your auth headers format
rampart scan <url> --auth-headers "Authorization: Bearer $TOKEN"
```

**Permission Denied**
```bash
# Check file permissions
chmod +x $(which rampart)
```

**Configuration File Not Found**
```bash
# Initialize configuration
rampart init-config
```

## Contributing

We welcome contributions to Rampart mcp scan. If you have suggestions, bug reports, or feature requests, please open an issue on our GitHub repository.

## Support

- **Issues**: [GitHub Issues](https://github.com/getjavelin/rampart/issues)

## Additional Resources

- [MCP Protocol Documentation](https://modelcontextprotocol.io/)
- [Configuration Examples](examples/config_example.json)
