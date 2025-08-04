# CLI Reference

This document provides detailed information about all Ramparts command-line interface options and commands.

## Basic Commands

```bash
# Scan an MCP server
ramparts scan <url> [options]

# Start Ramparts server mode
ramparts server [options]

# Scan from IDE configuration files
ramparts scan-config [options]

# Initialize configuration file
ramparts init-config

# Show help
ramparts --help
ramparts scan --help
```

## Global Options

These options are available for all commands:

```bash
Options:
  -v, --verbose                   Enable verbose output
      --debug                     Enable debug logging
  -h, --help                      Print help information
  -V, --version                   Print version information
```

## Scan Command

Scan a single MCP server for tools, resources, and security vulnerabilities.

### Usage
```bash
ramparts scan <URL> [OPTIONS]
```

### Arguments
- `<URL>` - MCP server URL or endpoint to scan

### Options

```bash
Options:
  -a, --auth-headers <HEADERS>    Authentication headers (format: "Header: Value")
                                  Can be specified multiple times
  -o, --output <FORMAT>           Output format [default: table]
                                  [possible values: json, raw, table, text]
      --report                    Generate detailed markdown report (scan_YYYYMMDD_HHMMSS.md)
  -t, --timeout <SECONDS>         Request timeout in seconds [default: 60]
      --http-timeout <SECONDS>    HTTP timeout in seconds [default: 30]
      --detailed                  Enable detailed output
      --min-severity <LEVEL>      Minimum severity level to report
                                  [possible values: low, medium, high, critical]
      --config <FILE>             Custom configuration file path
      --pretty                    Pretty print JSON output (only with --output json)
  -h, --help                      Print help information
```

### Examples

**Basic scan:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/
```

**Scan with authentication:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ \
  --auth-headers "Authorization: Bearer $TOKEN" \
  --auth-headers "X-API-Key: $API_KEY"
```

**Detailed JSON output:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ \
  --output json \
  --detailed \
  --pretty
```

**Custom timeout and severity:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ \
  --timeout 120 \
  --http-timeout 45 \
  --min-severity high
```

**Generate detailed report:**
```bash
ramparts scan https://api.githubcopilot.com/mcp/ --report
```

**STDIO server scan:**
```bash
ramparts scan "stdio:///usr/local/bin/mcp-server"
ramparts scan "node /path/to/server.js --config config.json"
ramparts scan "/usr/bin/python3 /path/to/server.py"
```

## Scan-Config Command

Scan MCP servers from IDE configuration files.

### Usage
```bash
ramparts scan-config [OPTIONS]
```

### Options

```bash
Options:
  -a, --auth-headers <HEADERS>    Authentication headers for MCP servers
  -o, --output <FORMAT>           Output format [default: table]
                                  [possible values: json, raw, table, text]
      --report                    Generate detailed markdown report (scan_YYYYMMDD_HHMMSS.md)
      --config <FILE>             Custom configuration file path
  -h, --help                      Print help information
```

### Examples

**Scan from IDE configs:**
```bash
ramparts scan-config
```

**With authentication:**
```bash
ramparts scan-config \
  --auth-headers "Authorization: Bearer $TOKEN" \
  --output json
```

**Generate report:**
```bash
ramparts scan-config --report
```

### Supported IDE Configuration Files

Ramparts automatically discovers and reads MCP server configurations from:

- **Cursor**: `~/.cursor/mcp.json`
- **Windsurf**: `~/.codeium/windsurf/mcp_config.json`
- **VS Code**: `~/.vscode/mcp.json`
- **Claude Desktop**: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
- **Claude Code**: `~/.claude.json`
- **Gemini**: `~/.gemini/mcp_config.json`
- **Neovim**: `~/.config/nvim/mcp.json`
- **Helix**: `~/.config/helix/mcp.json`
- **Zed**: `~/.config/zed/mcp.json`

## Server Command

Start the MCP Scanner microservice.

### Usage
```bash
ramparts server [OPTIONS]
```

### Options

```bash
Options:
  -p, --port <PORT>               Server port [default: 3000]
      --host <HOST>               Server host [default: 0.0.0.0]
      --config <FILE>             Configuration file path
  -h, --help                      Print help information
```

### Examples

**Default server:**
```bash
ramparts server
```

**Custom port and host:**
```bash
ramparts server --port 8080 --host 127.0.0.1
```

**With custom config:**
```bash
ramparts server --config /path/to/custom-config.yaml
```

## Init-Config Command

Create a custom configuration file with default settings.

### Usage
```bash
ramparts init-config [OPTIONS]
```

### Options

```bash
Options:
  -f, --force                     Overwrite existing configuration file
  -h, --help                      Print help information
```

### Examples

**Create default config:**
```bash
ramparts init-config
```

**Overwrite existing config:**
```bash
ramparts init-config --force
```

This creates a `ramparts.yaml` file in the current directory with all configuration options and their default values.

## Output Formats

### Table Format (Default)
Human-readable table format with colored output:
```bash
ramparts scan <url>
ramparts scan <url> --output table
```

### JSON Format
Machine-readable JSON output:
```bash
ramparts scan <url> --output json
ramparts scan <url> --output json --pretty
```

### Text Format
Simple text format:
```bash
ramparts scan <url> --output text
```

### Raw Format
Raw JSON format preserving original MCP server responses:
```bash
ramparts scan <url> --output raw
```

## Environment Variables

Ramparts respects the following environment variables:

### Logging
```bash
RUST_LOG=debug ramparts scan <url>        # Debug logging
RUST_LOG=info ramparts scan <url>         # Info logging
RUST_LOG=warn ramparts scan <url>         # Warning logging only
RUST_LOG=error ramparts scan <url>        # Error logging only
```

### Configuration
```bash
RAMPARTS_CONFIG=/path/to/config.yaml ramparts scan <url>
```

### API Keys
You can use environment variables in auth headers:
```bash
ramparts scan <url> --auth-headers "Authorization: Bearer $TOKEN"
ramparts scan <url> --auth-headers "X-API-Key: $API_KEY"
```

## Exit Codes

Ramparts uses standard exit codes:

- `0` - Success
- `1` - General error
- `2` - Configuration error
- `3` - Network/connection error
- `4` - Authentication error
- `5` - Timeout error

## Advanced Usage

### Batch Scanning from File
```bash
# Create a file with URLs
echo "https://server1.com/mcp/
https://server2.com/mcp/
stdio:///usr/local/bin/mcp-server" > servers.txt

# Scan each URL
while IFS= read -r url; do
  ramparts scan "$url" --output json >> results.json
done < servers.txt
```

### Using with jq for Processing
```bash
# Extract security issue count
ramparts scan <url> --output json | jq '.security_issues.total_issues'

# Filter high severity issues
ramparts scan <url> --output json | \
  jq '.security_issues.tool_issues[] | select(.severity == "HIGH")'

# Get all tool names
ramparts scan <url> --output json | jq -r '.tools[].name'
```

### Combining with Other Tools
```bash
# Save scan results with timestamp
ramparts scan <url> --output json > "scan-$(date +%Y%m%d-%H%M%S).json"

# Send results to webhook
ramparts scan <url> --output json | \
  curl -X POST -H "Content-Type: application/json" \
       -d @- https://webhook.example.com/ramparts

# Check exit code and send alert
if ! ramparts scan <url> --min-severity high; then
  echo "High severity issues found!" | mail -s "Security Alert" admin@example.com
fi
```

## Configuration File Locations

Ramparts looks for configuration files in the following order:

1. `--config` command line argument
2. `RAMPARTS_CONFIG` environment variable
3. `./ramparts.yaml` (current directory)
4. `~/.config/ramparts/config.yaml`
5. `/etc/ramparts/config.yaml`

## Shell Completion

Generate shell completion scripts:

### Bash
```bash
ramparts --generate-completion bash > /etc/bash_completion.d/ramparts
```

### Zsh
```bash
ramparts --generate-completion zsh > ~/.zsh/completions/_ramparts
```

### Fish
```bash
ramparts --generate-completion fish > ~/.config/fish/completions/ramparts.fish
```

### PowerShell
```powershell
ramparts --generate-completion powershell > ramparts.ps1
```

*Note: Completion generation may not be available in all versions.*

## Advanced Usage Examples

### Advanced Scanning Options

```bash
# Custom severity threshold
ramparts scan <url> --min-severity HIGH

# JSON output with formatting
ramparts scan <url> --output json --pretty

# Custom configuration file
ramparts scan <url> --config custom-ramparts.yaml

# Scan from IDE configurations
ramparts scan-config
```

### Server Mode

Ramparts can run as a REST API server for continuous monitoring:

```bash
# Start server (default: localhost:3000)
ramparts server

# Custom host and port
ramparts server --port 8080 --host 0.0.0.0
```

### Batch Scanning

```bash
# Create a servers list
echo "https://server1.com/mcp/
https://server2.com/mcp/
https://server3.com/mcp/" > servers.txt

# Run batch scan
ramparts scan --batch servers.txt
```

### Output Format Details

**Table Format (Default)**
- Human-readable with colored output
- Tree-style security issue display
- Progress indicators and summaries

**JSON Format**
- Machine-readable structured output
- Perfect for scripts and automation
- Use `--pretty` for formatted output

**Raw Format**
- Preserves original MCP server responses
- Useful for debugging and analysis
- Minimal processing of server data

### Integration Examples

**Server Mode Integration:**
- 📚 **[Complete API Documentation](docs/api.md)** - REST endpoints and request/response formats
- 🔧 **[Integration Patterns](docs/integration.md)** - CI/CD, Docker, Kubernetes, and monitoring examples