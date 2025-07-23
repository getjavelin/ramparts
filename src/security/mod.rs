// Security scan types used by the rest of the codebase

use crate::types::{MCPTool, MCPPrompt, MCPResource};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use spinners::{Spinner, Spinners};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SecurityIssueType {
    ToolPoisoning,
    SQLInjection,
    CommandInjection,
    PathTraversal,
    AuthBypass,
    PromptInjection,
    Jailbreak,
    PIILeakage,
    SecretsLeakage,
}

impl SecurityIssueType {
    fn default_severity(&self) -> &'static str {
        match self {
            SecurityIssueType::ToolPoisoning => "CRITICAL",
            SecurityIssueType::SQLInjection => "CRITICAL",
            SecurityIssueType::CommandInjection => "CRITICAL",
            SecurityIssueType::PathTraversal => "HIGH",
            SecurityIssueType::AuthBypass => "CRITICAL",
            SecurityIssueType::PromptInjection => "HIGH",
            SecurityIssueType::Jailbreak => "HIGH",
            SecurityIssueType::PIILeakage => "MEDIUM",
            SecurityIssueType::SecretsLeakage => "HIGH",
        }
    }

    fn default_message(&self) -> &'static str {
        match self {
            SecurityIssueType::ToolPoisoning => "Tool with destructive or malicious intent",
            SecurityIssueType::SQLInjection => "Tool allowing SQL injection attacks",
            SecurityIssueType::CommandInjection => "Tool that may execute system commands",
            SecurityIssueType::PathTraversal => "Tool allowing directory traversal attacks",
            SecurityIssueType::AuthBypass => "Tool allowing unauthorized access",
            SecurityIssueType::PromptInjection => "Prompt vulnerable to injection attacks",
            SecurityIssueType::Jailbreak => "Prompt that could bypass AI safety measures",
            SecurityIssueType::PIILeakage => "Tool processing personal information",
            SecurityIssueType::SecretsLeakage => "Tool processing sensitive credentials",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityIssue {
    pub issue_type: SecurityIssueType,
    pub tool_name: Option<String>,
    pub prompt_name: Option<String>,
    pub resource_uri: Option<String>,
    pub description: String,
    pub details: Option<String>,
    pub severity: String,
    pub message: String,
}

impl SecurityIssue {
    pub fn new(issue_type: SecurityIssueType, description: String) -> Self {
        let message = format!("{}: {}", issue_type.default_message(), &description);
        Self {
            issue_type: issue_type.clone(),
            tool_name: None,
            prompt_name: None,
            resource_uri: None,
            description,
            details: None,
            severity: issue_type.default_severity().to_string(),
            message,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityScanResult {
    pub tool_issues: Vec<SecurityIssue>,
    pub prompt_issues: Vec<SecurityIssue>,
    pub resource_issues: Vec<SecurityIssue>,
    pub tool_analysis_details: std::collections::HashMap<String, String>, // tool_name -> LLM analysis details
}

impl SecurityScanResult {
    pub fn new() -> Self {
        Self {
            tool_issues: Vec::new(),
            prompt_issues: Vec::new(),
            resource_issues: Vec::new(),
            tool_analysis_details: std::collections::HashMap::new(),
        }
    }

    pub fn add_tool_issues(&mut self, issues: Vec<SecurityIssue>) {
        self.tool_issues.extend(issues);
    }

    pub fn add_prompt_issues(&mut self, issues: Vec<SecurityIssue>) {
        self.prompt_issues.extend(issues);
    }

    pub fn add_resource_issues(&mut self, issues: Vec<SecurityIssue>) {
        self.resource_issues.extend(issues);
    }

    pub fn add_tool_analysis_details(&mut self, tool_name: String, details: String) {
        self.tool_analysis_details.insert(tool_name, details);
    }

    pub fn total_issues(&self) -> usize {
        self.tool_issues.len() + self.prompt_issues.len() + self.resource_issues.len()
    }

    pub fn has_critical_issues(&self) -> bool {
        self.tool_issues.iter().any(|issue| issue.severity == "CRITICAL") ||
        self.prompt_issues.iter().any(|issue| issue.severity == "CRITICAL") ||
        self.resource_issues.iter().any(|issue| issue.severity == "CRITICAL")
    }

    pub fn has_high_issues(&self) -> bool {
        self.tool_issues.iter().any(|issue| issue.severity == "HIGH") ||
        self.prompt_issues.iter().any(|issue| issue.severity == "HIGH") ||
        self.resource_issues.iter().any(|issue| issue.severity == "HIGH")
    }
}

/// Security scanner for MCP tools, prompts, and resources using LLM-based detection
pub struct SecurityScanner {
    pub model_endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model_name: String,
    pub config: Option<crate::config::ScannerConfig>,
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self {
            model_endpoint: Some("https://api.openai.com/v1/chat/completions".to_string()),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            model_name: "gpt-4o".to_string(),
            config: None,
        }
    }
}

impl SecurityScanner {
    /// Create a new SecurityScanner with configuration
    pub fn with_config(config: crate::config::ScannerConfig) -> Self {
        let api_key = if !config.llm.api_key.is_empty() {
            Some(config.llm.api_key.clone())
        } else {
            std::env::var("OPENAI_API_KEY").ok()
        };
        
        let model_endpoint = Some(format!("{}/chat/completions", config.llm.base_url));
        
        Self {
            model_endpoint,
            api_key,
            model_name: config.llm.model.clone(),
            config: Some(config),
        }
    }
    
    /// Check if LLM is configured
    fn is_llm_configured(&self) -> bool {
        self.model_endpoint.is_some() && self.api_key.is_some()
    }

    /// Batch scan multiple tools for security vulnerabilities
    pub async fn scan_tools_batch(&self, tools: &[MCPTool], show_details: bool) -> Result<(Vec<SecurityIssue>, std::collections::HashMap<String, String>)> {
        if tools.is_empty() {
            tracing::debug!("No tools to scan, returning empty result");
            return Ok((Vec::new(), std::collections::HashMap::new()));
        }
        
        if !self.is_llm_configured() {
            tracing::debug!("OpenAI API not configured, returning empty result");
            return Ok((Vec::new(), std::collections::HashMap::new()));
        }
        
        // Get batch size from config, default to 10 if not configured
        let batch_size = self.config.as_ref()
            .map(|c| c.scanner.llm_batch_size)
            .unwrap_or(10);
        
        tracing::debug!("Starting batch scan of {} tools in batches of {}", tools.len(), batch_size);
        
        let mut all_issues = Vec::new();
        let mut all_analysis_details = std::collections::HashMap::new();
        
        // Process tools in batches
        for (batch_index, chunk) in tools.chunks(batch_size as usize).enumerate() {
            tracing::debug!("Processing batch {} with {} tools", batch_index + 1, chunk.len());
            
            let tools_info = self.format_tools_info(chunk);
            let prompt = self.create_tools_analysis_prompt(&tools_info);
            
            tracing::debug!("Sending batch LLM request for batch {} ({} tools)", batch_index + 1, chunk.len());
            let response = self.query_llm(&prompt, show_details).await?;
            tracing::debug!("Received batch LLM response for batch {}: {}", batch_index + 1, response);
            
            let (issues, analysis_details) = self.parse_batch_llm_response(&response).await?;
            all_issues.extend(issues);
            all_analysis_details.extend(analysis_details);
        }
        
        tracing::debug!("Completed batch scan with {} total issues", all_issues.len());
        Ok((all_issues, all_analysis_details))
    }

    /// Batch scan prompts for security vulnerabilities
    pub async fn scan_prompts_batch(&self, prompts: &[MCPPrompt], show_details: bool) -> Result<Vec<SecurityIssue>> {
        if prompts.is_empty() {
            tracing::debug!("No prompts to scan, returning empty result");
            return Ok(Vec::new());
        }
        
        if !self.is_llm_configured() {
            tracing::debug!("OpenAI API not configured, returning empty result");
            return Ok(Vec::new());
        }
        
        // Get batch size from config, default to 10 if not configured
        let batch_size = self.config.as_ref()
            .map(|c| c.scanner.llm_batch_size)
            .unwrap_or(10);
        
        tracing::debug!("Starting batch scan of {} prompts in batches of {}", prompts.len(), batch_size);
        
        let mut all_issues = Vec::new();
        
        // Process prompts in batches
        for (batch_index, chunk) in prompts.chunks(batch_size as usize).enumerate() {
            tracing::debug!("Processing prompt batch {} with {} prompts", batch_index + 1, chunk.len());
            
            let prompts_info = self.format_prompts_info(chunk);
            let prompt_text = self.create_prompts_analysis_prompt(&prompts_info);
            
            tracing::debug!("Sending batch LLM request for prompt batch {} ({} prompts)", batch_index + 1, chunk.len());
            let response = self.query_llm(&prompt_text, show_details).await?;
            tracing::debug!("Received batch LLM response for prompt batch {}: {}", batch_index + 1, response);
            
            let (issues, _) = self.parse_batch_llm_response(&response).await?;
            all_issues.extend(issues);
        }
        
        tracing::debug!("Completed prompt batch scan with {} total issues", all_issues.len());
        Ok(all_issues)
    }

    /// Batch scan resources for security vulnerabilities
    pub async fn scan_resources_batch(&self, resources: &[MCPResource], show_details: bool) -> Result<Vec<SecurityIssue>> {
        if resources.is_empty() {
            tracing::debug!("No resources to scan, returning empty result");
            return Ok(Vec::new());
        }
        
        if !self.is_llm_configured() {
            tracing::debug!("OpenAI API not configured, returning empty result");
            return Ok(Vec::new());
        }
        
        // Get batch size from config, default to 10 if not configured
        let batch_size = self.config.as_ref()
            .map(|c| c.scanner.llm_batch_size)
            .unwrap_or(10);
        
        tracing::debug!("Starting batch scan of {} resources in batches of {}", resources.len(), batch_size);
        
        let mut all_issues = Vec::new();
        
        // Process resources in batches
        for (batch_index, chunk) in resources.chunks(batch_size as usize).enumerate() {
            tracing::debug!("Processing resource batch {} with {} resources", batch_index + 1, chunk.len());
            
            let resources_info = self.format_resources_info(chunk);
            let prompt_text = self.create_resources_analysis_prompt(&resources_info);
            
            tracing::debug!("Sending batch LLM request for resource batch {} ({} resources)", batch_index + 1, chunk.len());
            let response = self.query_llm(&prompt_text, show_details).await?;
            tracing::debug!("Received batch LLM response for resource batch {}: {}", batch_index + 1, response);
            
            let (issues, _) = self.parse_batch_llm_response(&response).await?;
            all_issues.extend(issues);
        }
        
        tracing::debug!("Completed resource batch scan with {} total issues", all_issues.len());
        Ok(all_issues)
    }

    /// Format tools information for LLM analysis
    fn format_tools_info(&self, tools: &[MCPTool]) -> String {
        let mut tools_info = String::new();
        for (i, tool) in tools.iter().enumerate() {
            // Create a concise summary of the input schema
            let input_summary = if let Some(schema) = &tool.input_schema {
                if let Some(properties) = schema.get("properties") {
                    if let Some(props_obj) = properties.as_object() {
                        let param_names: Vec<&str> = props_obj.keys().map(|k| k.as_str()).collect();
                        format!("Parameters: {}", param_names.join(", "))
                    } else {
                        "Parameters: complex schema".to_string()
                    }
                } else {
                    "Parameters: no properties".to_string()
                }
            } else {
                "Parameters: no schema".to_string()
            };
            
            tools_info.push_str(&format!(
                "\n\nTOOL {}: {}\nDescription: {}\nCategory: {}\nTags: {}\n{}",
                i + 1,
                tool.name,
                tool.description.as_deref().unwrap_or("No description"),
                tool.category.as_deref().unwrap_or("No category"),
                tool.tags.join(", "),
                input_summary
            ));
        }
        tools_info
    }

    /// Format prompts information for LLM analysis
    fn format_prompts_info(&self, prompts: &[MCPPrompt]) -> String {
        let mut prompts_info = String::new();
        for (i, prompt) in prompts.iter().enumerate() {
            let arguments = prompt.arguments.as_ref()
                .map(|args| serde_json::to_string_pretty(args).ok().unwrap_or_default())
                .unwrap_or_else(|| "No arguments".to_string());
            
            prompts_info.push_str(&format!(
                "\n\nPROMPT {}: {}\nDescription: {}\nArguments: {}",
                i + 1,
                prompt.name,
                prompt.description.as_deref().unwrap_or("No description"),
                arguments
            ));
        }
        prompts_info
    }

    /// Format resources information for LLM analysis
    fn format_resources_info(&self, resources: &[MCPResource]) -> String {
        let mut resources_info = String::new();
        for (i, resource) in resources.iter().enumerate() {
            resources_info.push_str(&format!(
                "\n\nRESOURCE {}: {}\nURI: {}\nDescription: {}",
                i + 1,
                resource.name,
                resource.uri,
                resource.description.as_deref().unwrap_or("No description")
            ));
        }
        resources_info
    }

    /// Create tools analysis prompt
    fn create_tools_analysis_prompt(&self, tools_info: &str) -> String {
        format!(
            "### ROLE\n\
You are a senior Application Security Engineer performing a static review of **MCP tool definitions**. MCP tools are typically authenticated through the MCP server context, not individual tool parameters.\n\n\
### TOOLS TO ANALYZE\n\
{}\n\n\
### GOAL\n\
Find **genuine** security risks in each tool definition and report them.\n\n\
### IMPORTANT CONTEXT\n\
- MCP tools inherit authentication from the MCP server context (headers, tokens, etc.)\n\
- Do NOT flag tools as AuthBypass just because they don't have explicit auth parameters\n\
- Focus on actual security vulnerabilities in the tool's functionality and parameters\n\
- Consider the tool's intended purpose and whether it's being used appropriately\n\n\
### SECURITY ANALYSIS FOCUS\n\
For MCP tools, pay special attention to:\n\
- **PathTraversal**: Tools that accept file paths - can user input be manipulated to access files outside intended directories? ONLY flag if the tool lacks proper path validation or allows accessing system files outside its intended scope.\n\
- **CommandInjection**: Tools that execute commands or scripts - do they properly sanitize user inputs?\n\
- **SecretsLeakage**: Tools that handle or return sensitive data (tokens, keys, credentials) - could they expose this data?\n\
- **PIILeakage**: Tools that process personal information - is data properly protected or could it be exposed?\n\
- **ToolPoisoning**: Tools with names/descriptions that suggest destructive or malicious intent\n\n\
### LEGITIMATE TOOL EXAMPLES (do NOT flag these as security issues):\n\
- **push_files**: Legitimate Git/GitHub functionality for pushing files to repositories (normal version control)\n\
- **create_file/update_file**: Standard file operations for development workflows\n\
- **read_file/write_file**: Normal file I/O operations for development tools\n\
- **git_* tools**: Standard Git operations (commit, push, pull, etc.)\n\
- **file_* tools**: Standard file system operations for development workflows\n\
- Any tool that performs normal development, deployment, or automation tasks\n\n\
### ANALYSIS REQUIREMENTS\n\
For each tool, provide SPECIFIC analysis based on:\n\
1. **Tool name and description** - What does it do? Is the purpose legitimate?\n\
2. **Parameters** - What inputs does it accept? Can any be manipulated maliciously?\n\
3. **Functionality** - What actions does it perform? Are there actual security risks?\n\
4. **Context** - How is it used in the MCP server's ecosystem?\n\n\
### RISK CATEGORIES (use these exact `issue_type` values)\n\
- **ToolPoisoning** – Name/description implies destructive or malicious purpose (e.g., \"wipe_db\", \"ddos_site\").\n\
- **SQLInjection**  – Untrusted input concatenated into SQL or passed to DB driver without bind params.\n\
- **CommandInjection** – Untrusted input incorporated into system shell/exec calls.\n\
- **PathTraversal**  – User-supplied path can reach `../` or absolute locations outside intended dir. ONLY flag if the tool lacks proper path validation or allows accessing system files outside its intended scope.\n\
- **AuthBypass**   – Tool explicitly bypasses authentication or has clear auth vulnerabilities.\n\
- **PromptInjection** – Free-form text inserted into another LLM prompt without sanitization or role isolation.\n\
- **PIILeakage**   – Tool processes or returns personal data (emails, SSNs, etc.) without strict need or masking.\n\
- **SecretsLeakage** – API keys, tokens, passwords hard-coded, logged, or returned to caller.\n\n\
### SEVERITY\n\
LOW | MEDIUM | HIGH | CRITICAL\n\n\
### OUTPUT\n\
Return **only** a JSON array where each element has this schema **and property order**:\n\n\
{{\n  \"tool_name\": \"<string>\",\n  \"found_issue\": <true|false>,\n  \"issues\": [\n    {{\n      \"issue_type\": \"<enum above>\",\n      \"severity\": \"<LOW|MEDIUM|HIGH|CRITICAL>\",\n      \"message\": \"<≤100 chars>\",\n      \"details\": \"<1–3 sentences>\"\n    }}\n  ],\n  \"details\": \"<SPECIFIC analysis of this tool's security posture based on its parameters and functionality>\"\n}}\n\n\
IMPORTANT: Be realistic and accurate. Only flag genuine security issues. Normal API functionality should not be flagged as security vulnerabilities.\n",
            tools_info
        )
    }

    /// Create prompts analysis prompt
    fn create_prompts_analysis_prompt(&self, prompts_info: &str) -> String {
        format!(
            "Analyze these MCP prompts for ALL potential security vulnerabilities in a single comprehensive assessment.

Prompts to analyze:{}

Check each prompt for these security issues:
1. PROMPT INJECTION: Does the prompt description suggest ignoring previous instructions or overriding system safety measures?
2. JAILBREAK: Does the prompt suggest bypassing AI safety measures or restrictions?
3. PII LEAKAGE: Does the prompt explicitly handle or request personal information like emails, addresses, SSNs, credit cards?

IMPORTANT CONTEXT:
- MCP prompts are legitimate API functionality for interacting with services
- Do NOT flag normal automation, task assignment, or workflow prompts as security issues
- Only flag prompts that explicitly request sensitive personal data or suggest bypassing security measures
- Consider the prompt's intended purpose and whether it's legitimate functionality

EXAMPLES OF LEGITIMATE PROMPTS (do NOT flag):
- AssignCodingAgent: Assigns coding tasks to AI agents (normal automation)
- Workflow triggers: Normal CI/CD and automation functionality
- Task management: Assigning tasks, creating issues, etc.
- Any prompt that automates legitimate business processes

EXAMPLES OF SUSPICIOUS PROMPTS (DO flag):
- Prompts that explicitly request personal data (emails, SSNs, etc.)
- Prompts that suggest bypassing authentication or security
- Prompts with malicious names like \"bypass_safety\" or \"extract_personal_data\"
- Prompts that explicitly try to extract or expose sensitive information

CRITICAL: If a prompt is doing normal automation or task management, it should NOT be flagged as a security issue. Only flag prompts that are clearly malicious or explicitly designed to bypass security measures.

MCP PROMPTS ARE LEGITIMATE API FUNCTIONALITY:
- AssignCodingAgent is a legitimate GitHub Copilot feature for automating coding tasks
- Task assignment prompts are normal business automation
- Workflow triggers are standard CI/CD functionality
- These are NOT security vulnerabilities - they are intended features

ONLY flag prompts that are:
1. Explicitly malicious (e.g., \"bypass_safety\", \"extract_personal_data\")
2. Clearly designed to override security measures
3. Explicitly request sensitive personal information
4. Have names/descriptions that suggest malicious intent

Respond with a JSON array of issues, each with:
- prompt_name: The name of the prompt with the issue
- issue_type: 'PromptInjection', 'Jailbreak', or 'PIILeakage'
- severity: 'LOW', 'MEDIUM', 'HIGH', or 'CRITICAL'
- message: Brief description of the issue
- details: More detailed explanation

If no genuine security issues found, return empty array [].",
            prompts_info
        )
    }

    /// Create resources analysis prompt
    fn create_resources_analysis_prompt(&self, resources_info: &str) -> String {
        format!(
            "Analyze these MCP resources for ALL potential security vulnerabilities in a single comprehensive assessment.

Resources to analyze:{}

Check each resource for these security issues:
1. PATH TRAVERSAL: Does the resource URI contain path traversal patterns that could access unauthorized files?
2. SENSITIVE DATA EXPOSURE: Does the resource contain API keys, tokens, credentials, or other sensitive information?

IMPORTANT: Only flag issues that pose REAL security risks. Normal resource functionality should not be flagged as security issues.

Respond with a JSON array of issues, each with:
- resource_name: The name of the resource with the issue
- issue_type: 'PathTraversal' or 'SecretsLeakage'
- severity: 'LOW', 'MEDIUM', 'HIGH', or 'CRITICAL'
- message: Brief description of the issue
- details: More detailed explanation

If no genuine security issues found, return empty array [].",
            resources_info
        )
    }

    /// Query the LLM with the given prompt
    async fn query_llm(&self, prompt: &str, show_details: bool) -> Result<String> {
        let client = Client::new();
        
        // Get configuration values, with defaults if not configured
        let temperature = self.config.as_ref()
            .map(|c| c.llm.temperature)
            .unwrap_or(0.1);
        let max_tokens = self.config.as_ref()
            .map(|c| c.llm.max_tokens)
            .unwrap_or(4000);
        let timeout = self.config.as_ref()
            .map(|c| c.llm.timeout)
            .unwrap_or(30);
        
        let request_body = json!({
            "model": self.model_name,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a security analyst specializing in detecting vulnerabilities in MCP (Model Context Protocol) tools, prompts, and resources. Your job is to identify potential security risks, even if they seem minor. Look for any security issues that could be exploited or lead to unauthorized access.

CRITICAL: You must respond with ONLY a valid JSON array. Do not include any explanatory text, markdown formatting, or other content outside the JSON array. 

IMPORTANT: You must analyze EVERY tool and include it in your response, even if no security issues are found. For tools with no issues, set found_issue: false and provide details about why no issues were found.

Example valid response: [{\"tool_name\": \"example\", \"found_issue\": true, \"issues\": [{\"issue_type\": \"SQLInjection\", \"severity\": \"HIGH\", \"message\": \"Brief description\", \"details\": \"Detailed explanation\"}], \"details\": \"Additional context\"}]"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": temperature,
            "max_tokens": max_tokens
        });
        
        if show_details {
            println!("\n🔍 LLM Request:");
            println!("{}", serde_json::to_string_pretty(&request_body).unwrap_or_default());
        }
        
        // Start spinner with text after animation
        let mut sp = Spinner::new(Spinners::Dots9, "Scanning for security vulnerabilities...(this may take a while)".into());
        
        let response = client
            .post(self.model_endpoint.as_ref().unwrap())
            .header("Authorization", format!("Bearer {}", self.api_key.as_ref().unwrap()))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(timeout))
            .json(&request_body)
            .send()
            .await?;
        
        // Stop spinner
        sp.stop();
        
        if !response.status().is_success() {
            return Err(anyhow!("LLM API request failed: {}", response.status()));
        }
        
        let response_json: Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid LLM response format"))?;
        
        if show_details {
            println!("\n🤖 LLM Response:");
            println!("{}", content);
        }
        
        Ok(content.to_string())
    }

    /// Parse batch LLM response for multiple tools, prompts, and resources
    async fn parse_batch_llm_response(&self, response: &str) -> Result<(Vec<SecurityIssue>, std::collections::HashMap<String, String>)> {
        let issues_array = self.extract_json_array(response)?;
        let mut issues = Vec::new();
        let mut analysis_details = std::collections::HashMap::new();
        
        tracing::debug!("Parsing {} tools from LLM response", issues_array.len());
        
        for tool_value in issues_array {
            // Handle new format with found_issue and issues array
            if let Some(tool_name) = tool_value["tool_name"].as_str() {
                if let Some(found_issue) = tool_value["found_issue"].as_bool() {
                    tracing::debug!("Tool {}: found_issue = {}", tool_name, found_issue);
                    
                    // Store the analysis details for this tool
                    if let Some(details) = tool_value["details"].as_str() {
                        analysis_details.insert(tool_name.to_string(), details.to_string());
                    }
                    
                    if found_issue {
                        if let Some(issues_array) = tool_value["issues"].as_array() {
                            tracing::debug!("Tool {} has {} issues", tool_name, issues_array.len());
                            for issue_value in issues_array {
                                tracing::debug!("Parsing issue: {:?}", issue_value);
                                if let Some(issue) = self.parse_issue_from_value(issue_value) {
                                    // Set the tool name from the parent object
                                    let mut issue = issue;
                                    issue.tool_name = Some(tool_name.to_string());
                                    issues.push(issue);
                                    tracing::debug!("Successfully parsed issue for tool {}", tool_name);
                                } else {
                                    tracing::warn!("Failed to parse issue for tool {}: {:?}", tool_name, issue_value);
                                }
                            }
                        } else {
                            tracing::warn!("Tool {} has found_issue=true but no issues array", tool_name);
                        }
                    }
                } else {
                    tracing::warn!("Tool {} missing found_issue field", tool_name);
                }
            } else {
                // Fallback to old format for prompts and resources
                if let Some(issue) = self.parse_issue_from_value(&tool_value) {
                    issues.push(issue);
                }
            }
        }
        
        tracing::debug!("Total issues parsed: {}", issues.len());
        Ok((issues, analysis_details))
    }

    /// Parse a single issue from JSON value
    fn parse_issue_from_value(&self, issue_value: &Value) -> Option<SecurityIssue> {
        tracing::debug!("Parsing issue value: {:?}", issue_value);
        
        // For individual issues in the issues array, we don't expect name fields
        // These are set by the parent object during parsing
        let issue_type_str = issue_value["issue_type"].as_str()?;
        let severity_str = issue_value["severity"].as_str()?;
        let message = issue_value["message"].as_str()?;
        let details = issue_value["details"].as_str()?;

        tracing::debug!("Issue fields: type={}, severity={}, message={}, details={}", 
                       issue_type_str, severity_str, message, details);

        let issue_type = self.parse_issue_type(issue_type_str)?;
        let severity = self.parse_severity(severity_str);
        
        let mut issue = SecurityIssue::new(issue_type, message.to_string());
        issue.severity = severity.to_string();
        issue.details = Some(details.to_string());
        
        // Note: tool_name, prompt_name, or resource_uri will be set by the parent parser
        // based on the context (which type of scan we're doing)
        
        tracing::debug!("Successfully created issue: {:?}", issue);
        Some(issue)
    }

    /// Parse severity string to SecuritySeverity enum
    fn parse_severity(&self, severity_str: &str) -> SecuritySeverity {
        match severity_str.to_uppercase().as_str() {
            "CRITICAL" => SecuritySeverity::Critical,
            "HIGH" => SecuritySeverity::High,
            "MEDIUM" => SecuritySeverity::Medium,
            "LOW" => SecuritySeverity::Low,
            _ => SecuritySeverity::Medium,
        }
    }

    /// Parse issue type string to SecurityIssueType enum
    fn parse_issue_type(&self, issue_type_str: &str) -> Option<SecurityIssueType> {
        match issue_type_str {
            "ToolPoisoning" => Some(SecurityIssueType::ToolPoisoning),
            "SQLInjection" => Some(SecurityIssueType::SQLInjection),
            "CommandInjection" => Some(SecurityIssueType::CommandInjection),
            "PathTraversal" => Some(SecurityIssueType::PathTraversal),
            "AuthBypass" => Some(SecurityIssueType::AuthBypass),
            "PromptInjection" => Some(SecurityIssueType::PromptInjection),
            "Jailbreak" => Some(SecurityIssueType::Jailbreak),
            "PIILeakage" => Some(SecurityIssueType::PIILeakage),
            "SecretsLeakage" => Some(SecurityIssueType::SecretsLeakage),
            _ => None,
        }
    }

    /// Extract JSON array from LLM response
    fn extract_json_array(&self, response: &str) -> Result<Vec<Value>> {
        tracing::debug!("Raw LLM response: {}", response);
        
        // Try to find JSON array in the response
        if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                let json_str = &response[start..=end];
                tracing::debug!("Extracted JSON string: {}", json_str);
                if let Ok(array) = serde_json::from_str::<Vec<Value>>(json_str) {
                    tracing::debug!("Successfully parsed JSON array with {} elements", array.len());
                    return Ok(array);
                } else {
                    tracing::warn!("Failed to parse extracted JSON string: {}", json_str);
                }
            }
        }
        
        // If no array found, try to parse the entire response as JSON
        if let Ok(array) = serde_json::from_str::<Vec<Value>>(response) {
            tracing::debug!("Successfully parsed entire response as JSON array with {} elements", array.len());
            return Ok(array);
        } else {
            tracing::warn!("Failed to parse entire response as JSON: {}", response);
        }
        
        // If we still can't parse it, try to find any JSON-like structure
        if response.contains("[]") {
            tracing::info!("Response contains empty array, returning empty result");
            return Ok(Vec::new());
        }
        
        Err(anyhow!("Could not extract JSON array from LLM response. Response was: {}", response))
    }
}

#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for SecuritySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecuritySeverity::Low => write!(f, "LOW"),
            SecuritySeverity::Medium => write!(f, "MEDIUM"),
            SecuritySeverity::High => write!(f, "HIGH"),
            SecuritySeverity::Critical => write!(f, "CRITICAL"),
        }
    }
} 