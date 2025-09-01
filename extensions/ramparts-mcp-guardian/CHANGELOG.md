# Change Log

All notable changes to the "Ramparts MCP Guardian" extension will be documented in this file.

## [0.1.0] - 2024-01-XX

### Added
- Initial release of Ramparts MCP Guardian
- Automatic MCP server discovery and protection
- Zero-configuration setup with Javelin Guardrails integration
- Real-time status monitoring and logging
- Per-server bypass controls
- Custom policy support via YAML files
- Support for VS Code and Cursor IDEs
- Comprehensive tree view for MCP server management
- Status bar integration with protection indicators

### Features
- **Auto-Discovery**: Automatically finds MCP configurations in:
  - VS Code: `~/.vscode/mcp.json`, workspace `.vscode/mcp.json`
  - Cursor: `~/.cursor/mcp.json`, workspace `.cursor/mcp.json`
  - Workspace-specific: `.cursor/rules/mcp.json`
- **Transparent Proxying**: Lightweight stdio proxy with JSON-RPC interception
- **Request Validation**: Integration with Javelin Guardrails API
- **Response Filtering**: Secret detection and redaction
- **Fail-Safe Operation**: Configurable fail-open/fail-closed behavior
- **Hot Reload**: Dynamic configuration updates without restart

### Security
- All MCP requests validated through Javelin Guardrails
- Secret pattern detection and automatic redaction
- Network request filtering and validation
- File system access controls
- Command injection prevention

### UI/UX
- Status bar indicator with real-time protection status
- Tree view showing all discovered MCP servers
- Context menu actions for server management
- Comprehensive settings panel
- Detailed logging with sensitive data redaction

### Performance
- Minimal latency overhead (<10ms typical)
- Intelligent request caching
- Efficient JSON-RPC parsing
- Lightweight proxy process per server

### Compatibility
- VS Code 1.74.0+
- Cursor IDE
- Windows, macOS, Linux
- All MCP server types (stdio, HTTP, SSE)

## [Unreleased]

### Planned
- HTTP/SSE server proxying support
- Advanced policy editor UI
- Bulk server management
- Integration with enterprise policy management
- Performance analytics dashboard
- Automated security reporting
