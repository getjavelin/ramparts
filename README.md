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

### What Ramparts Does

Ramparts provides **security scanning** of MCP servers by:

1. **Discovering Capabilities**: Scans all MCP endpoints to identify available tools, resources, and prompts
2. **Static Analysis**: Performs yara-based checks for common vulnerabilities
3. **Cross-Origin Analysis**: Detects when tools span multiple domains, which could enable context hijacking or injection attacks
4. **LLM-Powered Analysis**: Uses AI models to detect sophisticated security issues
5. **Risk Assessment**: Categorizes findings by severity and provides actionable recommendations

### Cross-Origin Escalation Detection

Ramparts includes specialized detection for **Cross-Origin Escalation** attacks, which occur when MCP tools access resources across multiple domains. This creates opportunities for:

- **Context Hijacking**: One domain injecting malicious content that affects tools on other domains
- **Domain Contamination**: Tools from untrusted domains mixing with trusted ones
- **Mixed Security Schemes**: HTTP and HTTPS tools creating security vulnerabilities

The scanner performs comprehensive URL analysis across all tools and resources to:
- Extract and normalize domains from tool parameters, schemas, and metadata
- Identify cross-domain contamination patterns
- Flag outlier tools using different domains than the majority
- Detect mixed HTTP/HTTPS schemes that could compromise security

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

## Installation

### YARA-X Integration (Optional)

Ramparts uses YARA-X, a modern rewrite of YARA in Rust, for advanced pattern-based security scanning. YARA-X integration is **optional** but **recommended** for comprehensive security analysis.

#### Key Benefits of YARA-X

- **Pure Rust**: No system dependencies required - everything is handled at compile time
- **Better Performance**: Optimized for complex security rules and mixed rule sets
- **Memory Safe**: Built with Rust's safety guarantees

#### Installation Options

**From crates.io (Recommended)**
```bash
# With YARA-X support (recommended)
cargo install ramparts

# Without YARA-X support (lighter installation)
cargo install ramparts --no-default-features
```

**From source**
```bash
git clone https://github.com/getjavelin/ramparts.git
cd ramparts

# With YARA-X support
cargo install --path .

# Without YARA-X support
cargo install --path . --no-default-features
```

> **Note**: You can disable YARA-X scanning temporarily via configuration (see Configuration section below).

## Quick Start

### Basic Usage

**Scan an MCP server**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"
```

**Scan with custom output format**
```bash
ramparts scan <url> --output json
ramparts scan <url> --output raw
```

**Scan with verbose output**
```bash
ramparts scan <url> --verbose
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

Start Ramparts as a REST API server for continuous monitoring and integration with other systems:

```bash
# Start server on default port 3000
ramparts server

# Start server on custom port and host
ramparts server --port 8080 --host 0.0.0.0
```

Once started, the server provides a REST API with the following endpoints:

## API Endpoints

### 1. Health Check
**GET /health**

Check server health and protocol information.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00.000Z",
  "service": "ramparts-server",
  "version": "0.2.0",
  "protocol_version": "2025-06-18"
}
```

### 2. Protocol Information
**GET /protocol**

Get detailed MCP protocol information and supported capabilities.

**Response:**
```json
{
  "protocol": {
    "version": "2025-06-18",
    "name": "Model Context Protocol",
    "transport": {
      "stdio": "supported",
      "http": "supported",
      "features": [
        "JSON-RPC 2.0",
        "Session Management",
        "Protocol Version Headers",
        "STDIO Process Communication",
        "Multi-Transport Support"
      ]
    },
    "capabilities": [
      "tools/list",
      "resources/list",
      "prompts/list",
      "server/info"
    ]
  },
  "server": {
    "version": "0.2.0",
    "stdio_support": true,
    "mcp_compliance": "2025-06-18"
  }
}
```

### 3. API Documentation
**GET /**

Get interactive API documentation with examples.

**Response:**
```json
{
  "service": "Ramparts Microservice",
  "version": "0.2.0",
  "protocol_version": "2025-06-18",
  "endpoints": {
    "GET /health": "Health check with protocol info",
    "GET /protocol": "MCP protocol information",
    "POST /scan": "Scan a single MCP server",
    "POST /validate": "Validate scan configuration",
    "POST /batch-scan": "Scan multiple MCP servers",
    "GET /": "API documentation"
  },
  "transports": {
    "http": {
      "supported": true,
      "description": "HTTP/HTTPS transport for remote MCP servers",
      "examples": [
        "http://localhost:3000",
        "https://api.example.com/mcp"
      ]
    },
    "stdio": {
      "supported": true,
      "description": "STDIO transport for local MCP server processes",
      "examples": [
        "stdio:///usr/local/bin/mcp-server",
        "stdio://node /path/to/mcp-server.js"
      ]
    }
  }
}
```

### 4. Single Server Scan
**POST /scan**

Scan a single MCP server for security vulnerabilities.

**Request Body:**
```json
{
  "url": "https://api.example.com/mcp/",
  "timeout": 180,
  "http_timeout": 30,
  "detailed": true,
  "format": "json",
  "auth_headers": {
    "Authorization": "Bearer your-token-here",
    "X-API-Key": "your-api-key"
  }
}
```

**Request Fields:**
- `url` (required): MCP server URL or STDIO command
- `timeout` (optional): Total scan timeout in seconds (1-3600, default: 60)
- `http_timeout` (optional): HTTP request timeout in seconds (default: 30)
- `detailed` (optional): Enable detailed analysis (default: false)
- `format` (optional): Output format - "json", "table", "text", "raw" (default: "table")
- `auth_headers` (optional): Authentication headers as key-value pairs

**STDIO Examples:**
```json
{
  "url": "stdio:///usr/local/bin/mcp-server",
  "timeout": 180,
  "detailed": true,
  "format": "json"
}
```

```json
{
  "url": "node /path/to/mcp-server.js --config config.json",
  "timeout": 180,
  "detailed": true
}
```

**Success Response (200):**
```json
{
  "success": true,
  "result": {
    "url": "https://api.example.com/mcp/",
    "status": "Success",
    "timestamp": "2024-01-01T12:00:00.000Z",
    "response_time_ms": 1234,
    "server_info": {
      "name": "Example MCP Server",
      "version": "1.0.0",
      "description": "Example server description",
      "capabilities": ["tools", "resources", "prompts"],
      "metadata": {
        "transport": "http"
      }
    },
    "tools": [
      {
        "name": "create_file",
        "description": "Create a new file",
        "input_schema": {
          "type": "object",
          "properties": {
            "path": { "type": "string" },
            "content": { "type": "string" }
          }
        },
        "parameters": {},
        "category": "file_system",
        "tags": ["write", "create"],
        "deprecated": false
      }
    ],
    "resources": [],
    "prompts": [],
    "security_issues": {
      "total_issues": 1,
      "critical_count": 0,
      "high_count": 1,
      "medium_count": 0,
      "low_count": 0,
      "tool_issues": [
        {
          "tool_name": "create_file",
          "severity": "HIGH",
          "issue_type": "path_traversal",
          "message": "Tool allows potential path traversal attacks",
          "details": "The 'path' parameter lacks proper validation"
        }
      ],
      "prompt_issues": [],
      "resource_issues": []
    },
    "yara_results": [
      {
        "target_type": "summary",
        "target_name": "pre-scan",
        "rule_name": "PreScanSummary",
        "context": "Pre-scan completed: 2 rules executed on 1 items",
        "total_items_scanned": 1,
        "total_matches": 1,
        "rules_executed": ["secrets_leakage", "path_traversal"],
        "security_issues_detected": ["path_traversal:PathTraversalVulnerability"],
        "status": "warning"
      }
    ],
    "errors": []
  },
  "error": null,
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**Error Response (400):**
```json
{
  "success": false,
  "error": "URL is required",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

### 5. Configuration Validation
**POST /validate**

Validate scan configuration without performing actual scan.

**Request Body:**
```json
{
  "url": "https://api.example.com/mcp/",
  "timeout": 60,
  "http_timeout": 30,
  "detailed": false,
  "format": "json"
}
```

**Success Response (200):**
```json
{
  "success": true,
  "valid": true,
  "error": null,
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**Validation Error Response (400):**
```json
{
  "success": false,
  "valid": false,
  "error": "Timeout must be between 1 and 3600 seconds",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

### 6. Batch Scan
**POST /batch-scan**

Scan multiple MCP servers with shared configuration.

**Request Body:**
```json
{
  "urls": [
    "https://api.example1.com/mcp/",
    "https://api.example2.com/mcp/",
    "stdio:///usr/local/bin/mcp-server"
  ],
  "options": {
    "url": "",
    "timeout": 180,
    "http_timeout": 30,
    "detailed": true,
    "format": "json",
    "auth_headers": {
      "Authorization": "Bearer shared-token"
    }
  }
}
```

**Request Fields:**
- `urls` (required): Array of MCP server URLs or STDIO commands
- `options` (optional): Shared scan configuration. Note: the `url` field in options is ignored and overridden by each URL in the `urls` array

**Response (200):**
```json
{
  "success": true,
  "results": [
    {
      "success": true,
      "result": {
        "url": "https://api.example1.com/mcp/",
        "status": "Success",
        "timestamp": "2024-01-01T12:00:00.000Z",
        "response_time_ms": 1234,
        "server_info": { /* Server info object */ },
        "tools": [ /* Tools array */ ],
        "resources": [],
        "prompts": [],
        "security_issues": { /* Security issues object */ },
        "yara_results": [ /* YARA results array */ ],
        "errors": []
      },
      "error": null,
      "timestamp": "2024-01-01T12:00:00.000Z"
    },
    {
      "success": true,
      "result": {
        "url": "https://api.example2.com/mcp/",
        "status": { "Failed": "Failed to initialize MCP session with any protocol version" },
        "timestamp": "2024-01-01T12:00:01.000Z", 
        "response_time_ms": 0,
        "server_info": null,
        "tools": [],
        "resources": [],
        "prompts": [],
        "security_issues": null,
        "yara_results": [],
        "errors": ["Scan operation failed: Failed to initialize MCP session with any protocol version"]
      },
      "error": null,
      "timestamp": "2024-01-01T12:00:01.000Z"
    }
  ],
  "total": 3,
  "successful": 2,
  "failed": 1,
  "timestamp": "2024-01-01T12:00:02.000Z"
}
```

## Using the API

### Example: Basic Health Check
```bash
curl -X GET http://localhost:3000/health
```

### Example: Single Server Scan
```bash
curl -X POST http://localhost:3000/scan \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://api.example.com/mcp/",
    "timeout": 60,
    "detailed": true,
    "format": "json",
    "auth_headers": {
      "Authorization": "Bearer your-token"
    }
  }'
```

### Example: STDIO Server Scan
```bash
curl -X POST http://localhost:3000/scan \
  -H "Content-Type: application/json" \
  -d '{
    "url": "stdio:///usr/local/bin/my-mcp-server",
    "timeout": 120,
    "detailed": true
  }'
```

### Example: Batch Scan
```bash
curl -X POST http://localhost:3000/batch-scan \
  -H "Content-Type: application/json" \
  -d '{
    "urls": [
      "https://server1.example.com/mcp/",
      "https://server2.example.com/mcp/"
    ],
    "options": {
      "url": "",
      "timeout": 60,
      "detailed": false,
      "format": "json"
    }
  }'
```

### Example: Configuration Validation
```bash
curl -X POST http://localhost:3000/validate \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://api.example.com/mcp/",
    "timeout": 60,
    "http_timeout": 30
  }'
```

## Error Handling

The API uses standard HTTP status codes and provides detailed error messages:

### Common Error Responses

**400 Bad Request - Invalid URL:**
```json
{
  "success": false,
  "error": "URL is required",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**400 Bad Request - Empty URLs Array (Batch Scan):**
```json
{
  "success": false,
  "error": "URLs array is required",  
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**400 Bad Request - Invalid Timeout:**
```json
{
  "success": false,
  "error": "Timeout must be between 1 and 3600 seconds",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**400 Bad Request - Unsupported Protocol:**
```json
{
  "success": false,
  "error": "HTTP URL must start with http:// or https://",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**400 Bad Request - Scan Failed:**
```json
{
  "success": false,
  "error": "Connection timeout after 30 seconds",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

**500 Internal Server Error:**
```json
{
  "success": false,
  "error": "Internal server error occurred",
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

### CORS Support

The server includes CORS headers for browser-based applications:
- `Access-Control-Allow-Origin: *`
- `Access-Control-Allow-Methods: GET, POST, OPTIONS`
- `Access-Control-Allow-Headers: *`

### Request Validation

All endpoints validate input parameters:
- **URL validation**: Checks for proper format and supported protocols
- **Timeout validation**: Ensures values are within acceptable ranges (1-3600 seconds)
- **JSON validation**: Validates request body structure and required fields
- **Auth headers**: Validates header format when provided

## Integration Patterns

### CI/CD Pipeline Integration
```yaml
# GitHub Actions example
- name: Scan MCP Server
  run: |
    ramparts server --port 3000 &
    SERVER_PID=$!
    sleep 5  # Wait for server to start
    
    # Scan your MCP server
    curl -X POST http://localhost:3000/scan \
      -H "Content-Type: application/json" \
      -d '{"url": "${{ secrets.MCP_SERVER_URL }}", "detailed": true}' \
      | jq '.result.security_issues.total_issues'
    
    kill $SERVER_PID
```

### Docker Deployment
```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y curl
COPY ramparts /usr/local/bin/ramparts
EXPOSE 3000
CMD ["ramparts", "server", "--port", "3000", "--host", "0.0.0.0"]
```

### Health Monitoring
```bash
#!/bin/bash
# Health check script
response=$(curl -s http://localhost:3000/health)
if echo "$response" | jq -e '.status == "healthy"' > /dev/null; then
  echo "Ramparts server is healthy"
  exit 0
else
  echo "Ramparts server is unhealthy"
  exit 1
fi
```

### Load Balancer Configuration
```nginx
# Nginx configuration
upstream ramparts {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    server 127.0.0.1:3002;
}

server {
    listen 80;
    location / {
        proxy_pass http://ramparts;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

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

## Additional Resources

- [MCP Protocol Documentation](https://modelcontextprotocol.io/)
- [Configuration Examples](examples/config_example.json)

