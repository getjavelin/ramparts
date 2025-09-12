# Ramparts Examples

This directory contains example configurations and demonstration scripts for Ramparts.

## 📁 Files

### Configuration Examples
- **`config_example.json`** - Example MCP server configuration
- **`servers.txt`** - List of example MCP server URLs for testing

### Proxy Examples
- **`demo_proxy.sh`** - Complete proxy demonstration script
- **`stdio_example.md`** - STDIO proxy usage examples

## 🚀 Quick Start

### Run Proxy Demo
```bash
# Run the complete proxy demonstration
./examples/demo_proxy.sh
```

This script will:
1. ✅ Build the STDIO proxy binary
2. ✅ Test STDIO proxy self-check
3. ✅ Start HTTP proxy in test mode
4. ✅ Test health endpoints
5. ✅ Validate safe requests
6. ✅ Block dangerous requests
7. ✅ Clean up processes

### Expected Output
```
🚀 Ramparts Proxy Demo
======================
ℹ️  Building Ramparts proxy...
✅ STDIO proxy binary built successfully
ℹ️  Testing STDIO proxy self-check...
✅ STDIO proxy self-check passed
ℹ️  Starting HTTP proxy in test mode...
ℹ️  Testing HTTP proxy health endpoint...
✅ HTTP proxy health check passed
ℹ️  Testing safe request validation...
✅ Safe request validation passed
ℹ️  Testing dangerous request blocking...
✅ Dangerous request properly blocked
ℹ️  Cleaning up...

🎉 Demo completed successfully!
```

## 📚 Documentation

For detailed documentation, see:
- [Proxy Overview](../docs/proxy/README.md)
- [Configuration Guide](../docs/proxy/configuration.md)
- [Proxy Modes](../docs/proxy/modes.md)
- [E2E Test Results](../docs/proxy/E2E_TEST_RESULTS.md)

## 🔧 Manual Testing

### HTTP Proxy
```bash
# Start HTTP proxy
JAVELIN_API_KEY=test-mode cargo run -- proxy 127.0.0.1:8080

# Test health (in another terminal)
curl http://localhost:8080/health

# Test validation
curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"read_file"}}'
```

### STDIO Proxy
```bash
# Build STDIO proxy
cd proxy && cargo build --bin ramparts-mcp-proxy-stdio

# Test self-check
./proxy/target/debug/ramparts-mcp-proxy-stdio --self-check

# Run with target (requires MCP server)
RAMPARTS_TARGET_CMD="your-mcp-server" ./proxy/target/debug/ramparts-mcp-proxy-stdio
```

## 🛡️ Security Testing

The demo script tests these security scenarios:

### ✅ Safe Requests (Allowed)
- File read operations: `read_file`
- Data queries: `database_query`
- Network requests: `http_request`

### ❌ Dangerous Requests (Blocked)
- System commands: `exec`, `shell`, `bash`
- File operations: `rm`, `del`, `format`
- Injection patterns: `../`, `'; DROP`, `rm -rf`

## 🎯 Next Steps

After running the demo:

1. **Production Setup**: Configure with real Javelin API key
2. **Integration**: Connect to actual MCP servers
3. **Deployment**: Use Docker or Kubernetes
4. **Monitoring**: Set up logging and metrics

See the [documentation](../docs/proxy/) for detailed deployment guides.
