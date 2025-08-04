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

MCP servers expose powerful capabilities—file systems, databases, APIs, and system commands—that can become attack vectors like tool poisoning, command injection, and data exfiltration without proper security analysis. - 📚 **[Security Features & Attack Vectors](docs/security-features.md)** 



### What Ramparts Does

Ramparts provides **security scanning** of MCP servers by:

1. **Discovering Capabilities**: Scans all MCP endpoints to identify available tools, resources, and prompts
2. **Static Analysis**: Performs yara-based checks for common vulnerabilities
3. **Cross-Origin Analysis**: Detects when tools span multiple domains, which could enable context hijacking or injection attacks
4. **LLM-Powered Analysis**: Uses AI models to detect sophisticated security issues
5. **Risk Assessment**: Categorizes findings by severity and provides actionable recommendations
>
> **💡 Jump directly to detailed Rampart features?**
> [**📚 Detailed Features**](docs/features.md)

## Who Ramparts is For

- **Developers**: Scan MCP servers for vulnerabilities in your development environment (Cursor, Windsurf, Claude Code) or production deployments.  
- **MCP users**: Scan third-party servers before connecting, validate local servers before production.  
- **MCP developers**: Ensure your tools, resources, and prompts don't expose vulnerabilities to AI agents.

## Use Cases

- **Security Audits**: Comprehensive assessment of MCP server security posture
- **Development**: Testing MCP servers during development and testing phases  
- **CI/CD Integration**: Automated security scanning in deployment pipelines
- **Compliance**: Meeting security requirements for AI agent deployments

> **💡 Caution**: Ramparts analyzes MCP server metadata and static configurations. For comprehensive security, combine with runtime MCP guardrails and adopt a layered security approach. The MCP threat landscape is rapidly evolving, and rampart is not perfect and inaccuracies are inevitable.

## Quick Start

**Installation**
```bash
cargo install ramparts
```

**Scan an MCP server**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"

# Generate detailed markdown report (scan_YYYYMMDD_HHMMSS.md)
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN" --report
```

**Scan your IDE's MCP configurations**
```bash
# Automatically discovers and scans MCP servers from Cursor, Windsurf, VS Code, Claude Desktop, Claude Code
ramparts scan-config

# With detailed report generation
ramparts scan-config --report
```

> **💡 Did you know you can start Ramparts as a server?** Run `ramparts server` to get a REST API for continuous monitoring and CI/CD integration. See 📚 **[Ramparts Server Mode](docs/api.md)** 

## Example Output

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

## Contributing

We welcome contributions to Ramparts mcp scan. If you have suggestions, bug reports, or feature requests, please open an issue on our GitHub repository.

## Documentation
- 🔍 **[Troubleshooting Guide](docs/troubleshooting.md)** - Solutions to common issues
- ⚙️ **[Configuration Reference](docs/configuration.md)** - Complete configuration file documentation
- 📖 **[CLI Reference](docs/cli.md)** - All commands, options, and usage examples

## Additional Resources
- [Need Support?](https://github.com/getjavelin/ramparts/issues)
- [MCP Protocol Documentation](https://modelcontextprotocol.io/)
- [Configuration Examples](examples/config_example.json)

