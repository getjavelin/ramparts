# Troubleshooting Guide

## Common Issues

### Installation Problems

#### Rust Not Installed

**Error:** `cargo: command not found`

**Solution:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
cargo --version
```

#### Compilation Errors

**Error:** Build failures during installation

**Solutions:**
```bash
# Update Rust toolchain
rustup update

# Clean and retry installation
cargo clean
cargo install ramparts

# Try without YARA-X if build fails
cargo install ramparts --no-default-features
```

#### Permission Errors

**Error:** Permission denied when running ramparts

**Solutions:**
```bash
# Make sure ~/.cargo/bin is in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Check file permissions
chmod +x $(which ramparts)

# Alternative: Install to system location (Linux/macOS)
sudo cargo install ramparts
```

### Connection Issues

#### Connection Timeout

**Error:** Connection timed out

**Solutions:**
```bash
# Increase timeout
ramparts scan <url> --timeout 60

# Check network connectivity
ping <hostname>
curl -I <url>

# Try with verbose output for debugging
ramparts scan <url> --verbose
```

#### SSL/TLS Errors

**Error:** SSL certificate verification failed

**Solutions:**
```bash
# Check certificate validity
openssl s_client -connect <hostname>:443

# Update CA certificates (Linux)
sudo apt-get update && sudo apt-get install ca-certificates

# Update CA certificates (macOS)
brew install ca-certificates
```

#### DNS Resolution Errors

**Error:** Name or service not known

**Solutions:**
```bash
# Check DNS resolution
nslookup <hostname>
dig <hostname>

# Try with IP address instead of hostname
ramparts scan http://192.168.1.100:3000/mcp/

# Check /etc/hosts file for local overrides
cat /etc/hosts
```

### Authentication Issues

#### Invalid API Key

**Error:** 401 Unauthorized or 403 Forbidden

**Solutions:**
```bash
# Verify API key format
echo $OPENAI_API_KEY

# Test API key directly
curl -H "Authorization: Bearer $OPENAI_API_KEY" https://api.openai.com/v1/models

# Check authentication header format
ramparts scan <url> --auth-headers "Authorization: Bearer $API_KEY"
```

#### Header Format Issues

**Error:** Authentication failed

**Solutions:**
```bash
# Correct header formats
ramparts scan <url> --auth-headers "Authorization: Bearer token"
ramparts scan <url> --auth-headers "X-API-Key: key"
ramparts scan <url> --auth-headers "Authorization: Basic base64string"

# Multiple headers
ramparts scan <url> --auth-headers "Authorization: Bearer $TOKEN" --auth-headers "X-Custom: value"
```

### Configuration Issues

#### Configuration File Not Found

**Error:** Configuration file not found

**Solutions:**
```bash
# Initialize configuration
ramparts init-config

# Specify configuration file explicitly
ramparts scan <url> --config /path/to/ramparts.yaml

# Check search paths
ls -la ramparts.yaml
ls -la ~/.config/ramparts/ramparts.yaml
ls -la /etc/ramparts/ramparts.yaml
```

#### Invalid Configuration Format

**Error:** YAML parsing error

**Solutions:**
```bash
# Validate YAML syntax
python -c "import yaml; yaml.safe_load(open('ramparts.yaml'))"

# Check for common YAML issues
# - Proper indentation (spaces, not tabs)
# - Quoted strings with special characters
# - Proper list formatting

# Example of correct YAML format
cat << EOF > ramparts.yaml
scanner:
  enable_yara: true
  timeout: 30
security:
  enabled: true
  min_severity: "low"
EOF
```

#### Environment Variable Issues

**Error:** API key not loaded from environment

**Solutions:**
```bash
# Check environment variables
env | grep API_KEY
echo $OPENAI_API_KEY

# Set environment variables
export OPENAI_API_KEY="your-key-here"
export ANTHROPIC_API_KEY="your-key-here"

# Make persistent (add to ~/.bashrc or ~/.zshrc)
echo 'export OPENAI_API_KEY="your-key-here"' >> ~/.bashrc
source ~/.bashrc
```

### YARA-X Related Issues

#### YARA-X Rules Not Loading

**Error:** No YARA rules found or loaded

**Solutions:**
```bash
# Check rules directory structure
ls -la rules/
ls -la rules/pre/
ls -la rules/post/

# Create rules directory if missing
mkdir -p rules/pre rules/post

# List .yar files
find rules/ -name "*.yar" -type f

# Check file permissions
chmod -R 755 rules/
chmod 644 rules/pre/*.yar
```

#### Rule Compilation Errors

**Error:** YARA rule syntax errors

**Solutions:**
```bash
# YARA-X provides detailed error messages
# Common issues and fixes:

# 1. Escape special characters in strings
# Wrong: $pattern = /api_key_{32}/
# Right: $pattern = /api_key_\{32\}/

# 2. Proper rule structure
rule RuleName
{
    meta:
        description = "Rule description"
        severity = "high"
    
    strings:
        $string1 = "pattern1"
        $string2 = /regex_pattern/
    
    condition:
        any of them
}

# 3. Check metadata format
# Ensure severity is quoted: severity = "high"
```

#### YARA-X Performance Issues

**Error:** Slow scanning with YARA-X

**Solutions:**
```bash
# Temporarily disable YARA-X
echo "scanner:
  enable_yara: false" > ramparts.yaml

# Or reinstall without YARA-X
cargo install ramparts --no-default-features --force

# Optimize rules
# - Remove overly complex regex patterns
# - Reduce number of active rules
# - Use more specific conditions
```

#### YARA-X Installation Issues

**Error:** YARA-X compilation fails

**Solutions:**
```bash
# Install without YARA-X
cargo install ramparts --no-default-features

# Update Rust toolchain
rustup update stable

# Check system requirements
rustc --version  # Should be 1.70+

# Clear cargo cache and retry
cargo clean
rm -rf ~/.cargo/registry/cache/
cargo install ramparts
```

### Performance Issues

#### Slow Scanning

**Error:** Scanning takes too long

**Solutions:**
```bash
# Increase timeout
ramparts scan <url> --timeout 120

# Adjust configuration for performance
cat << EOF > ramparts.yaml
scanner:
  parallel: true
  llm_batch_size: 20
  max_retries: 1
  enable_yara: false
security:
  min_severity: "medium"
logging:
  level: "error"
EOF

# Use performance configuration
ramparts scan <url> --config ramparts.yaml
```

#### Memory Issues

**Error:** Out of memory errors

**Solutions:**
```bash
# Reduce batch size
cat << EOF > ramparts.yaml
scanner:
  llm_batch_size: 5
  parallel: false
EOF

# Monitor memory usage
top -p $(pgrep ramparts)

# Increase system swap if needed (Linux)
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Output Issues

#### JSON Format Errors

**Error:** Invalid JSON output

**Solutions:**
```bash
# Use pretty printing
ramparts scan <url> --output json --pretty

# Validate JSON output
ramparts scan <url> --output json | jq '.'

# Save to file for inspection
ramparts scan <url> --output json > output.json
```

#### Missing Output

**Error:** No output produced

**Solutions:**
```bash
# Enable verbose output
ramparts scan <url> --verbose

# Check log level
ramparts scan <url> --verbose 2>&1 | grep -i error

# Use debug logging
RAMPARTS_LOG_LEVEL=debug ramparts scan <url>
```

### Server Mode Issues

#### Server Won't Start

**Error:** Failed to start server

**Solutions:**
```bash
# Check port availability
netstat -tlnp | grep 8080
lsof -i :8080

# Use different port
ramparts server --port 9090

# Check permissions for binding to port
# Ports < 1024 require root privileges
sudo ramparts server --port 80
```

#### Server Connection Issues

**Error:** Cannot connect to Ramparts server

**Solutions:**
```bash
# Check server status
curl http://localhost:8080/health

# Bind to all interfaces
ramparts server --host 0.0.0.0 --port 8080

# Check firewall settings
sudo ufw status
sudo iptables -L
```

## Getting Help

### Debug Information

When reporting issues, include:

```bash
# System information
uname -a
rustc --version
cargo --version

# Ramparts version
ramparts --version

# Configuration (remove sensitive data)
cat ramparts.yaml

# Error output with verbose logging
RAMPARTS_LOG_LEVEL=debug ramparts scan <url> --verbose
```

### Support Channels

1. **GitHub Issues** - [https://github.com/getjavelin/ramparts/issues](https://github.com/getjavelin/ramparts/issues)
   - Bug reports
   - Feature requests
   - Installation issues

2. **Documentation** - Review related documentation:
   - [Installation Guide](installation.md)
   - [Configuration Guide](configuration.md)
   - [Usage Guide](usage.md)
   - [Security Features](security-features.md)

3. **Community Support**
   - Check existing GitHub issues for similar problems
   - Search documentation for related topics
   - Review configuration examples

### Before Reporting Issues

1. **Search existing issues** on GitHub
2. **Try latest version** - Update to latest Ramparts version
3. **Minimal reproduction** - Provide minimal steps to reproduce the issue
4. **Include environment details** - OS, Rust version, Ramparts version
5. **Remove sensitive data** - Don't include API keys or private URLs

### Issue Report Template

```markdown
## Environment
- OS: [e.g., Ubuntu 22.04, macOS 13.0, Windows 11]
- Rust Version: [output of `rustc --version`]
- Ramparts Version: [output of `ramparts --version`]

## Issue Description
[Clear description of the problem]

## Steps to Reproduce
1. [First step]
2. [Second step]
3. [Third step]

## Expected Behavior
[What you expected to happen]

## Actual Behavior
[What actually happened]

## Error Output
```
[Error messages or logs]
```

## Configuration
```yaml
[Your ramparts.yaml configuration, with sensitive data removed]
```

## Additional Context
[Any other relevant information]
```