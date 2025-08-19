use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{timeout, Instant};
use serde_json::Value;
use std::collections::HashMap;
use crate::cache::ValidationCacheEntry;
use ramparts_common::anyhow::Result;

/// Request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Critical = 0,  // System operations, health checks
    High = 1,      // User-facing operations
    Normal = 2,    // Regular tool calls
    Low = 3,       // Background operations
}

/// Validation mode determines how strict we are
#[derive(Debug, Clone)]
pub enum ValidationMode {
    /// Block request until validation completes
    Blocking,
    /// Allow request, validate in background
    NonBlocking,
    /// Allow if request matches safe patterns, validate others
    SmartBlocking,
    /// Skip validation entirely (emergency mode)
    Bypass,
}

/// Request classification for smart routing
#[derive(Debug, Clone)]
pub struct RequestClassification {
    pub priority: RequestPriority,
    pub validation_mode: ValidationMode,
    pub estimated_risk: f32, // 0.0 = safe, 1.0 = dangerous
    pub cache_ttl: Duration,
}

/// Async validation request
#[derive(Debug)]
pub struct AsyncValidationRequest {
    pub id: String,
    pub request: Value,
    pub classification: RequestClassification,
    pub response_tx: oneshot::Sender<Result<ValidationCacheEntry>>,
    pub submitted_at: Instant,
}

/// Fast-path patterns for immediate approval
pub struct SafePatternMatcher {
    safe_tools: Arc<RwLock<HashMap<String, f32>>>, // tool_name -> risk_score
    safe_patterns: Arc<RwLock<Vec<regex::Regex>>>,
}

impl SafePatternMatcher {
    pub fn new() -> Self {
        let mut safe_tools = HashMap::new();
        
        // Pre-populate with known safe tools
        safe_tools.insert("read_file".to_string(), 0.1);
        safe_tools.insert("list_files".to_string(), 0.1);
        safe_tools.insert("get_weather".to_string(), 0.0);
        safe_tools.insert("search_web".to_string(), 0.2);
        safe_tools.insert("calculate".to_string(), 0.0);
        
        // Dangerous tools
        safe_tools.insert("execute_command".to_string(), 0.9);
        safe_tools.insert("delete_file".to_string(), 0.8);
        safe_tools.insert("write_file".to_string(), 0.6);
        safe_tools.insert("database_query".to_string(), 0.7);

        Self {
            safe_tools: Arc::new(RwLock::new(safe_tools)),
            safe_patterns: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn classify_request(&self, request: &Value) -> RequestClassification {
        let tool_name = self.extract_tool_name(request);
        let risk_score = self.calculate_risk_score(request, &tool_name).await;
        
        let priority = match risk_score {
            r if r < 0.2 => RequestPriority::Low,
            r if r < 0.5 => RequestPriority::Normal,
            r if r < 0.8 => RequestPriority::High,
            _ => RequestPriority::Critical,
        };

        let validation_mode = match risk_score {
            r if r < 0.1 => ValidationMode::NonBlocking,
            r if r < 0.3 => ValidationMode::SmartBlocking,
            _ => ValidationMode::Blocking,
        };

        let cache_ttl = match risk_score {
            r if r < 0.2 => Duration::from_secs(3600), // 1 hour for safe operations
            r if r < 0.5 => Duration::from_secs(300),  // 5 minutes for normal
            _ => Duration::from_secs(60),              // 1 minute for risky
        };

        RequestClassification {
            priority,
            validation_mode,
            estimated_risk: risk_score,
            cache_ttl,
        }
    }

    fn extract_tool_name(&self, request: &Value) -> Option<String> {
        request
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
    }

    async fn calculate_risk_score(&self, request: &Value, tool_name: &Option<String>) -> f32 {
        let mut risk_score = 0.5; // Default medium risk

        // Check tool-based risk
        if let Some(tool) = tool_name {
            let safe_tools = self.safe_tools.read().await;
            if let Some(&tool_risk) = safe_tools.get(tool) {
                risk_score = tool_risk;
            }
        }

        // Check argument-based risk
        if let Some(args) = request.get("params").and_then(|p| p.get("arguments")) {
            risk_score += self.analyze_arguments(args).await;
        }

        risk_score.min(1.0)
    }

    async fn analyze_arguments(&self, args: &Value) -> f32 {
        let mut additional_risk = 0.0;

        // Check for dangerous paths
        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
            if path.starts_with("/etc/") || path.starts_with("/sys/") || path.contains("..") {
                additional_risk += 0.3;
            }
        }

        // Check for dangerous commands
        if let Some(command) = args.get("command").and_then(|c| c.as_str()) {
            if command.contains("rm -rf") || command.contains("sudo") || command.contains("chmod") {
                additional_risk += 0.4;
            }
        }

        // Check for SQL injection patterns
        if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
            if query.contains("'") && (query.contains("OR") || query.contains("UNION")) {
                additional_risk += 0.5;
            }
        }

        additional_risk
    }

    pub async fn update_safe_tools(&self, updates: HashMap<String, f32>) {
        let mut safe_tools = self.safe_tools.write().await;
        safe_tools.extend(updates);
    }
}

/// High-performance async validation engine
pub struct AsyncValidationEngine {
    pattern_matcher: SafePatternMatcher,
    validation_tx: mpsc::UnboundedSender<AsyncValidationRequest>,
    _worker_handles: Vec<tokio::task::JoinHandle<()>>,
}

impl AsyncValidationEngine {
    pub fn new<F, Fut>(
        worker_count: usize,
        validator_fn: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<ValidationCacheEntry>> + Send,
    {
        let (validation_tx, validation_rx) = mpsc::unbounded_channel();
        let validation_rx = Arc::new(tokio::sync::Mutex::new(validation_rx));
        
        let mut worker_handles = Vec::new();
        
        // Spawn multiple worker tasks for parallel processing
        for worker_id in 0..worker_count {
            let validation_rx = validation_rx.clone();
            let validator_fn = validator_fn.clone();
            
            let handle = tokio::spawn(async move {
                tracing::info!("Validation worker {} started", worker_id);
                
                loop {
                    let request = {
                        let mut rx = validation_rx.lock().await;
                        rx.recv().await
                    };
                    
                    match request {
                        Some(req) => {
                            let start_time = Instant::now();
                            let result = validator_fn(req.request.clone()).await;
                            let duration = start_time.elapsed();
                            
                            tracing::debug!(
                                "Worker {} processed request {} in {:?}",
                                worker_id, req.id, duration
                            );
                            
                            let _ = req.response_tx.send(result);
                        }
                        None => {
                            tracing::info!("Validation worker {} shutting down", worker_id);
                            break;
                        }
                    }
                }
            });
            
            worker_handles.push(handle);
        }

        Self {
            pattern_matcher: SafePatternMatcher::new(),
            validation_tx,
            _worker_handles: worker_handles,
        }
    }

    /// Validate request with smart routing
    pub async fn validate_request(&self, request: &Value) -> Result<ValidationCacheEntry> {
        let classification = self.pattern_matcher.classify_request(request).await;
        
        match classification.validation_mode {
            ValidationMode::Bypass => {
                // Emergency mode - skip validation
                return Ok(ValidationCacheEntry {
                    allowed: true,
                    reason: Some("Validation bypassed".to_string()),
                    confidence: Some(0.0),
                    timestamp: Instant::now(),
                });
            }
            
            ValidationMode::NonBlocking => {
                // Start validation but don't wait
                self.start_background_validation(request.clone()).await;
                
                return Ok(ValidationCacheEntry {
                    allowed: true,
                    reason: Some("Pre-approved safe operation".to_string()),
                    confidence: Some(1.0 - classification.estimated_risk),
                    timestamp: Instant::now(),
                });
            }
            
            ValidationMode::SmartBlocking => {
                // Fast timeout for medium-risk operations
                let validation_timeout = Duration::from_millis(100);
                
                match timeout(validation_timeout, self.validate_with_priority(request, classification.priority)).await {
                    Ok(result) => result,
                    Err(_) => {
                        // Timeout - allow but log
                        tracing::warn!("Validation timeout for medium-risk request, allowing");
                        Ok(ValidationCacheEntry {
                            allowed: true,
                            reason: Some("Validation timeout - allowed".to_string()),
                            confidence: Some(0.5),
                            timestamp: Instant::now(),
                        })
                    }
                }
            }
            
            ValidationMode::Blocking => {
                // Full validation required
                self.validate_with_priority(request, classification.priority).await
            }
        }
    }

    async fn start_background_validation(&self, request: Value) {
        let (tx, _rx) = oneshot::channel(); // We don't wait for the result
        
        let validation_request = AsyncValidationRequest {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            classification: RequestClassification {
                priority: RequestPriority::Low,
                validation_mode: ValidationMode::NonBlocking,
                estimated_risk: 0.1,
                cache_ttl: Duration::from_secs(3600),
            },
            response_tx: tx,
            submitted_at: Instant::now(),
        };

        let _ = self.validation_tx.send(validation_request);
    }

    async fn validate_with_priority(&self, request: &Value, priority: RequestPriority) -> Result<ValidationCacheEntry> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let validation_request = AsyncValidationRequest {
            id: uuid::Uuid::new_v4().to_string(),
            request: request.clone(),
            classification: RequestClassification {
                priority,
                validation_mode: ValidationMode::Blocking,
                estimated_risk: 0.5,
                cache_ttl: Duration::from_secs(300),
            },
            response_tx,
            submitted_at: Instant::now(),
        };

        self.validation_tx.send(validation_request)
            .map_err(|_| ramparts_common::anyhow::anyhow!("Failed to submit validation request"))?;

        response_rx.await
            .map_err(|_| ramparts_common::anyhow::anyhow!("Validation request cancelled"))?
    }

    /// Update safe patterns based on learning
    pub async fn update_patterns(&self, tool_risks: HashMap<String, f32>) {
        self.pattern_matcher.update_safe_tools(tool_risks).await;
    }

    /// Get engine statistics
    pub async fn stats(&self) -> AsyncValidationStats {
        AsyncValidationStats {
            worker_count: self._worker_handles.len(),
            queue_size: 0, // Would need to track this
            avg_processing_time_ms: 0, // Would need to track this
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AsyncValidationStats {
    pub worker_count: usize,
    pub queue_size: usize,
    pub avg_processing_time_ms: u64,
}
