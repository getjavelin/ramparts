<div align="center">

# Ramparts: mcp (model context protocol) scanner

<img src="assets/ramparts.png" alt="Ramparts Banner" width="250" />

*A fast, lightweight security scanner for Model Context Protocol (MCP) servers with built-in vulnerability detection.*

[![Crates.io](https://img.shields.io/crates/v/ramparts)](https://crates.io/crates/ramparts)
[![GitHub stars](https://img.shields.io/github/stars/getjavelin/ramparts?style=social)](https://github.com/getjavelin/ramparts)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/github/actions/workflow/status/getjavelin/ramparts/pr-check.yml?label=tests)](https://github.com/getjavelin/ramparts/actions)
[![Clippy](https://img.shields.io/github/actions/workflow/status/getjavelin/rampart/pr-check.yml?label=lint)](https://github.com/getjavelin/ramparts/actions)
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
- **MCP Rug Pulls** - unauthorized changes to MCP tool descriptions after initial user approval.
- **Data exfiltration** - leaking sensitive information
- **Privilege escalation** - gaining unauthorized access
- **Path traversal attacks** - accessing files outside intended directories
- **Command injection** - executing unauthorized system commands
- **SQL injection** - manipulating database queries

### What Ramparts Does

Ramparts provides **security scanning** of MCP servers by:

1. **Discovering Capabilities**: Scans all MCP endpoints to identify available tools, resources, and prompts
2. **Static Analysis**: Performs rule-based checks for common vulnerabilities
3. **LLM-Powered Analysis**: Uses AI models to detect sophisticated security issues
4. **Risk Assessment**: Categorizes findings by severity and provides actionable recommendations

## Who Ramparts is For

Ramparts is designed for developers using local, remote MCP servers or building their own MCP servers and interested in scanning it for any vulnerabilities it may expose. Developers may use Ramparts locally to scan the MCP servers they use in their local development environment (e.g., Cursor, Windsurf, Claude Code etc.,). 

**If you're using MCP servers** - whether they're running locally on your machine or hosted remotely - Ramparts helps you understand what security risks they might pose. You can scan third-party MCP servers before connecting to them, or validate your own local MCP servers before deploying them to production.

**If you're building MCP servers** - whether you're creating tools, resources, or prompts - Ramparts gives you confidence that your implementation doesn't expose vulnerabilities to AI agents. It's especially useful for developers who want to ensure their MCP tools are secure by design.


## Why Rust?

The Ramparts mcp scanner is implemented in Rust to prioritize performance, reliability, and broad portability. Rust offers native execution speed with minimal memory overhead, making it well-suited for analyzing large prompt contexts, tool manifests, or server topologies—without the need for a heavyweight runtime. Ramparts was built with a view of operating in CI pipelines, agent sandboxes, or constrained edge environments which made the ability to compile to a single, compact binary essential.

## Features

- **MCP Server Scanning**: Scan MCP servers for tools, resources, and prompts
- **Security Analysis**: Built-in security scanning with LLM-based analysis
- **Optional YARA Integration**: Advanced pattern-based scanning with configurable YARA rules
- **Flexible Installation**: Install with or without YARA dependency based on your needs
- **Multiple Transport Support**: HTTP and STDIO transport mechanisms
- **Configuration Management**: Load settings from IDE configuration files and custom YAML configs
- **Comprehensive Output**: Multiple output formats (JSON, table, text)

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

## Prerequisites

### YARA Installation

Ramparts uses YARA rules for advanced pattern-based security scanning. YARA installation is **optional** but **recommended** for comprehensive security analysis.

#### Install YARA

**macOS (using Homebrew)**
```bash
brew install yara
```

**Ubuntu/Debian**
```bash
sudo apt update
sudo apt install yara
```

**CentOS/RHEL/Fedora**
```bash
# CentOS/RHEL
sudo yum install yara
# or Fedora
sudo dnf install yara
```

**Windows**
```bash
# Using vcpkg
vcpkg install yara

# Or download from: https://github.com/VirusTotal/yara/releases
```

**From Source**
```bash
git clone https://github.com/VirusTotal/yara.git
cd yara
./bootstrap.sh
./configure
make
sudo make install
```

#### Verify Installation
```bash
yara --version
```

#### Automated Setup Script

For easier setup, you can use our automated setup script:

```bash
# Make the script executable
chmod +x scripts/setup_yara.sh

# Run the setup script
./scripts/setup_yara.sh
```

This script will:
- Detect your operating system
- Install YARA via the appropriate package manager
- Set up environment variables for development
- Test the installation and compilation

#### Alternative: Install Without YARA

If you prefer to skip YARA installation, you can install Ramparts without YARA support:

```bash
cargo install ramparts --no-default-features
```

When YARA is disabled, you'll see a helpful message with installation instructions if needed:

```
📋 YARA Scanning Disabled

YARA rule scanning is enabled in your config but YARA is not available.
To enable YARA scanning, please:

1. Install YARA on your system:
   • macOS: brew install yara
   • Ubuntu/Debian: sudo apt install yara
   • CentOS/RHEL: sudo yum install yara

2. Reinstall ramparts:
   cargo install ramparts --force

3. Or disable YARA in your config.yaml:
   scanner:
     enable_yara: false

Continuing without YARA scanning...
```

## Quick Start

### Installation

**From crates.io (Recommended)**
```bash
# With YARA support (recommended - install YARA first)
cargo install ramparts

# Without YARA support (lighter installation)
cargo install ramparts --no-default-features
```

**From source**
```bash
git clone https://github.com/getjavelin/ramparts.git
cd ramparts

# With YARA support
cargo install --path .

# Without YARA support
cargo install --path . --no-default-features
```

> **Note**: If you have YARA installed but want to disable it temporarily, you can control this via configuration (see Configuration section below).

### Troubleshooting YARA Issues

If you encounter build errors related to YARA, try these solutions:

#### macOS Issues

**Error: `'yara.h' file not found`**
```bash
# Set environment variables manually
export YARA_LIBRARY_PATH="/opt/homebrew/lib"  # Apple Silicon
export YARA_LIBRARY_PATH="/usr/local/lib"     # Intel Mac
export BINDGEN_EXTRA_CLANG_ARGS="-I/opt/homebrew/include"  # Apple Silicon
export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/local/include"     # Intel Mac

# Then rebuild
cargo clean && cargo build
```

**Error: `dyld: Library not loaded`**
```bash
# Reinstall YARA and rebuild
brew uninstall yara && brew install yara
cargo clean && cargo build
```

#### Linux Issues

**Error: `yara.h: No such file or directory`**
```bash
# Install development headers
sudo apt-get install libyara-dev  # Ubuntu/Debian
sudo yum install yara-devel       # CentOS/RHEL

# Then rebuild
cargo clean && cargo build
```

#### General Issues

If you continue to have problems, you can temporarily disable YARA:
```bash
cargo install ramparts --no-default-features
```

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

Start Ramparts as a server for continuous monitoring:

```bash
ramparts server --port 8080
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

### Scan from IDE Configuration

Scan MCP servers configured in your IDE:

```bash
ramparts scan-config
```

## CLI Reference

### Basic Commands

```bash
# Scan an MCP server
ramparts scan <url> [options]

# Start Ramparts server mode
ramparts server [options]

# Scan from IDE configuration
ramparts scan-config

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
  -p, --port <PORT>               Server port [default: 8080]
  -h, --host <HOST>               Server host [default: 127.0.0.1]
  --config <FILE>                 Configuration file
```

## Configuration

Ramparts uses a `ramparts.yaml` configuration file for customizing security rules and thresholds:

### Initialize Configuration

Create a custom configuration file:

```bash
ramparts init-config
```

This creates a `ramparts.yaml` file:

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
  # YARA Configuration
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

### YARA Configuration

Ramparts supports flexible YARA configuration options:

#### Enable/Disable YARA
```yaml
scanner:
  enable_yara: true   # Enable YARA scanning (default)
  # enable_yara: false  # Disable YARA scanning
```

#### YARA Behavior Matrix

| YARA Installed | `enable_yara` | Result |
|---------------|---------------|---------|
| ✅ Yes | `true` | Full YARA scanning enabled |
| ✅ Yes | `false` | YARA scanning disabled (silent) |
| ❌ No | `true` | Shows installation message, continues without YARA |
| ❌ No | `false` | No YARA scanning, no messages |

#### YARA Rules Directory Structure

Ramparts automatically loads YARA rules from the `rules/` directory:

```
rules/
├── pre/           # Pre-scan rules (applied before LLM analysis)
│   ├── secrets_leakage.yarac
│   ├── command_injection.yarac
│   ├── path_traversal.yarac
│   └── sql_injection.yarac
└── post/          # Post-scan rules (applied after LLM analysis)
    └── (future enhancement)
```

#### Custom YARA Rules

To add custom YARA rules:

1. **Create rule source files** (`.yar` format)
2. **Compile to `.yarac`** using YARA compiler:
   ```bash
   yarac your_rule.yar your_rule.yarac
   ```
3. **Place in rules directory** (`rules/pre/` or `rules/post/`)
4. **Restart Ramparts** - rules are loaded automatically

#### YARA Rule Development

Example custom rule:
```yara
rule suspicious_eval_usage
{
    meta:
        description = "Detects potentially dangerous eval() usage"
        severity = "HIGH"
        tags = "code-injection"
    
    strings:
        $eval1 = "eval(" nocase
        $exec1 = "exec(" nocase
        $system1 = "system(" nocase
    
    condition:
        any of them
}
```

Compile and use:
```bash
yarac custom_rule.yar rules/pre/custom_rule.yarac
```

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

### YARA-Related Issues

**YARA Not Found During Installation**
```bash
# Error: failed to find YARA installation
# Solution: Install YARA first, then reinstall ramparts
brew install yara  # macOS
sudo apt install yara  # Ubuntu/Debian
cargo install ramparts --force
```

**YARA Rules Not Loading**
```bash
# Check if rules directory exists
ls -la rules/
ls -la rules/pre/

# Check YARA rule compilation
yarac --help
yarac rules/src/your_rule.yar rules/pre/your_rule.yarac
```

**YARA Compilation Errors**
```bash
# Error: cannot compile .yar files
# Solution: Check YARA syntax
yara rules/src/your_rule.yar /dev/null

# Common fixes:
# 1. Check rule syntax
# 2. Verify string escaping
# 3. Ensure proper rule structure
```

**Mixed YARA Versions**
```bash
# Error: YARA version mismatch
# Solution: Ensure consistent YARA version
yara --version
yarac --version

# Reinstall YARA if versions differ
brew reinstall yara  # macOS
sudo apt remove yara && sudo apt install yara  # Ubuntu
```

**Performance Issues with YARA**
```bash
# If YARA scanning is slow, you can:
# 1. Disable YARA temporarily
echo "scanner:
  enable_yara: false" > ramparts.yaml

# 2. Or reduce rule complexity
# 3. Or use --no-default-features installation
cargo install ramparts --no-default-features --force
```

**YARA Rules Directory Permissions**
```bash
# Error: Permission denied accessing rules
# Solution: Check directory permissions
chmod -R 755 rules/
chmod 644 rules/pre/*.yarac
```

**Custom Rules Not Working**
```bash
# Debug rule loading
# 1. Check file extension (.yarac not .yar)
ls rules/pre/*.yarac

# 2. Test rule compilation
yarac your_rule.yar test.yarac

# 3. Verify rule syntax
yara your_rule.yar test_file.txt
```

## Contributing

We welcome contributions to Ramparts mcp scan. If you have suggestions, bug reports, or feature requests, please open an issue on our GitHub repository.

## Support

- **Issues**: [GitHub Issues](https://github.com/getjavelin/ramparts/issues)

## Additional Resources

- [MCP Protocol Documentation](https://modelcontextprotocol.io/)
- [Configuration Examples](examples/config_example.json)

## Advanced YARA Integration

Ramparts includes sophisticated YARA rule integration for advanced security pattern detection. YARA support is **optional** and can be configured based on your security requirements.

### YARA Feature Overview

- **🔧 Configurable**: Enable/disable YARA via config file
- **📁 Auto-loading**: Automatically loads all `.yarac` files from rules directory
- **⚡ Performance**: Pre-compiled rules for fast pattern matching
- **🎯 Phased scanning**: Pre-scan and post-scan rule execution
- **🛠 Extensible**: Easy addition of custom security rules

### Default Security Rules

Ramparts includes built-in YARA rules for common vulnerabilities:

| Rule File | Purpose | Detects |
|-----------|---------|---------|
| `secrets_leakage.yarac` | API keys, tokens, passwords | Hardcoded credentials, API keys |
| `command_injection.yarac` | System command execution | Dangerous command patterns |
| `path_traversal.yarac` | Directory traversal attacks | `../`, absolute path patterns |
| `sql_injection.yarac` | Database query manipulation | SQL injection patterns |

### Directory Structure
```
rules/
├── pre/           # Pre-scan rules (applied before LLM analysis)
│   ├── secrets_leakage.yarac
│   ├── command_injection.yarac
│   ├── path_traversal.yarac
│   └── sql_injection.yarac
└── post/          # Post-scan rules (applied after LLM analysis)
    └── (reserved for future enhancements)
```

### Rule Compilation Workflow

```bash
# 1. Create YARA source files (.yar)
echo 'rule test_rule { strings: $a = "test" condition: $a }' > test.yar

# 2. Compile to binary format (.yarac)
yarac test.yar test.yarac

# 3. Place in rules directory
mv test.yarac rules/pre/

# 4. Ramparts automatically loads on next scan
```

### Performance Considerations

- **Pre-compiled rules**: `.yarac` files load faster than `.yar` source
- **Memory efficient**: Rules are loaded once and reused
- **Configurable**: Disable YARA if not needed to reduce resource usage
- **Graceful fallback**: Continues operation if YARA unavailable

For detailed YARA rule development, see the [YARA Documentation](https://yara.readthedocs.io/).
