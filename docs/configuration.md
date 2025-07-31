# Configuration Guide

## Overview

Ramparts uses a YAML configuration file (`ramparts.yaml`) for customizing security rules, scanner behavior, and output settings. This guide covers all available configuration options.

## Creating a Configuration File

Initialize a default configuration file:

```bash
ramparts init-config
```

This creates a `ramparts.yaml` file in your current directory with default settings.

## Configuration File Structure

```yaml
# Example ramparts.yaml
llm:
  provider: "openai"
  model: "gpt-4o"
  base_url: "https://api.openai.com/v1"
  api_key: ""
  timeout: 30
  max_tokens: 4000
  temperature: 0.1

scanner:
  http_timeout: 30
  scan_timeout: 60
  detailed: false
  format: "table"
  parallel: true
  max_retries: 3
  retry_delay_ms: 1000
  llm_batch_size: 10
  enable_yara: true

security:
  enabled: true
  min_severity: "low"
  checks:
    tool_poisoning: true
    secrets_leakage: true
    sql_injection: true
    command_injection: true
    path_traversal: true
    auth_bypass: true
    cross_origin_escalation: true
    prompt_injection: true
    pii_leakage: true
    jailbreak: true

logging:
  level: "info"
  colored: true
  timestamps: true

performance:
  tracking: true
  slow_threshold_ms: 5000
```

## Configuration Sections

### LLM Configuration

Configure the AI model used for security analysis:

```yaml
llm:
  provider: "openai"          # LLM provider (openai, anthropic, local)
  model: "gpt-4o"            # Model name
  base_url: "https://api.openai.com/v1"  # API endpoint
  api_key: ""                # API key (can use environment variable)
  timeout: 30                # Request timeout in seconds
  max_tokens: 4000           # Maximum tokens per request
  temperature: 0.1           # Model temperature (0.0-1.0)
```

**Supported Providers:**
- `openai` - OpenAI GPT models
- `anthropic` - Anthropic Claude models
- `local` - Local/self-hosted models

**Environment Variables:**
- `OPENAI_API_KEY` - OpenAI API key
- `ANTHROPIC_API_KEY` - Anthropic API key

### Scanner Configuration

Control scanner behavior and performance:

```yaml
scanner:
  http_timeout: 30           # HTTP request timeout in seconds
  scan_timeout: 60           # Total scan timeout in seconds
  detailed: false            # Enable detailed analysis
  format: "table"            # Default output format (table, json, raw)
  parallel: true             # Enable parallel processing
  max_retries: 3             # Maximum retry attempts
  retry_delay_ms: 1000       # Delay between retries in milliseconds
  llm_batch_size: 10         # Batch size for LLM requests
  enable_yara: true          # Enable/disable YARA-X scanning
```

**Performance Tuning:**
- Increase `llm_batch_size` for faster scanning of large servers
- Reduce `http_timeout` for faster timeouts on unresponsive servers
- Set `parallel: false` to reduce resource usage

### Security Configuration

Configure security checks and thresholds:

```yaml
security:
  enabled: true              # Enable security scanning
  min_severity: "low"        # Minimum severity level (low, medium, high, critical)
  checks:
    tool_poisoning: true           # Detect tool poisoning attacks
    secrets_leakage: true          # Detect secret leakage
    sql_injection: true            # Detect SQL injection vulnerabilities
    command_injection: true        # Detect command injection vulnerabilities
    path_traversal: true           # Detect path traversal vulnerabilities
    auth_bypass: true              # Detect authentication bypass issues
    cross_origin_escalation: true  # Detect cross-origin escalation
    prompt_injection: true         # Detect prompt injection vulnerabilities
    pii_leakage: true             # Detect PII leakage
    jailbreak: true               # Detect jailbreak attempts
```

**Severity Levels:**
- `low` - Show all findings
- `medium` - Show medium, high, and critical findings
- `high` - Show only high and critical findings
- `critical` - Show only critical findings

### Logging Configuration

Configure logging output:

```yaml
logging:
  level: "info"              # Log level (debug, info, warn, error)
  colored: true              # Enable colored output
  timestamps: true           # Include timestamps in logs
```

**Log Levels:**
- `debug` - Detailed debugging information
- `info` - General information messages
- `warn` - Warning messages
- `error` - Error messages only

### Performance Configuration

Track and optimize performance:

```yaml
performance:
  tracking: true             # Enable performance tracking
  slow_threshold_ms: 5000    # Threshold for slow operation warnings
```

## YARA-X Configuration

### Enabling/Disabling YARA-X

Control YARA-X scanning via configuration:

```yaml
scanner:
  enable_yara: true          # Enable YARA-X scanning
```

Set to `false` to disable YARA-X scanning if needed.

### YARA Rules

**Rules Directory Structure:**
```
rules/
├── pre/                    # Pre-scan rules
│   ├── secrets.yar
│   ├── injection.yar
│   └── custom.yar
└── post/                   # Post-scan rules
    ├── analysis.yar
    └── reporting.yar
```

**Built-in Rules:**
- `secrets_leakage` - Detect hardcoded secrets and API keys
- `command_injection` - Detect command injection patterns
- `path_traversal` - Detect path traversal vulnerabilities
- `sql_injection` - Detect SQL injection patterns
- `cross_origin_escalation` - Detect cross-domain issues

**Custom Rules:**
Create `.yar` files in `rules/pre/` or `rules/post/` directories. YARA-X supports standard YARA syntax with 99% compatibility.

Example custom rule:
```yara
rule CustomSecretPattern
{
    meta:
        description = "Detect custom secret patterns"
        severity = "high"
        
    strings:
        $secret1 = /secret_key_[a-zA-Z0-9]{32}/
        $secret2 = /api_token_[a-zA-Z0-9]{40}/
        
    condition:
        any of them
}
```

## Environment Variables

Ramparts supports environment variables for sensitive configuration:

```bash
# API Keys
export OPENAI_API_KEY="your-openai-key"
export ANTHROPIC_API_KEY="your-anthropic-key"

# Configuration override
export RAMPARTS_CONFIG="/path/to/custom/ramparts.yaml"

# Logging
export RAMPARTS_LOG_LEVEL="debug"
export RAMPARTS_LOG_COLORED="true"
```

## Configuration File Locations

Ramparts searches for configuration files in this order:

1. File specified with `--config` flag
2. `RAMPARTS_CONFIG` environment variable
3. `ramparts.yaml` in current directory
4. `~/.config/ramparts/ramparts.yaml`
5. `/etc/ramparts/ramparts.yaml`

## Configuration Examples

### High-Security Configuration

```yaml
scanner:
  enable_yara: true
  detailed: true
  
security:
  enabled: true
  min_severity: "medium"
  checks:
    tool_poisoning: true
    secrets_leakage: true
    sql_injection: true
    command_injection: true
    path_traversal: true
    auth_bypass: true
    cross_origin_escalation: true
    prompt_injection: true
    pii_leakage: true
    jailbreak: true

logging:
  level: "debug"
```

### Performance-Optimized Configuration

```yaml
scanner:
  parallel: true
  llm_batch_size: 20
  max_retries: 1
  retry_delay_ms: 500
  
security:
  min_severity: "high"
  
logging:
  level: "warn"
```

### Development Configuration

```yaml
scanner:
  http_timeout: 10
  detailed: true
  enable_yara: false
  
security:
  min_severity: "low"
  
logging:
  level: "debug"
  colored: true
```

## Validation

Validate your configuration file:

```bash
# Test configuration
ramparts scan --config ramparts.yaml --help

# Debug configuration loading
RAMPARTS_LOG_LEVEL=debug ramparts scan <url>
```

## Best Practices

### Security
- Store API keys in environment variables, not config files
- Use appropriate minimum severity levels for your use case
- Enable all relevant security checks for comprehensive scanning

### Performance
- Tune `llm_batch_size` based on your rate limits
- Adjust timeouts based on your network conditions
- Use parallel processing for faster scans

### Maintenance
- Keep configuration files in version control (without secrets)
- Document custom YARA rules
- Review and update configurations regularly

## Troubleshooting Configuration

**Configuration not loading:**
```bash
# Check file permissions
ls -la ramparts.yaml

# Validate YAML syntax
python -c "import yaml; yaml.safe_load(open('ramparts.yaml'))"
```

**YARA rules not working:**
```bash
# Check rules directory
ls -la rules/pre/

# Validate rule syntax
# YARA-X will show compilation errors
```

**API key issues:**
```bash
# Check environment variables
echo $OPENAI_API_KEY

# Test API connectivity
curl -H "Authorization: Bearer $OPENAI_API_KEY" https://api.openai.com/v1/models
```

## Next Steps

- Learn about [security features](security-features.md) that can be configured
- Check the [troubleshooting guide](troubleshooting.md) for common issues
- Review [usage examples](usage.md) with custom configurations