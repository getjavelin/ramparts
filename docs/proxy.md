# Ramparts Proxy

The Ramparts Proxy is a client-side MCP (Model Context Protocol) proxy that intercepts tool calls and validates them against Javelin Guardrails before forwarding them to the target MCP server.

## Overview

The proxy acts as a security layer between MCP clients and servers, providing:

- **Request Validation**: All MCP tool calls are validated against Javelin Guardrails
- **Security Filtering**: Blocks or modifies requests that fail guardrail validation
- **Transparent Proxying**: Maintains MCP protocol compatibility
- **Licensing Control**: Requires valid Javelin API key for operation

## Architecture

```
MCP Client → Ramparts Proxy → Javelin Guardrails → Target MCP Server
```

The proxy intercepts requests, validates them with Javelin's security policies, and only forwards approved requests to the target server.

## Installation

The proxy is included as part of the Ramparts CLI tool:

```bash
cargo install ramparts
```

## Usage

### Basic Usage

Start the proxy server:

```bash
ramparts proxy 127.0.0.1:8080
```

### Environment Variables

Configure the proxy using environment variables:

```bash
# Required: API key for Javelin Guardrails
export JAVELIN_API_KEY="your-api-key"
# or
export LLM_API_KEY="your-api-key"
# or (legacy)
export OPENAI_API_KEY="your-api-key"

# Optional: Javelin API URL (default: https://api.getjavelin.com)
export JAVELIN_API_URL="https://api.getjavelin.com"

# Optional: Request timeout in seconds (default: 30)
export JAVELIN_TIMEOUT_SECONDS="30"

# Optional: Fail open/closed when API unavailable (default: true)
export JAVELIN_FAIL_OPEN="true"

# Optional: Proxy configuration
export PROXY_LOG_REQUESTS="true"
export PROXY_CACHE_VALIDATIONS="false"
export PROXY_CACHE_TTL_SECONDS="300"
export PROXY_MAX_REQUEST_SIZE="1048576"
```

### API Endpoints

The proxy exposes several HTTP endpoints:

#### Health Check
```bash
GET /health
```

Response:
```json
{
  "status": "healthy",
  "service": "ramparts-proxy",
  "version": "0.7.0"
}
```

#### License Status
```bash
GET /license
```

Response:
```json
{
  "license": {
    "status": "Valid license using JAVELIN_API_KEY",
    "component": "ramparts-proxy",
    "license_type": "Javelin Proprietary License",
    "requires_api_key": true,
    "contact": "legal@getjavelin.com"
  },
  "timestamp": "2025-01-XX:XX:XX.XXXZ"
}
```

#### Request Validation
```bash
POST /validate
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "file_read",
    "arguments": {
      "path": "/etc/passwd"
    }
  }
}
```

#### MCP Proxy
```bash
POST /proxy/{target}
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "safe_tool",
    "arguments": {
      "data": "safe_value"
    }
  }
}
```

## Configuration

### Proxy Behavior

- **Log Requests**: Enable/disable request logging
- **Cache Validations**: Cache validation results for performance
- **Max Request Size**: Limit request payload size
- **Fail Open/Closed**: Behavior when Javelin API is unavailable

### Javelin Integration

- **API Key**: Required for all proxy operations
- **Base URL**: Javelin API endpoint
- **Timeout**: Request timeout for API calls
- **Fail Strategy**: Open (allow) or closed (deny) on API errors

## Security Features

### Request Validation

All MCP tool calls are validated against Javelin Guardrails policies:

- **Tool Poisoning Detection**: Identifies malicious tools
- **Command Injection Prevention**: Blocks dangerous system commands
- **Path Traversal Protection**: Prevents unauthorized file access
- **SQL Injection Detection**: Identifies database attack vectors
- **Secrets Leakage Prevention**: Protects sensitive credentials

### Response Filtering

Responses are also validated to prevent:

- **Data Exfiltration**: Sensitive information leakage
- **Malicious Payloads**: Harmful response content
- **Policy Violations**: Content that violates security policies

## Licensing

The ramparts-proxy component uses a proprietary license:

- **License**: Javelin Proprietary License
- **Requirements**: Valid Javelin API key
- **Usage**: Subject to Javelin Terms of Service
- **Contact**: legal@getjavelin.com

## Examples

### Docker Deployment

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ramparts /usr/local/bin/
EXPOSE 8080
CMD ["ramparts", "proxy", "0.0.0.0:8080"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ramparts-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ramparts-proxy
  template:
    metadata:
      labels:
        app: ramparts-proxy
    spec:
      containers:
      - name: ramparts-proxy
        image: ramparts:latest
        ports:
        - containerPort: 8080
        env:
        - name: JAVELIN_API_KEY
          valueFrom:
            secretKeyRef:
              name: javelin-secret
              key: api-key
        command: ["ramparts", "proxy", "0.0.0.0:8080"]
```

### Client Configuration

Configure your MCP client to use the proxy:

```json
{
  "mcpServers": {
    "proxied-server": {
      "url": "http://localhost:8080/proxy/target-server",
      "headers": {
        "Content-Type": "application/json"
      }
    }
  }
}
```

## Troubleshooting

### Common Issues

1. **License Validation Failed**
   - Ensure valid API key is set
   - Check network connectivity to Javelin API
   - Verify API key permissions

2. **Request Blocked**
   - Review Javelin Guardrails policies
   - Check request content for security violations
   - Examine proxy logs for details

3. **Connection Errors**
   - Verify proxy is running and accessible
   - Check firewall and network configuration
   - Ensure target MCP server is reachable

### Debug Mode

Enable debug logging for detailed information:

```bash
RUST_LOG=debug ramparts proxy 127.0.0.1:8080
```

## Support

- **Documentation**: https://docs.getjavelin.com
- **API Access**: https://www.getjavelin.com
- **Technical Support**: support@getjavelin.com
- **License Questions**: legal@getjavelin.com
