# API Reference

This document provides detailed information about the Ramparts REST API server mode.

## Server Mode Overview

Think of Ramparts server mode as your security scanner running as a service instead of a one-off command. While `ramparts scan` is great for quick checks, the server mode gives you a REST API that your applications, CI/CD pipelines, and monitoring systems can talk to.

### When Should You Use Server Mode?

If you're running `ramparts scan` more than once, you probably want server mode. Here are some common scenarios:

**You're building CI/CD pipelines** and want to fail builds when MCP servers have critical security issues. Instead of installing and running the CLI in every pipeline, just POST to `/scan` and check the response.

**Your team is using multiple MCP servers** and you want a centralized way to monitor their security posture. Start one Ramparts server, point it at all your MCP endpoints, and you've got a security dashboard.

**You're integrating security scanning into an existing application.** Maybe you're building a developer portal that needs to show MCP server security status, or you want to automatically scan servers when they're added to your system.

**You need scheduled scanning** without setting up complex cron jobs. The API makes it easy to build simple monitoring scripts that run regular scans and send alerts when issues are found.

### What You Get

Server mode handles multiple requests concurrently, so your team can scan different servers simultaneously without waiting. The JSON responses are consistent and easy to parse, making it simple to build automation around the results.

You also get health check endpoints for monitoring, batch scanning for efficiency, and all the same security detection capabilities as the CLI—just wrapped in a REST API that plays nicely with modern development workflows.

## Starting the Server

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

**Javelin Integration Headers:**

For Javelin MCP servers, you can include the `X-Javelin-Apikey` header which will be automatically converted to the appropriate authentication formats:

```bash
curl -X POST http://localhost:3000/scan \
  -H "Content-Type: application/json" \
  -H "X-Javelin-Apikey: your-javelin-key" \
  -d '{
    "url": "https://api.example.com/mcp/",
    "timeout": 60
  }'
```

**Request Fields:**
- `url` (required): MCP server URL or STDIO command
- `timeout` (optional): Total scan timeout in seconds (1-3600, default: 60)
- `http_timeout` (optional): HTTP request timeout in seconds (default: 30)
- `detailed` (optional): Enable detailed analysis (default: false)
- `format` (optional): Output format - "json", "table", "text", "raw" (default: "table")
- `auth_headers` (optional): Authentication headers as key-value pairs

**Special Headers:**
- `X-Javelin-Apikey`: When included in request headers, automatically adds appropriate authentication for Javelin MCP servers

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

**Javelin Integration:**
```bash
curl -X POST http://localhost:3000/batch-scan \
  -H "Content-Type: application/json" \
  -H "X-Javelin-Apikey: your-javelin-key" \
  -d '{
    "urls": ["https://api.example.com/mcp/"],
    "options": {"detailed": true}
  }'
```

**Request Fields:**
- `urls` (required): Array of MCP server URLs or STDIO commands
- `options` (optional): Shared scan configuration. Note: the `url` field in options is ignored and overridden by each URL in the `urls` array

**Special Headers:**
- `X-Javelin-Apikey`: When included in request headers, automatically adds appropriate authentication for Javelin MCP servers

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