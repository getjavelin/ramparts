# Ramparts AGENT.md

## Build/Test/Lint Commands
- **Build**: `make build` (includes formatting + clippy checks)
- **Quick check**: `make check` (runs `cargo check --all-features`)
- **Test**: `cargo test --all-features` or `make test`
- **Lint**: `cargo clippy --all-features -- -D warnings` or `make lint`
- **Format**: `cargo fmt` or `make fmt`
- **Format check**: `cargo fmt --all -- --check` or `make fmt-check`
- **Full CI check**: `make ci-check` (format + clippy + tests + audit)

## Architecture
- **Rust CLI tool** for scanning Model Context Protocol (MCP) servers
- **Multi-transport MCP client**: HTTP/HTTPS, SSE, stdio/subprocess using rmcp SDK
- **Security scanning**: YARA-X rules engine + cross-origin vulnerability detection
- **Three operation modes**: CLI scanner, HTTP server (Axum), MCP server (stdio/SSE/HTTP)
- **Main modules**: `scanner.rs` (core), `mcp_client.rs` (transport), `server.rs` (HTTP API), `config.rs` (IDE & scanner configuration)

## CLI Subcommands
- `scan <URL>`: Scan a single MCP server
- `scan-config`: Scan from IDE configs (e.g., VSCode, Cursor, Claude)
- `init-config [--force]`: Create default `config.yaml`
- `server [--port 3000] [--host 0.0.0.0]`: Start HTTP API
- `mcp-stdio`: Run as MCP server over stdio
- `mcp-sse [--host 0.0.0.0] [--port 8000]`: Run MCP SSE server
- `mcp-http [--host 0.0.0.0] [--port 8081]`: Run streamable HTTP MCP server

## Code Style
- **Error handling**: Use `anyhow::Result` for main errors, `thiserror` for custom error types
- **Logging**: Use `tracing` crate with structured logging
- **Async**: Tokio runtime, async/await throughout
- **CLI**: clap with derive macros
- **JSON**: serde for serialization
- **Imports**: Group std, external crates, then local modules with blank lines between
- **Naming**: snake_case for vars/functions, PascalCase for types, SCREAMING_SNAKE for constants

## Security/Secrets
- Do not commit secrets or API keys; use env vars or local config only
- Prefer `make ci-check` before PRs (format, clippy, tests, audit)

## Quick Usage
```bash
# Basic scan
ramparts scan http://localhost:3000 --format json
# With auth header(s)
ramparts scan https://api.example.com/mcp --auth-headers "Authorization: Bearer TOKEN"
# From IDE configs
ramparts scan-config --format json
# Start HTTP server
ramparts server --port 3000
# Run MCP server modes
ramparts mcp-stdio
ramparts mcp-sse --port 8000
ramparts mcp-http --port 8081
```
