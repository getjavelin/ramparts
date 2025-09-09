# Ramparts Proxy - Security-First AI Gateway

The Ramparts Proxy is a **security-first AI gateway** specifically designed for the Model Context Protocol (MCP). Unlike traditional AI gateways that focus primarily on routing and cost management, Ramparts puts security at the center of every request.

## Why Choose Ramparts Over Other AI Gateways?

### **Security-First Design**
While solutions like Nexus, LiteLLM, and Cloudflare AI Gateway provide routing and basic controls, Ramparts is built from the ground up for security:

- **Advanced Threat Detection**: Powered by Javelin Guardrails, not basic pattern matching
- **MCP-Native Security**: Deep understanding of MCP tool semantics and risks
- **Real-time Validation**: Every request validated against enterprise security policies
- **Zero-Trust Architecture**: No request is trusted by default

### **MCP-Native vs. Generic LLM Routing**
Traditional AI gateways treat all requests as generic HTTP/JSON. Ramparts understands MCP:

- **Tool-Aware Security**: Knows the difference between `file_read` and `execute_command`
- **Context-Sensitive Validation**: Validates arguments based on tool semantics
- **MCP Protocol Optimization**: Built for MCP's specific request/response patterns
- **Native Tool Categorization**: Automatic classification of security-sensitive operations

### **Enterprise Security vs. Basic Rate Limiting**
Ramparts provides enterprise-grade security features that basic AI gateways lack:

- **Policy-Based Validation**: Configurable security policies per organization
- **Audit Trail**: Complete security audit logs for compliance
- **Data Loss Prevention**: Prevents sensitive data exfiltration
- **Threat Intelligence**: Continuously updated security models

## Overview

The proxy acts as a comprehensive security gateway for MCP environments, providing:

- **Advanced Request Validation**: All MCP tool calls validated against Javelin Guardrails
- **Intelligent Security Filtering**: Context-aware blocking of malicious requests
- **Transparent MCP Proxying**: Full MCP protocol compatibility with zero client changes
- **Enterprise Licensing**: Professional-grade security requires valid Javelin API key

## Architecture

```
MCP Client → Ramparts Security Gateway → Javelin Guardrails → Target MCP Server
                    ↓
            Security Policies, Audit Logs, Threat Detection
```

### **Competitive Architecture Comparison**

| Feature | Ramparts | Nexus | LiteLLM | Cloudflare AI Gateway |
|---------|----------|-------|---------|----------------------|
| **MCP-Native** | ✅ Built for MCP | ❌ Generic routing | ❌ Generic routing | ❌ Generic routing |
| **Security-First** | ✅ Javelin Guardrails | ❌ Basic validation | ❌ Basic validation | ❌ Basic validation |
| **Tool-Aware** | ✅ Understands MCP tools | ❌ Treats as HTTP | ❌ Treats as HTTP | ❌ Treats as HTTP |
| **Enterprise Security** | ✅ Full audit & compliance | ❌ Limited logging | ❌ Limited logging | ✅ Basic logging |
| **Threat Detection** | ✅ AI-powered analysis | ❌ Pattern matching | ❌ Pattern matching | ❌ Pattern matching |
| **Zero-Trust** | ✅ Every request validated | ❌ Basic rate limiting | ❌ Basic rate limiting | ❌ Basic rate limiting |

The proxy intercepts requests, applies enterprise-grade security validation through Javelin's AI-powered threat detection, and only forwards approved requests to target servers.

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

#### MCP Protocol Endpoint
```bash
POST /mcp
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

## Security Features - Beyond Basic AI Gateways

### **Enterprise-Grade Request Validation**
Unlike basic AI gateways that only provide rate limiting, Ramparts offers comprehensive security:

**Advanced Threat Detection:**
- **AI-Powered Analysis**: Javelin Guardrails uses machine learning, not simple regex patterns
- **Tool Poisoning Detection**: Identifies sophisticated malicious tool injections
- **Command Injection Prevention**: Blocks dangerous system commands with context awareness
- **Path Traversal Protection**: Prevents unauthorized file access with semantic understanding
- **SQL Injection Detection**: Advanced database attack vector identification
- **Secrets Leakage Prevention**: Protects sensitive credentials using AI pattern recognition

**MCP-Specific Security:**
- **Tool Semantics Understanding**: Knows what each MCP tool actually does
- **Argument Validation**: Validates tool arguments based on expected schemas
- **Cross-Tool Attack Detection**: Identifies attack chains across multiple tool calls
- **Resource Access Control**: Fine-grained permissions for file, network, and system access

### **Intelligent Response Filtering**
Goes far beyond what traditional AI gateways offer:

- **Data Exfiltration Prevention**: AI-powered detection of sensitive information leakage
- **Malicious Payload Detection**: Advanced analysis of response content for threats
- **Policy Compliance**: Automated enforcement of organizational security policies
- **Content Sanitization**: Intelligent removal of sensitive data while preserving functionality

### **Why This Matters vs. Competitors**

**Nexus/LiteLLM Limitations:**
- Only route requests, don't understand MCP tool semantics
- Basic pattern matching for security (easily bypassed)
- No understanding of tool argument context
- Limited audit capabilities

**Cloudflare AI Gateway Limitations:**
- Generic HTTP proxy, not MCP-aware
- Basic rate limiting and caching only
- No tool-specific security validation
- Limited enterprise security features

**Ramparts Advantages:**
- Deep MCP protocol understanding
- AI-powered threat detection
- Tool-aware security policies
- Enterprise compliance features

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
      "url": "http://localhost:8080/mcp",
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

## Competitive Analysis: Why Choose Ramparts?

### **vs. Nexus Router**
| Feature | Ramparts | Nexus |
|---------|----------|-------|
| **Security Focus** | ✅ AI-powered threat detection | ❌ Basic routing only |
| **MCP Understanding** | ✅ Tool-aware validation | ❌ Generic HTTP proxy |
| **Enterprise Features** | ✅ Full audit & compliance | ❌ Limited logging |
| **Threat Detection** | ✅ Javelin Guardrails | ❌ No security validation |
| **Response Filtering** | ✅ AI-powered analysis | ❌ No response validation |

### **vs. LiteLLM**
| Feature | Ramparts | LiteLLM |
|---------|----------|---------|
| **Protocol Support** | ✅ MCP-native | ❌ Generic LLM APIs |
| **Security Validation** | ✅ Every request/response | ❌ Basic rate limiting |
| **Tool Awareness** | ✅ Understands MCP tools | ❌ Treats as HTTP |
| **Enterprise Security** | ✅ Policy enforcement | ❌ No security policies |
| **Compliance** | ✅ Full audit trails | ❌ Basic logging |

### **vs. Cloudflare AI Gateway**
| Feature | Ramparts | Cloudflare AI Gateway |
|---------|----------|----------------------|
| **MCP Support** | ✅ Built for MCP | ❌ Generic HTTP proxy |
| **Security Depth** | ✅ AI threat analysis | ❌ Basic caching/limits |
| **Tool Validation** | ✅ Semantic understanding | ❌ No tool awareness |
| **On-Premise** | ✅ Self-hosted option | ❌ Cloud-only |
| **Customization** | ✅ Configurable policies | ❌ Limited options |

### **Key Differentiators**

**🔒 Security-First Architecture**
- Built from the ground up for security, not retrofitted
- AI-powered threat detection vs. simple pattern matching
- Understands MCP tool semantics and risks

**🎯 MCP-Native Design**
- Deep understanding of MCP protocol and tools
- Tool-specific validation rules and policies
- Optimized for MCP request/response patterns

**🏢 Enterprise-Ready**
- Complete audit trails for compliance
- Configurable security policies
- Self-hosted deployment options

**🚀 Performance & Reliability**
- Intelligent caching with deduplication
- Fail-open/fail-closed options
- Connection pooling and optimization

## Support

- **Documentation**: https://docs.getjavelin.com
- **API Access**: https://www.getjavelin.com
- **Technical Support**: support@getjavelin.com
- **License Questions**: legal@getjavelin.com
- **Competitive Inquiries**: sales@getjavelin.com
