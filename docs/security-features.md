# Security Features & Threat Model

## Overview

Ramparts is designed to identify security vulnerabilities in Model Context Protocol (MCP) servers before they can be exploited by malicious actors. This document explains the security challenges in the MCP ecosystem and how Ramparts addresses them.

## The MCP Security Challenge

### What is MCP?

The Model Context Protocol (MCP) is an open standard that enables AI assistants to securely connect to external data sources and tools. It allows AI agents to access databases, file systems, and APIs through tool calling to retrieve real-time information and interact with external or internal services.

### Why MCP Security Matters

MCP servers can expose powerful capabilities to AI agents, including:

- **File System Access** - Read/write files, directory operations, file management
- **Database Operations** - SQL queries, data manipulation, schema access
- **API Integrations** - External service calls, authentication, data exchange
- **System Commands** - Process execution, system administration, shell access

### Attack Vectors

Without proper security analysis, these capabilities become attack vectors for:

#### Tool Poisoning
Bypassing AI safety measures by manipulating tool descriptions or behaviors to make malicious actions appear benign.

#### MCP Rug Pulls
Unauthorized changes to MCP tool descriptions after initial user approval, effectively changing the tool's behavior without user consent.

#### Cross-Origin Escalation
Exploiting tools that span multiple domains to hijack context or inject malicious content from untrusted sources.

#### Data Exfiltration
Leaking sensitive information through tool parameters, responses, or side channels.

#### Privilege Escalation
Gaining unauthorized access to resources or capabilities beyond intended scope.

#### Path Traversal Attacks
Accessing files or directories outside intended boundaries through manipulated file paths.

#### Command Injection
Executing unauthorized system commands through improperly validated input parameters.

#### SQL Injection
Manipulating database queries to access unauthorized data or perform unintended operations.

## Ramparts Security Analysis

### 1. Capability Discovery

Ramparts performs comprehensive discovery of MCP server capabilities:

- **Endpoint Scanning** - Analyzes all MCP endpoints (`server/info`, `tools/list`, `resources/list`, `prompts/list`)
- **Tool Enumeration** - Catalogs all available tools with their parameters and descriptions
- **Resource Mapping** - Identifies accessible resources and their access patterns
- **Prompt Analysis** - Examines available prompts for security implications

### 2. Static Analysis

Ramparts performs static analysis using multiple techniques:

#### YARA-X Integration
- **Pattern-Based Detection** - Uses YARA-X rules to detect known vulnerability patterns
- **Custom Rules** - Supports custom security rules for organization-specific threats
- **Built-in Rule Sets** - Includes rules for common vulnerabilities and attack patterns

#### Code Analysis
- **Parameter Validation** - Checks for missing or insufficient input validation
- **Schema Analysis** - Examines tool schemas for security anti-patterns
- **Metadata Review** - Analyzes tool metadata for security-relevant information

### 3. Cross-Origin Analysis

Specialized detection for cross-origin escalation attacks:

#### Domain Analysis
- **URL Extraction** - Extracts and normalizes domains from tool parameters and schemas
- **Cross-Domain Detection** - Identifies tools that span multiple domains
- **Security Scheme Analysis** - Detects mixed HTTP/HTTPS usage patterns

#### Contamination Detection
- **Domain Clustering** - Groups tools by domain to identify outliers
- **Trust Boundary Analysis** - Identifies tools crossing trust boundaries
- **Mixed Source Detection** - Flags tools mixing trusted and untrusted sources

### 4. LLM-Powered Analysis

Advanced AI-based security analysis:

#### Semantic Analysis
- **Intent Detection** - Analyzes tool descriptions for malicious intent
- **Context Understanding** - Evaluates security implications in context
- **Behavior Prediction** - Predicts potential misuse scenarios

#### Natural Language Processing
- **Description Analysis** - Examines tool descriptions for security red flags
- **Parameter Analysis** - Analyzes parameter names and descriptions for risks
- **Documentation Review** - Reviews any available documentation for security issues

### 5. Risk Assessment

Comprehensive risk scoring and categorization:

#### Severity Classification
- **CRITICAL** - Immediate security risk requiring urgent attention
- **HIGH** - Significant security risk that should be addressed promptly
- **MEDIUM** - Moderate security risk that should be reviewed
- **LOW** - Minor security concern for awareness

#### Risk Factors
- **Impact Assessment** - Evaluates potential damage from exploitation
- **Exploitability Analysis** - Assesses ease of exploitation
- **Scope Analysis** - Determines affected systems and data

## Security Checks

### Tool Poisoning Detection

Identifies attempts to manipulate tool behavior:

```yaml
security:
  checks:
    tool_poisoning: true
```

**Detects:**
- Misleading tool descriptions
- Hidden or obfuscated functionality
- Tools with excessive permissions
- Suspicious parameter patterns

### Secrets Leakage Detection

Identifies hardcoded secrets and sensitive data exposure:

```yaml
security:
  checks:
    secrets_leakage: true
```

**Detects:**
- API keys and tokens in tool descriptions
- Database credentials in connection strings
- Hardcoded passwords or secrets
- PII in tool metadata

### Injection Vulnerability Detection

Identifies various injection attack vectors:

```yaml
security:
  checks:
    sql_injection: true
    command_injection: true
    prompt_injection: true
```

**SQL Injection Detects:**
- Unsafe SQL query construction
- Missing parameterized queries
- Dynamic query building patterns
- Database connection vulnerabilities

**Command Injection Detects:**
- Unsafe system command execution
- Shell injection vulnerabilities
- Process execution without validation
- File system command risks

**Prompt Injection Detects:**
- Prompt manipulation attempts
- Context hijacking patterns
- Input sanitization bypass
- LLM jailbreak techniques

### Path Traversal Detection

Identifies file system access vulnerabilities:

```yaml
security:
  checks:
    path_traversal: true
```

**Detects:**
- Unrestricted file path access
- Directory traversal patterns (`../`)
- Absolute path vulnerabilities
- File system boundary violations

### Authentication Bypass Detection

Identifies authentication and authorization issues:

```yaml
security:
  checks:
    auth_bypass: true
```

**Detects:**
- Missing authentication checks
- Weak authentication mechanisms
- Authorization bypass patterns
- Privilege escalation opportunities

### Cross-Origin Escalation Detection

Specialized detection for cross-domain security issues:

```yaml
security:
  checks:
    cross_origin_escalation: true
```

**Detects:**
- Tools spanning multiple domains
- Mixed security schemes (HTTP/HTTPS)
- Domain contamination patterns
- Trust boundary violations

### PII Leakage Detection

Identifies personally identifiable information exposure:

```yaml
security:
  checks:
    pii_leakage: true
```

**Detects:**
- Personal information in tool descriptions
- Sensitive data in parameters
- Privacy policy violations
- Data classification issues

## Security Best Practices

### For MCP Server Users

1. **Regular Scanning** - Scan MCP servers before connecting and periodically thereafter
2. **Severity Thresholds** - Set appropriate minimum severity levels for your risk tolerance
3. **Authentication Security** - Use secure authentication methods and rotate credentials
4. **Network Security** - Prefer HTTPS connections and secure network configurations

### For MCP Server Developers

1. **Input Validation** - Implement comprehensive input validation for all parameters
2. **Principle of Least Privilege** - Grant minimal necessary permissions
3. **Secure Defaults** - Use secure configurations by default
4. **Regular Testing** - Integrate Ramparts scanning into development workflow

### For Security Teams

1. **Policy Enforcement** - Establish MCP security policies and scanning requirements
2. **Continuous Monitoring** - Implement ongoing security monitoring of MCP deployments
3. **Incident Response** - Develop procedures for responding to security findings
4. **Training** - Educate developers on MCP security best practices

## Limitations and Considerations

### Layered Security Approach

Ramparts is designed to work on MCP server metadata and provides static analysis. Consider a layered approach to security:

- **Static Analysis** - Use Ramparts for pre-deployment security scanning
- **Runtime Protection** - Implement runtime MCP guardrails and monitoring
- **Access Controls** - Enforce proper authentication and authorization
- **Network Security** - Use secure network configurations and monitoring

### Evolving Threat Landscape

Both the MCP standard and AI/MCP threat landscape are evolving rapidly. There may be threats or attack vectors that Ramparts may not yet detect. Stay informed about:

- New MCP security research and findings
- Updates to MCP protocol specifications
- Emerging AI security threats and mitigations
- Ramparts updates and new detection capabilities

### False Positives and Negatives

Like all security tools, Ramparts may produce:

- **False Positives** - Flagging legitimate functionality as potentially malicious
- **False Negatives** - Missing actual security vulnerabilities

Always review findings in context and supplement with additional security measures.

## Getting Help

For questions about security features or to report potential security issues:

- Review our [configuration guide](configuration.md) for security settings
- Check the [troubleshooting guide](troubleshooting.md) for common issues
- Open an issue on [GitHub](https://github.com/getjavelin/ramparts/issues)
- Contact support@getjavelin.com for runtime MCP guardrails