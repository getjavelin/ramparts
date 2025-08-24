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
2. **Multi-Transport Support**: Supports HTTP, SSE, stdio, and subprocess transports with intelligent fallback
3. **Session Management**: Handles stateful MCP servers with automatic session ID management
4. **Static Analysis**: Performs yara-based checks for common vulnerabilities
5. **Cross-Origin Analysis**: Detects when tools span multiple domains, which could enable context hijacking or injection attacks
6. **LLM-Powered Analysis**: Uses AI models to detect sophisticated security issues
7. **Risk Assessment**: Categorizes findings by severity and provides actionable recommendations
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

Quick install (one-line):
```bash
curl -sSL https://raw.githubusercontent.com/getjavelin/ramparts/main/scripts/install.sh | bash
```

Or via Cargo:
```bash
cargo install ramparts
```

Or via Docker:
```bash
export JAVELIN_API_KEY="your-api-key"
docker run -d -p 8080:8080 -e JAVELIN_API_KEY="$JAVELIN_API_KEY" getjavelin/ramparts:latest proxy 0.0.0.0:8080
```

📦 **[Complete Installation Guide](INSTALL.md)** - All methods, platforms, and configurations

**Scan an MCP server**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $TOKEN"

# Generate detailed markdown report (scan_YYYYMMDD_HHMMSS.md)
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $TOKEN" --report

# Scan stdio/subprocess MCP servers
ramparts scan "stdio:npx:mcp-server-commands"
ramparts scan "stdio:python3:/path/to/mcp_server.py"
```

**Scan your IDE's MCP configurations**
```bash
# Automatically discovers and scans MCP servers from Cursor, Windsurf, VS Code, Claude Desktop, Claude Code
ramparts scan-config

# With detailed report generation
ramparts scan-config --report
```

**Start Security-First AI Gateway**
```bash
# Start Ramparts AI Gateway (requires Javelin API key)
export LLM_API_KEY="your-api-key"
ramparts proxy 127.0.0.1:8080
```

> **🚀 Ramparts AI Gateway** - The first **security-first AI gateway** built specifically for MCP. Unlike generic solutions like Nexus, LiteLLM, or Cloudflare AI Gateway, Ramparts understands MCP tools and provides enterprise-grade security validation.
>
> **💡 Traditional Server Mode** Run `ramparts server` to get a REST API for continuous monitoring and CI/CD integration. See 📚 **[Ramparts Server Mode](docs/api.md)**
>
> **🔒 Security-First AI Gateway** The `ramparts proxy` command provides the most advanced MCP security available, powered by Javelin Guardrails. See 📚 **[AI Gateway Documentation](docs/proxy.md)**

### Run as an MCP server (stdio)

```bash
ramparts mcp-stdio
```

When publishing to Docker MCP Toolkit, configure the container command to `ramparts mcp-stdio` so the toolkit connects via stdio. Use `MCP-Dockerfile` to make this the default.

## Example Output

**Single server scan:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $TOKEN"
```

```
RAMPARTS
MCP Security Scanner

Version: 0.7.0
Current Time: 2025-08-04 07:32:19 UTC
Git Commit: 9d0c37c

🌐 GitHub Copilot MCP Server
  ✅ All tools passed security checks

  └── push_files ✅ passed
  └── create_or_update_file ⚠️ 2 warnings
      │   └── 🟠 HIGH (LLM): Tool allowing directory traversal attacks
      │   └── 🟠 HIGH (YARA): EnvironmentVariableLeakage
  └── get_secret_scanning_alert ⚠️ 1 warning
      │   └── 🟠 HIGH (YARA): EnvironmentVariableLeakage

Summary:
  • Tools scanned: 83
  • Security issues: 3 findings
```

**IDE configuration scan:**
```bash
ramparts scan-config --report
```

```
🔍 Found 3 IDE config files:
  ✓ vscode IDE: /Users/user/.vscode/mcp.json
  ✓ claude IDE: /Users/user/Library/Application Support/Claude/claude_desktop_config.json
  ✓ cursor IDE: /Users/user/.cursor/mcp.json

📁 vscode IDE config: /Users/user/.vscode/mcp.json (2 servers)
  └─ github-copilot [HTTP]: https://api.githubcopilot.com/mcp/
  └─ local-tools [STDIO]: stdio:python[local-mcp-server]

🌍 MCP Servers Security Scan Summary
────────────────────────────────────────────────────────────
📊 Scan Summary:
  • Servers: 2 total (2 ✅ successful, 0 ❌ failed)
  • Resources: 81 tools, 0 resources, 2 prompts
  • Security: ✅ All servers passed security checks

📄 Detailed report generated: scan_20250804_073225.md
```

## Contributing

We welcome contributions to Ramparts mcp scan. If you have suggestions, bug reports, or feature requests, please open an issue on our GitHub repository.

## Documentation
- 🔍 **[Troubleshooting Guide](docs/troubleshooting.md)** - Solutions to common issues
- ⚙️ **[Configuration Reference](docs/configuration.md)** - Complete configuration file documentation
- 📖 **[CLI Reference](docs/cli.md)** - All commands, options, and usage examples
- 🔒 **[AI Gateway Documentation](docs/proxy.md)** - Security-first AI gateway for MCP (competitive alternative to Nexus/LiteLLM)

## Project Structure

Ramparts uses a Cargo workspace architecture for modular development:

```
ramparts/
├── scan/           # Main CLI tool and scanning functionality (Apache 2.0)
├── proxy/          # MCP proxy with Javelin Guardrails (Proprietary)
├── common/         # Shared types and utilities (Apache 2.0)
├── docs/           # Documentation
└── examples/       # Configuration examples
```

### Components

- **ramparts-scan**: Core MCP security scanning functionality (Apache 2.0 license)
- **ramparts-proxy**: Security-first AI gateway for MCP - competitive alternative to Nexus, LiteLLM, and Cloudflare AI Gateway (Proprietary license)
- **ramparts-common**: Shared types and utilities used by both components

### Licensing

- **Scan functionality**: Apache 2.0 (free and open source)
- **Proxy functionality**: Javelin Proprietary License (requires API key)

## Why Choose Ramparts AI Gateway?

**🔒 Security-First vs. Routing-First**
Unlike Nexus, LiteLLM, or Cloudflare AI Gateway that focus on routing and cost management, Ramparts puts security at the center of every request with AI-powered threat detection.

**🎯 MCP-Native vs. Generic HTTP**
While other gateways treat MCP as generic HTTP/JSON, Ramparts understands MCP tools semantically - knowing the difference between `file_read` and `execute_command` for precise security validation.

**🏢 Enterprise Security vs. Basic Rate Limiting**
Ramparts provides enterprise-grade security policies, audit trails, and compliance features that basic AI gateways simply don't offer.

**🚀 Competitive Advantage**
- **AI-Powered Security**: Javelin Guardrails vs. simple pattern matching
- **Tool-Aware Validation**: Understands MCP tool semantics and risks
- **Enterprise Compliance**: Complete audit trails and policy enforcement
- **Self-Hosted Option**: Deploy on-premise vs. cloud-only solutions

## Additional Resources
- [Need Support?](https://github.com/getjavelin/ramparts/issues)
- [MCP Protocol Documentation](https://modelcontextprotocol.io/)
- [Configuration Examples](examples/config_example.json)
- [Javelin Guardrails](https://www.getjavelin.com)
- [Competitive Analysis](docs/proxy.md#competitive-analysis-why-choose-ramparts)

