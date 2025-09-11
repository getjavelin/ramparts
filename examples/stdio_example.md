# STDIO Transport Example

This example demonstrates how to use the MCP Scanner with STDIO transport to scan local MCP server processes.

## Overview

STDIO transport allows you to scan MCP servers that run as local processes, communicating via standard input/output. This is useful for:

- Local development servers
- Command-line MCP tools
- Process-based MCP implementations
- Testing MCP servers without HTTP setup

## Supported STDIO URL Formats

The scanner supports several STDIO URL formats:

### 1. Explicit STDIO Protocol
```
stdio:///usr/local/bin/mcp-server
stdio://node /path/to/mcp-server.js
stdio://python3 /path/to/mcp-server.py
```

### 2. Direct Command
```
/usr/local/bin/mcp-server
node /path/to/mcp-server.js
python3 /path/to/mcp-server.py
mcp-server --config config.json
```

### 3. Command with Arguments
```
stdio://mcp-server --port 3000 --debug
stdio://node server.js --env production
stdio://python3 -m mcp_server --config config.yaml
```

## CLI Usage Examples

### Basic STDIO Scan
```bash
# Scan a local MCP server
cargo run -- scan stdio:///usr/local/bin/mcp-server

# Scan with command arguments
cargo run -- scan "stdio://node /path/to/server.js --debug"

# Direct command (without stdio:// prefix)
cargo run -- scan "/usr/local/bin/mcp-server"
```

### STDIO Scan with Options
```bash
# Scan with timeout and detailed output
cargo run -- scan stdio:///usr/local/bin/mcp-server \
  --timeout 60 \
  --detailed \
  --format json

# Scan with authentication (if supported by the server)
cargo run -- scan stdio:///usr/local/bin/mcp-server \
  --auth-headers "Authorization: Bearer token123"
```

### Batch STDIO Scanning
```bash
# Scan multiple STDIO servers
cargo run -- scan-config \
  --auth-headers "Authorization: Bearer token123"
```

With config file containing STDIO servers:
```json
{
  "servers": [
    {
      "name": "local-server-1",
      "url": "stdio:///usr/local/bin/mcp-server-1",
      "description": "Local development server 1"
    },
    {
      "name": "local-server-2", 
      "url": "stdio://node /path/to/server.js",
      "description": "Node.js MCP server"
    }
  ]
}
```

## Server API Usage

### HTTP API with STDIO
```bash
# Start the scanner server
cargo run -- server --port 3001

# Scan STDIO server via HTTP API
curl -X POST http://localhost:3001/v1/ramparts/scan \
  -H "Content-Type: application/json" \
  -d '{
    "url": "stdio:///usr/local/bin/mcp-server",
    "timeout": 60,
    "detailed": true,
    "format": "json"
  }'
```



## Protocol Compliance

The STDIO transport implements the MCP 2025-06-18 specification:

### JSON-RPC Communication
- Sends JSON-RPC 2.0 requests via stdin
- Receives JSON-RPC responses via stdout
- Uses newline-delimited JSON format
- Supports session management

### Supported MCP Methods
- `initialize` - Initialize MCP session
- `tools/list` - List available tools
- `resources/list` - List available resources  
- `prompts/list` - List available prompts
- `server/info` - Get server information
- `notifications/shutdown` - Shutdown session

### Error Handling
- Process startup failures
- Communication timeouts
- JSON parsing errors
- Protocol version mismatches

## Security Considerations

### Process Isolation
- Each STDIO scan runs in a separate process
- Processes are automatically cleaned up
- No persistent connections between scans

### Command Validation
- Basic executable path validation
- PATH resolution for commands
- Warning for non-existent executables

### Input Sanitization
- Command argument validation
- URL encoding/decoding
- Path traversal prevention

## Troubleshooting

### Common Issues

1. **Command Not Found**
   ```
   Error: STDIO command may not exist or be executable: /usr/local/bin/mcp-server
   ```
   Solution: Verify the executable path and permissions

2. **Process Timeout**
   ```
   Error: STDIO read timeout
   ```
   Solution: Increase timeout or check if the server is responding

3. **JSON-RPC Errors**
   ```
   Error: JSON-RPC error: {"code": -32601, "message": "Method not found"}
   ```
   Solution: Check if the MCP server supports the requested methods

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug cargo run -- scan stdio:///usr/local/bin/mcp-server
```

### Validation
```bash
# Validate STDIO configuration
curl -X POST http://localhost:3001/v1/ramparts/validate \
  -H "Content-Type: application/json" \
  -d '{
    "url": "stdio:///usr/local/bin/mcp-server"
  }'
```

## Integration Examples

### IDE Configuration
```json
// ~/.cursor/mcp.json
{
  "servers": [
    {
      "name": "local-tools",
      "url": "stdio:///usr/local/bin/mcp-tools",
      "description": "Local development tools"
    }
  ]
}
```

### CI/CD Pipeline
```yaml
# .github/workflows/mcp-scan.yml
- name: Scan MCP Servers
  run: |
    cargo run -- scan stdio:///usr/local/bin/mcp-server \
      --timeout 30 \
      --format json > scan-results.json
```

### Docker Integration
```dockerfile
# Dockerfile
FROM rust:latest
COPY . /app
WORKDIR /app
RUN cargo build --release

ENTRYPOINT ["cargo", "run", "--release", "--", "scan"]
```

## Performance Considerations

### Process Overhead
- Each STDIO scan creates a new process
- Process startup time affects scan duration
- Memory usage scales with concurrent scans

### Optimization Tips
- Use appropriate timeouts for your servers
- Batch scan multiple servers when possible
- Consider HTTP transport for frequently scanned servers

### Monitoring
```bash
# Monitor scan performance
cargo run -- scan stdio:///usr/local/bin/mcp-server --detailed
```

The output will include timing information for each capability scan. 