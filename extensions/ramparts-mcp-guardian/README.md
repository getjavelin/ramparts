# Ramparts MCP Guardian

**Zero-configuration security for MCP servers with Javelin Guardrails**

Ramparts MCP Guardian automatically secures all your MCP (Model Context Protocol) servers without requiring any manual configuration. Simply install the extension and all MCP requests will be validated through Javelin Guardrails.

## Features

- **🛡️ Automatic Protection**: Automatically intercepts and validates all MCP requests
- **🔧 Zero Configuration**: Works out of the box - no manual setup required
- **👁️ Real-time Monitoring**: Visual status indicators and detailed logging
- **⚡ Lightweight**: Minimal performance impact with intelligent caching
- **🎛️ Granular Control**: Per-server bypass options and custom policies
- **🔄 Hot Reload**: Dynamic policy updates without restart

## Quick Start

1. **Install the Extension**
   - Install from VS Code Marketplace or Cursor Extensions
   - The extension activates automatically

2. **Set Your Javelin API Key**
   - Open VS Code/Cursor Settings
   - Search for "Ramparts"
   - Enter your Javelin API key (get one at [getjavelin.com](https://getjavelin.com))

3. **You're Protected!**
   - All MCP servers are now automatically secured
   - Check the status bar for real-time protection status

## How It Works

Ramparts MCP Guardian works by:

1. **Auto-Discovery**: Automatically finds all MCP server configurations
2. **Transparent Proxying**: Inserts a lightweight security proxy between your IDE and MCP servers
3. **Request Validation**: Every MCP request is validated through Javelin Guardrails
4. **Seamless Operation**: Blocked requests return helpful error messages; approved requests work normally

## Configuration

### Basic Settings

- **Enable/Disable**: Toggle protection on/off
- **Javelin API Key**: Your Javelin Guardrails API key
- **Fail Mode**: Choose fail-open (allow on error) or fail-closed (block on error)
- **Log Level**: Control logging verbosity

### Advanced Settings

- **Bypassed Servers**: List of server names to exclude from protection
- **Custom Policy**: Path to custom YAML policy file for advanced rules

### Example Custom Policy

```yaml
version: 1
defaultAction: deny

tools:
  allow:
    - http.get
    - http.post
    - fs.read
    - search.index
  deny:
    - shell.exec
    - process.spawn

http:
  allowlist:
    - "https://api.getjavelin.com"
    - "https://*.githubusercontent.com"
  denyCidrs:
    - "10.0.0.0/8"
    - "192.168.0.0/16"

responseGuards:
  secretPatterns:
    - type: "ssh-key"
      regex: "-----BEGIN (?:RSA|EC|OPENSSH) PRIVATE KEY-----"
    - type: "aws-access-key"
      regex: "AKIA[0-9A-Z]{16}"
  action: redact
```

## Status Indicators

- **🟢 Protected**: Server is secured by Ramparts
- **🟡 Bypassed**: Server is excluded from protection
- **🔴 Unprotected**: Server not yet secured (extension disabled)

## Troubleshooting

### Extension Not Working?

1. Check that the extension is enabled in VS Code/Cursor
2. Verify your Javelin API key is set correctly
3. Check the Ramparts output channel for error messages

### MCP Server Not Starting?

1. Ensure the original MCP server command is valid
2. Check if the server requires specific environment variables
3. Try temporarily bypassing the server to test

### Performance Issues?

1. Enable request caching in settings
2. Reduce log level to 'error' only
3. Consider bypassing non-critical servers

## Support

- **Documentation**: [ramparts.dev](https://ramparts.dev)
- **Issues**: [GitHub Issues](https://github.com/getjavelin/ramparts/issues)
- **Community**: [Discord](https://discord.gg/javelin)

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.
