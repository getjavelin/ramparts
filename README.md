<div align="center">

# Ramparts: MCP Security Scanner

<img src="assets/ramparts.png" alt="Ramparts Banner" width="250" />

*A fast, lightweight security scanner for Model Context Protocol (MCP) servers with built-in vulnerability detection.*

[![Crates.io](https://img.shields.io/crates/v/ramparts)](https://crates.io/crates/ramparts)
[![GitHub stars](https://img.shields.io/github/stars/getjavelin/ramparts?style=social)](https://github.com/getjavelin/ramparts)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/github/actions/workflow/status/getjavelin/ramparts/pr-check.yml?label=tests)](https://github.com/getjavelin/ramparts/actions)

</div>

## Overview

**Ramparts** scans Model Context Protocol (MCP) servers for security vulnerabilities. As AI agents increasingly rely on MCP servers for external tools and data access, ensuring these connections are secure has become critical.

MCP servers can expose powerful capabilities like file system access, database operations, API integrations, and system commands. Without proper security analysis, these become attack vectors for tool poisoning, data exfiltration, privilege escalation, and injection attacks.

Read our [launch blog](https://www.getjavelin.com/blogs/ramparts-mcp-scan) for more details.

## Key Features

- **Comprehensive MCP Coverage** - Analyzes all MCP endpoints and evaluates tools, resources, and prompts
- **Scan AI Coding Tools** - Analyzes all MCP servers used in AI Based Coding tools in local environment
- **Advanced Security Detection** - Detects path traversal, command injection, SQL injection, secret leakage, and cross-origin escalation
- **YARA-X Integration** - Optional pattern-based scanning with configurable rules
- **High Performance** - Built in Rust for fast, efficient scanning with minimal overhead
- **Rich Output Formats** - Text, JSON, and raw formats for easy integration
- **LLM-Powered Analysis** - Uses AI models to detect sophisticated security issues

## Quick Start

```bash
cargo install ramparts
```

## Quick Start

**Scan MCP servers used in my local IDE (Cursor, Windsurf, Claude Code, etc.,):**
```bash
ramparts scan-config
```

**Scan a single MCP server:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"
```

**Basic local MCP server scan:**
```bash
ramparts scan http://localhost:3000/mcp/
```

**Output formats:**
```bash
ramparts scan <url> --output json --pretty
ramparts scan <url> --verbose
```

## Example Output

```
================================================================================
MCP Server Scan Result
================================================================================
URL: https://api.githubcopilot.com/mcp/
Status: Success

Server Information:
  Name: GitHub Copilot MCP Server
  Version: 1.0.0
  Tools: 74

Security Assessment Results
================================================================================
🌐 GitHub Copilot MCP Server
  └── create_or_update_file warning
      ├── HIGH: Potential Path Traversal Vulnerability
      │   Details: Tool accepts 'path' parameter without proper validation

YARA Scan Results
================================================================================
⚠️ domain-analysis (HIGH)
  Rule: CrossDomainContamination
  Matched: Cross-domain contamination across 2 domains: api.github.com, webhooks.github.com

Summary: 74 tools scanned, 2 warnings found
================================================================================
```

## Who Should Use Ramparts

- **Developers Using AI Coding Tools** - Scan your Cursor, Windsurf, Claude Code MCP configurations for security threats
- **MCP Server Users** - Scan third-party servers before connecting or validate local servers before production
- **MCP Server Developers** - Ensure your implementations don't expose vulnerabilities to AI agents
- **Security Teams** - Comprehensive assessment of MCP server security posture in AI agent deployments

## Documentation

- 📖 [Security Features & Threat Model](docs/security-features.md)
- 🚀 [Installation Guide](docs/installation.md) 
- 💻 [Usage & CLI Reference](docs/usage.md)
- ⚙️ [Configuration](docs/configuration.md)
- 🔧 [Troubleshooting](docs/troubleshooting.md)

## Use Cases

- **Security Audits** - Comprehensive MCP server security assessment
- **Development** - Testing during development and CI/CD pipelines  
- **Compliance** - Meeting security requirements for AI agent deployments

## Contributing

We welcome contributions! Please open an issue on our [GitHub repository](https://github.com/getjavelin/ramparts/issues) for bug reports, feature requests, or suggestions.

## Additional Resources

- [Model Context Protocol Documentation](https://modelcontextprotocol.io/)
- [Javelin AI Security Blog](https://www.getjavelin.com/blogs)

