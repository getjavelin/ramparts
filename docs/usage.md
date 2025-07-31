# Usage Guide

## Basic Commands

### Scan Command

The primary command for scanning MCP servers:

```bash
ramparts scan <url> [options]
```

### Server Mode

Start Ramparts as a server for continuous monitoring:

```bash
ramparts server [options]
```

### Configuration

Initialize a configuration file:

```bash
ramparts init-config
```

### Help

Show help information:

```bash
ramparts --help
ramparts scan --help
ramparts server --help
```

## Scanning Examples

### Basic Scanning

**Scan a public MCP server:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --auth-headers "Authorization: Bearer $GITHUB_TOKEN"
```

**Scan a local MCP server:**
```bash
ramparts scan http://localhost:3000/mcp/
```

**Scan with custom timeout:**
```bash
ramparts scan <url> --timeout 60
```

### Authentication

**API Key authentication:**
```bash
ramparts scan https://api.example.com/mcp/ --auth-headers "X-API-Key: $API_KEY"
```

**Bearer token authentication:**
```bash
ramparts scan <url> --auth-headers "Authorization: Bearer $TOKEN"
```

**Multiple headers:**
```bash
ramparts scan <url> --auth-headers "Authorization: Bearer $TOKEN" --auth-headers "X-Custom-Header: value"
```

### Output Formats

**Default text output:**
```bash
ramparts scan <url>
```

**JSON output:**
```bash
ramparts scan <url> --output json
```

**Pretty-printed JSON:**
```bash
ramparts scan <url> --output json --pretty
```

**Raw output:**
```bash
ramparts scan <url> --output raw
```

**Verbose output:**
```bash
ramparts scan <url> --verbose
```

### Filtering Results

**Set minimum severity level:**
```bash
ramparts scan <url> --min-severity HIGH
ramparts scan <url> --min-severity MEDIUM
ramparts scan <url> --min-severity LOW
```

**Use custom configuration:**
```bash
ramparts scan <url> --config custom-ramparts.yaml
```

## Advanced Usage

### Server Mode

Start Ramparts as a server for continuous monitoring and API access:

```bash
# Start server on default port (8080)
ramparts server

# Start server on custom port
ramparts server --port 9090

# Start server on specific host
ramparts server --host 0.0.0.0 --port 8080

# Start server with custom configuration
ramparts server --config ramparts.yaml
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

### Integration Examples

**Save results to file:**
```bash
ramparts scan <url> --output json > scan-results.json
```

**Pipe to other tools:**
```bash
ramparts scan <url> --output json | jq '.security_issues[] | select(.severity == "HIGH")'
```

**Use in CI/CD:**
```bash
#!/bin/bash
RESULT=$(ramparts scan $MCP_SERVER_URL --output json)
HIGH_ISSUES=$(echo $RESULT | jq '.security_issues[] | select(.severity == "HIGH") | length')

if [ "$HIGH_ISSUES" -gt 0 ]; then
  echo "High severity security issues found!"
  exit 1
fi
```

## CLI Reference

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
  --batch <FILE>                  Scan multiple servers from file
```

### Server Options

```bash
Options:
  -p, --port <PORT>               Server port [default: 8080]
  -h, --host <HOST>               Server host [default: 127.0.0.1]
  --config <FILE>                 Configuration file
```

### Global Options

```bash
Options:
  --version                       Print version information
  --help                          Print help information
```

## Output Formats

### Text Format (Default)

The default human-readable format:

```
================================================================================
MCP Server Scan Result
================================================================================
URL: https://api.githubcopilot.com/mcp/
Status: Success
Response Time: 1234ms

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

Summary: 74 tools scanned, 2 warnings found
================================================================================
```

### JSON Format

Structured data format for programmatic use:

```json
{
  "url": "https://api.githubcopilot.com/mcp/",
  "status": "success",
  "response_time": 1234,
  "timestamp": "2024-01-01T12:00:00.000Z",
  "server_info": {
    "name": "GitHub Copilot MCP Server",
    "version": "1.0.0",
    "description": "GitHub Copilot MCP server",
    "capabilities": ["tools", "resources", "prompts"]
  },
  "scan_results": {
    "tools": 74,
    "resources": 0,
    "prompts": 0
  },
  "security_issues": [
    {
      "tool": "create_or_update_file",
      "severity": "HIGH",
      "type": "path_traversal",
      "description": "Potential path traversal vulnerability",
      "details": "Tool accepts 'path' parameter without proper validation"
    }
  ],
  "yara_results": [
    {
      "rule": "CrossDomainContamination",
      "severity": "HIGH",
      "description": "Cross-domain contamination detected",
      "matched": "api.github.com, webhooks.github.com"
    }
  ]
}
```

### Raw Format

Minimal output for scripting:

```
HIGH: create_or_update_file: path_traversal
HIGH: domain-analysis: CrossDomainContamination
```

## Best Practices

### Regular Scanning

- Scan MCP servers before connecting to them
- Re-scan periodically to catch new vulnerabilities
- Integrate scanning into your CI/CD pipeline

### Security Considerations

- Use appropriate authentication headers for private servers
- Set reasonable timeouts for network operations
- Review scan results regularly and address high-severity issues

### Performance Optimization

- Use batch scanning for multiple servers
- Configure YARA-X settings based on your requirements
- Adjust timeout values based on server response times

### Integration Tips

- Use JSON output for programmatic processing
- Filter results by severity level in automation
- Save scan results for historical comparison
- Set up alerting for high-severity findings

## Common Use Cases

### Development Workflow

```bash
# Test local MCP server during development
ramparts scan http://localhost:3000/mcp/ --verbose

# Validate before deployment
ramparts scan $STAGING_SERVER --config production.yaml
```

### Security Auditing

```bash
# Comprehensive audit with all checks
ramparts scan $TARGET_SERVER --output json --pretty > audit-report.json

# Focus on high-severity issues
ramparts scan $TARGET_SERVER --min-severity HIGH
```

### Continuous Monitoring

```bash
# Start monitoring server
ramparts server --port 8080

# Scheduled scanning via cron
0 */6 * * * ramparts scan $MCP_SERVER --output json >> daily-scans.log
```

## Next Steps

- Review [configuration options](configuration.md) for customization
- Learn about [security features](security-features.md) and what Ramparts detects
- Check the [troubleshooting guide](troubleshooting.md) if you encounter issues