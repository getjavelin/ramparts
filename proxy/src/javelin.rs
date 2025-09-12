use crate::cache::{CacheConfig, ValidationCache, ValidationCacheEntry};
use crate::config::ProxyBehavior;
use ramparts_common::{
    anyhow::Result,
    tracing::{debug, info, warn},
};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

/// Javelin Guardrails API client with caching
pub struct JavelinClient {
    api_key: String,
    base_url: String,
    client: Client,
    cache: Arc<ValidationCache>,
    cache_enabled: bool,
}

/// Javelin API request structure
#[derive(serde::Serialize)]
struct GuardrailRequest {
    content: Value,
    metadata: GuardrailMetadata,
}

/// Metadata for guardrail requests
#[derive(serde::Serialize)]
struct GuardrailMetadata {
    request_type: String,
    timestamp: String,
    source: String,
}

/// Javelin API response structure
#[derive(serde::Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct GuardrailResponse {
    allowed: bool,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
}

impl JavelinClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10)) // Reduced timeout for faster failures
            .user_agent(format!("ramparts-proxy/{}", env!("CARGO_PKG_VERSION")))
            .pool_max_idle_per_host(10) // Connection pooling optimization
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Initialize cache with optimized settings
        let cache_config = CacheConfig {
            max_entries: 10_000,
            ttl: Duration::from_secs(300), // 5 minutes
            enable_deduplication: true,
            hash_sensitive_data: true,
        };
        let cache = Arc::new(ValidationCache::new(cache_config));

        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| {
                std::env::var("JAVELIN_API_URL")
                    .unwrap_or_else(|_| "https://api.getjavelin.com".to_string())
            }),
            client,
            cache,
            cache_enabled: true,
        }
    }

    /// Create a new client with custom configuration
    pub fn with_config(api_key: String, base_url: String, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent(format!("ramparts-proxy/{}", env!("CARGO_PKG_VERSION")))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Initialize cache with optimized settings
        let cache_config = CacheConfig {
            max_entries: 10_000,
            ttl: Duration::from_secs(300),
            enable_deduplication: true,
            hash_sensitive_data: true,
        };
        let cache = Arc::new(ValidationCache::new(cache_config));

        Self {
            api_key,
            base_url,
            client,
            cache,
            cache_enabled: true,
        }
    }

    /// Create a client honoring proxy behavior (cache TTL and enable/disable)
    pub fn with_behavior(
        api_key: String,
        base_url: String,
        timeout_secs: u64,
        behavior: &ProxyBehavior,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent(format!("ramparts-proxy/{}", env!("CARGO_PKG_VERSION")))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let cache_ttl = Duration::from_secs(behavior.cache_ttl_seconds);
        let cache = Arc::new(ValidationCache::new(CacheConfig {
            max_entries: 10_000,
            ttl: cache_ttl,
            enable_deduplication: true,
            hash_sensitive_data: true,
        }));

        Self {
            api_key,
            base_url,
            client,
            cache,
            cache_enabled: behavior.cache_validations,
        }
    }

    pub async fn validate_request(&self, request: &Value) -> Result<bool> {
        debug!("Validating request with Javelin Guardrails API");

        if self.cache_enabled {
            // Use cache with deduplication
            let cache_result =
                self.cache
                    .get_or_compute(request, || {
                        let api_key = self.api_key.clone();
                        let base_url = self.base_url.clone();
                        let client = self.client.clone();
                        let request = request.clone();

                        async move {
                            Self::validate_request_uncached(api_key, base_url, client, request)
                                .await
                        }
                    })
                    .await?;
            Ok(cache_result.allowed)
        } else {
            // Bypass cache entirely
            let result =
                Self::validate_request_uncached(
                    self.api_key.clone(),
                    self.base_url.clone(),
                    self.client.clone(),
                    request.clone(),
                )
                .await?;
            Ok(result.allowed)
        }
    }

    /// Internal method for uncached validation
    async fn validate_request_uncached(
        api_key: String,
        base_url: String,
        client: Client,
        request: Value,
    ) -> Result<ValidationCacheEntry> {
        debug!("Making uncached request to Javelin API");

        // Prepare the guardrail request
        let guardrail_request = GuardrailRequest {
            content: request,
            metadata: GuardrailMetadata {
                request_type: "mcp_tool_call".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                source: "ramparts-proxy".to_string(),
            },
        };

        // Prepare headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-Javelin-Apikey",
            HeaderValue::from_str(&api_key)
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid API key format: {}", e))?,
        );

        // Use the correct Javelin Guard API endpoint
        let endpoint = format!("{}/v1/internal/guard/predict", base_url);
        debug!("Making request to Javelin Guard API: {}", endpoint);

        // Format request for Javelin Guard API
        let request_text = Self::format_request_for_guard(&guardrail_request.content);
        let request_body = serde_json::json!({
            "text": request_text
        });

        // Avoid logging full request content; log a safe preview only
        let preview = crate::logging::truncate_for_log(&request_text);
        debug!("Sending request to Javelin Guard (preview): {}", preview);

        let response = client
            .post(&endpoint)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                ramparts_common::anyhow::anyhow!(
                    "Failed to send request to Javelin Guard API: {}",
                    e
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!(
                "Javelin Guard API returned error {}: {}",
                status, error_text
            );

            // Fail-open: allow requests if the API is unavailable
            return Ok(ValidationCacheEntry {
                allowed: true,
                reason: Some(format!("Guard API error: {}", status)),
                confidence: None,
                timestamp: Instant::now(),
            });
        }

        Self::parse_guard_response(response).await
    }

    /// Format MCP request content for Javelin Guard API (optimized version)
    pub fn format_request_for_guard(content: &Value) -> String {
        // Fast path: extract tool name and arguments
        let (tool_name, args) = match Self::extract_tool_info(content) {
            Some((name, args)) => (name, args),
            None => {
                return format!(
                    "perform action: {}",
                    serde_json::to_string(content).unwrap_or_default()
                )
            }
        };

        // Use optimized formatter based on tool type
        Self::format_tool_request(tool_name, args)
    }

    /// Extract tool name and arguments efficiently
    pub fn extract_tool_info(content: &Value) -> Option<(&str, &Value)> {
        let params = content.get("params")?;
        let name = params.get("name")?.as_str()?;
        let args = params.get("arguments")?;
        Some((name, args))
    }

    /// Format tool request using lookup table and templates
    fn format_tool_request(tool_name: &str, args: &Value) -> String {
        // Optimized lookup table for common tools
        match tool_name {
            // File operations
            "read_file" | "file_read" => Self::format_with_path("read file", args),
            "write_file" | "file_write" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    let content_preview = args
                        .get("content")
                        .and_then(|c| c.as_str())
                        .map(|s| if s.len() > 50 { &s[..50] } else { s })
                        .unwrap_or("data");
                    format!("write file {} with content: {}", path, content_preview)
                } else {
                    format!("write file with content")
                }
            }
            "delete_file" | "file_delete" => Self::format_with_path("delete file", args),

            // Command execution
            "execute_command" | "run_command" | "shell_exec" => {
                Self::format_with_field("execute command", "command", args)
            }

            // Database operations
            "database_query" | "sql_query" => {
                Self::format_with_field("run database query", "query", args)
            }

            // Network operations
            "network_request" | "http_request" => {
                Self::format_with_field("make network request to", "url", args)
            }

            // Generic tool
            _ => {
                format!(
                    "use tool {} with arguments: {}",
                    tool_name,
                    serde_json::to_string(args).unwrap_or_default()
                )
            }
        }
    }

    /// Helper: format tool with path argument
    pub fn format_with_path(action: &str, args: &Value) -> String {
        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
            format!("{} {}", action, path)
        } else {
            action.to_string()
        }
    }

    /// Helper: format tool with specific field
    pub fn format_with_field(action: &str, field: &str, args: &Value) -> String {
        if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
            format!("{}: {}", action, value)
        } else {
            action.to_string()
        }
    }

    /// Parse response from Javelin Guard API
    async fn parse_guard_response(response: reqwest::Response) -> Result<ValidationCacheEntry> {
        let response_text = response.text().await.map_err(|e| {
            ramparts_common::anyhow::anyhow!("Failed to read Guard API response: {}", e)
        })?;

        // Truncate response to avoid leaking large/sensitive content
        let resp_preview = crate::logging::truncate_for_log(&response_text);
        debug!("Javelin Guard API response (preview): {}", resp_preview);

        // Parse Guard API response - it might return various formats
        // Try to parse as JSON first
        if let Ok(json_response) = serde_json::from_str::<Value>(&response_text) {
            // Check for different possible response structures

            // Format 1: Javelin Guard format with categories and scores
            if let Some(categories) = json_response.get("categories") {
                let category_scores = json_response.get("category_scores");

                // Check if any category indicates a threat
                let mut is_threat = false;
                let mut threat_reasons = Vec::new();
                let mut max_score: f64 = 0.0;

                if let Some(categories_obj) = categories.as_object() {
                    for (category, value) in categories_obj {
                        let is_category_threat = match value.as_str() {
                            Some("true") => true,
                            Some("false") => false,
                            _ => value.as_bool().unwrap_or(false),
                        };

                        if is_category_threat {
                            is_threat = true;
                            threat_reasons.push(category.clone());
                        }

                        // Get the score for this category
                        if let Some(scores) = category_scores.and_then(|s| s.as_object()) {
                            if let Some(score) = scores.get(category).and_then(|s| s.as_f64()) {
                                max_score = max_score.max(score);
                            }
                        }
                    }
                }

                let allowed = !is_threat;
                let reason = if threat_reasons.is_empty() {
                    Some("No threats detected".to_string())
                } else {
                    Some(format!("Threats detected: {}", threat_reasons.join(", ")))
                };

                if allowed {
                    info!("Request approved by Javelin Guard (no threats detected)");
                } else {
                    warn!(
                        "Request blocked by Javelin Guard: {}",
                        reason.as_ref().unwrap_or(&"Threats detected".to_string())
                    );
                }

                return Ok(ValidationCacheEntry {
                    allowed,
                    reason,
                    confidence: Some(max_score),
                    timestamp: Instant::now(),
                });
            }

            // Format 2: {"safe": true/false, "confidence": 0.9, "reason": "..."}
            if let Some(safe) = json_response.get("safe").and_then(|s| s.as_bool()) {
                let confidence = json_response.get("confidence").and_then(|c| c.as_f64());
                let reason = json_response
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .map(|s| s.to_string());

                if safe {
                    info!("Request approved by Javelin Guard (safe: true)");
                } else {
                    warn!(
                        "Request blocked by Javelin Guard (safe: false): {}",
                        reason.as_ref().unwrap_or(&"No reason provided".to_string())
                    );
                }

                return Ok(ValidationCacheEntry {
                    allowed: safe,
                    reason,
                    confidence,
                    timestamp: Instant::now(),
                });
            }

            // Format 2: {"allowed": true/false, ...}
            if let Some(allowed) = json_response.get("allowed").and_then(|a| a.as_bool()) {
                let confidence = json_response.get("confidence").and_then(|c| c.as_f64());
                let reason = json_response
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .map(|s| s.to_string());

                return Ok(ValidationCacheEntry {
                    allowed,
                    reason,
                    confidence,
                    timestamp: Instant::now(),
                });
            }

            // Format 3: {"result": "safe"/"unsafe", ...}
            if let Some(result) = json_response.get("result").and_then(|r| r.as_str()) {
                let allowed = result.to_lowercase().contains("safe")
                    || result.to_lowercase().contains("allow");
                let confidence = json_response.get("confidence").and_then(|c| c.as_f64());

                return Ok(ValidationCacheEntry {
                    allowed,
                    reason: Some(format!("Guard result: {}", result)),
                    confidence,
                    timestamp: Instant::now(),
                });
            }
        }

        // Try to parse as simple boolean
        if let Ok(simple_bool) = serde_json::from_str::<bool>(&response_text) {
            return Ok(ValidationCacheEntry {
                allowed: simple_bool,
                reason: Some("Boolean response from Guard API".to_string()),
                confidence: Some(if simple_bool { 0.9 } else { 0.1 }),
                timestamp: Instant::now(),
            });
        }

        // Parse as text response
        let response_lower = response_text.to_lowercase();
        let allowed = response_lower.contains("safe")
            || response_lower.contains("allow")
            || response_lower.contains("ok")
            || response_lower.contains("approved");

        let blocked = response_lower.contains("unsafe")
            || response_lower.contains("block")
            || response_lower.contains("deny")
            || response_lower.contains("reject");

        if allowed || blocked {
            if allowed {
                info!("Request approved by Javelin Guard (text response)");
            } else {
                warn!(
                    "Request blocked by Javelin Guard (text response): {}",
                    response_text
                );
            }

            return Ok(ValidationCacheEntry {
                allowed,
                reason: Some(format!("Guard text response: {}", response_text)),
                confidence: Some(0.7),
                timestamp: Instant::now(),
            });
        }

        // Default: treat unknown response as allowed (fail-open)
        warn!(
            "Unknown response format from Javelin Guard API: {}",
            response_text
        );
        Ok(ValidationCacheEntry {
            allowed: true,
            reason: Some(format!("Unknown Guard response: {}", response_text)),
            confidence: Some(0.5),
            timestamp: Instant::now(),
        })
    }

    /// Health check for the Javelin API
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/v1/health", self.base_url);
        debug!("Checking Javelin API health: {}", url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                ramparts_common::anyhow::anyhow!("Failed to check Javelin API health: {}", e)
            })?;

        Ok(response.status().is_success())
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats().await
    }

    /// Clear the validation cache
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    /// Cleanup expired cache entries
    pub async fn cleanup_cache(&self) {
        self.cache.cleanup().await;
    }
}
