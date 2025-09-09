use ramparts_common::{anyhow::Result, tracing::{debug, info, warn, error}};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::{JavelinClient, ProxyConfig};

/// Unified validation service that handles all request/response validation
pub struct ValidationService {
    javelin_client: Arc<JavelinClient>,
    config: ProxyConfig,
}

/// Validation result with detailed information
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub request_id: String,
    pub timestamp: String,
}

/// Validation error with proper JSON-RPC formatting
#[derive(Debug)]
pub struct ValidationError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl ValidationService {
    pub fn new(javelin_client: Arc<JavelinClient>, config: ProxyConfig) -> Self {
        Self {
            javelin_client,
            config,
        }
    }

    /// Validate a request with consistent error handling
    pub async fn validate_request(&self, request: &Value) -> Result<ValidationResult> {
        debug!("Validating request with unified validation service");

        let request_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Extract method for method-specific validation
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("unknown");
        debug!("Validating method: {}", method);

        // Apply method-specific validation rules
        if let Some(method_result) = self.validate_method_specific(request, method, &request_id, &timestamp).await? {
            return Ok(method_result);
        }

        // Check if we're in test mode (no Javelin API key)
        if self.config.javelin.api_key == "test-mode" {
            debug!("Test mode: allowing all requests without Javelin validation");
            return Ok(ValidationResult {
                allowed: true,
                reason: Some(format!("Test mode - {} validation bypassed", method)),
                confidence: Some(1.0),
                request_id,
                timestamp,
            });
        }

        match self.javelin_client.validate_request(request).await {
            Ok(is_valid) => {
                let result = ValidationResult {
                    allowed: is_valid,
                    reason: if is_valid {
                        Some("Request approved by Javelin Guardrails".to_string())
                    } else {
                        Some("Request blocked by Javelin Guardrails".to_string())
                    },
                    confidence: Some(if is_valid { 0.9 } else { 0.1 }),
                    request_id,
                    timestamp,
                };

                if is_valid {
                    info!("Request {} approved by validation service", result.request_id);
                } else {
                    warn!("Request {} blocked by validation service", result.request_id);
                }

                Ok(result)
            }
            Err(e) => {
                error!("Validation error for request {}: {}", request_id, e);

                // Apply fail-open/fail-closed policy
                let allowed = self.config.javelin.fail_open;
                let reason = if allowed {
                    format!("Validation service unavailable, failing open: {}", e)
                } else {
                    format!("Validation service unavailable, failing closed: {}", e)
                };

                if allowed {
                    warn!("Request {} allowed due to fail-open policy", request_id);
                } else {
                    error!("Request {} blocked due to fail-closed policy", request_id);
                }

                Ok(ValidationResult {
                    allowed,
                    reason: Some(reason),
                    confidence: Some(0.0),
                    request_id,
                    timestamp,
                })
            }
        }
    }

    /// Validate a response (optional, for response filtering)
    pub async fn validate_response(&self, response: &Value) -> Result<ValidationResult> {
        debug!("Validating response with unified validation service");

        // For responses, we might want different validation logic
        // For now, reuse the same validation but with different logging
        let mut result = self.validate_request(response).await?;
        
        // Update the reason to indicate this was response validation
        if let Some(ref mut reason) = result.reason {
            *reason = reason.replace("Request", "Response");
        }

        Ok(result)
    }

    /// Create a JSON-RPC error response for blocked requests
    pub fn create_blocked_response(&self, original_request: &Value, validation_result: &ValidationResult) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": original_request.get("id"),
            "error": {
                "code": -32600,
                "message": "Request blocked by Javelin Guardrails",
                "data": {
                    "reason": validation_result.reason,
                    "confidence": validation_result.confidence,
                    "request_id": validation_result.request_id,
                    "timestamp": validation_result.timestamp,
                    "blocked_by": "ramparts-proxy"
                }
            }
        })
    }

    /// Create a JSON-RPC error response for validation failures
    pub fn create_error_response(&self, original_request: &Value, error_message: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": original_request.get("id"),
            "error": {
                "code": -32603,
                "message": "Internal validation error",
                "data": {
                    "error": error_message,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "service": "ramparts-proxy"
                }
            }
        })
    }

    /// Validate and handle request with automatic error response generation
    pub async fn validate_and_handle(&self, request: &Value) -> Result<ValidationResult, Value> {
        match self.validate_request(request).await {
            Ok(result) => {
                if result.allowed {
                    Ok(result)
                } else {
                    Err(self.create_blocked_response(request, &result))
                }
            }
            Err(e) => {
                error!("Validation service error: {}", e);
                Err(self.create_error_response(request, &e.to_string()))
            }
        }
    }

    /// Get validation service health status
    pub async fn health_check(&self) -> Result<bool> {
        self.javelin_client.health_check().await
    }

    /// Get cache statistics from the underlying client
    pub async fn cache_stats(&self) -> crate::cache::CacheStats {
        self.javelin_client.cache_stats().await
    }

    /// Clear validation cache
    pub async fn clear_cache(&self) {
        self.javelin_client.clear_cache().await;
    }

    /// Apply method-specific validation rules
    async fn validate_method_specific(
        &self,
        request: &Value,
        method: &str,
        request_id: &str,
        timestamp: &str,
    ) -> Result<Option<ValidationResult>> {
        match method {
            "tools/call" => {
                debug!("Applying tools/call specific validation rules");
                // Check for dangerous tool calls
                if let Some(params) = request.get("params") {
                    if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                        // Block dangerous tools
                        if self.is_dangerous_tool(name) {
                            warn!("Blocked dangerous tool call: {}", name);
                            return Ok(Some(ValidationResult {
                                allowed: false,
                                reason: Some(format!("Dangerous tool '{}' blocked by security policy", name)),
                                confidence: Some(0.9),
                                request_id: request_id.to_string(),
                                timestamp: timestamp.to_string(),
                            }));
                        }

                        // Check tool arguments for injection patterns
                        if let Some(args) = params.get("arguments") {
                            if self.contains_injection_patterns(args) {
                                warn!("Blocked tool call with injection patterns: {}", name);
                                return Ok(Some(ValidationResult {
                                    allowed: false,
                                    reason: Some(format!("Tool '{}' arguments contain injection patterns", name)),
                                    confidence: Some(0.8),
                                    request_id: request_id.to_string(),
                                    timestamp: timestamp.to_string(),
                                }));
                            }
                        }
                    }
                }
            }
            "resources/read" => {
                debug!("Applying resources/read specific validation rules");
                // Check for path traversal attempts
                if let Some(params) = request.get("params") {
                    if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                        if self.contains_path_traversal(uri) {
                            warn!("Blocked resource read with path traversal: {}", uri);
                            return Ok(Some(ValidationResult {
                                allowed: false,
                                reason: Some(format!("Resource URI '{}' contains path traversal patterns", uri)),
                                confidence: Some(0.9),
                                request_id: request_id.to_string(),
                                timestamp: timestamp.to_string(),
                            }));
                        }
                    }
                }
            }
            "prompts/get" => {
                debug!("Applying prompts/get specific validation rules");
                // Check for prompt injection attempts
                if let Some(params) = request.get("params") {
                    if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                        if self.contains_prompt_injection(name) {
                            warn!("Blocked prompt with injection patterns: {}", name);
                            return Ok(Some(ValidationResult {
                                allowed: false,
                                reason: Some(format!("Prompt '{}' contains injection patterns", name)),
                                confidence: Some(0.8),
                                request_id: request_id.to_string(),
                                timestamp: timestamp.to_string(),
                            }));
                        }
                    }
                }
            }
            _ => {
                debug!("No specific validation rules for method: {}", method);
            }
        }

        Ok(None) // No method-specific blocking, continue with general validation
    }

    /// Check if a tool name is considered dangerous
    fn is_dangerous_tool(&self, tool_name: &str) -> bool {
        let dangerous_tools = [
            "exec", "shell", "bash", "cmd", "powershell", "eval", "system",
            "subprocess", "popen", "spawn", "fork", "kill", "rm", "del",
            "format", "fdisk", "mkfs", "dd", "nc", "netcat", "telnet",
            "curl_exec", "wget_exec", "download_exec"
        ];

        dangerous_tools.iter().any(|&dangerous| {
            tool_name.to_lowercase().contains(dangerous)
        })
    }

    /// Check for injection patterns in tool arguments
    fn contains_injection_patterns(&self, args: &Value) -> bool {
        let args_str = args.to_string().to_lowercase();
        let injection_patterns = [
            "; ", "| ", "& ", "$(", "`", "&&", "||", "../", "..\\",
            "rm -", "del ", "format ", "fdisk", "mkfs", "dd if=",
            "curl ", "wget ", "nc ", "netcat", "telnet", "ssh ",
            "base64", "eval", "exec", "system", "popen"
        ];

        injection_patterns.iter().any(|&pattern| args_str.contains(pattern))
    }

    /// Check for path traversal patterns
    fn contains_path_traversal(&self, uri: &str) -> bool {
        let uri_lower = uri.to_lowercase();
        uri_lower.contains("../") || uri_lower.contains("..\\") ||
        uri_lower.contains("%2e%2e") || uri_lower.contains("....") ||
        uri_lower.contains("/etc/") || uri_lower.contains("\\windows\\") ||
        uri_lower.contains("/proc/") || uri_lower.contains("/sys/")
    }

    /// Check for prompt injection patterns
    fn contains_prompt_injection(&self, prompt_name: &str) -> bool {
        let prompt_lower = prompt_name.to_lowercase();
        let injection_patterns = [
            "ignore", "forget", "disregard", "override", "bypass", "jailbreak",
            "system:", "assistant:", "user:", "human:", "ai:", "chatgpt:",
            "\\n\\n", "---", "###", "```", "exec", "eval", "script"
        ];

        injection_patterns.iter().any(|&pattern| prompt_lower.contains(pattern))
    }
}

/// Helper function to extract request ID from JSON-RPC request
pub fn extract_request_id(request: &Value) -> Option<Value> {
    request.get("id").cloned()
}

/// Helper function to check if a request is a JSON-RPC request
pub fn is_jsonrpc_request(request: &Value) -> bool {
    request.get("jsonrpc").and_then(|v| v.as_str()) == Some("2.0")
}

/// Helper function to create a success response
pub fn create_success_response(request_id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": result
    })
}
