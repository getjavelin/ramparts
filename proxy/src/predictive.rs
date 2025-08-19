use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use serde_json::Value;
use crate::cache::ValidationCacheEntry;

/// Pattern learning system for predictive caching
pub struct PatternLearner {
    request_history: Arc<RwLock<VecDeque<RequestEvent>>>,
    pattern_cache: Arc<RwLock<HashMap<String, PredictedPattern>>>,
    max_history_size: usize,
}

#[derive(Debug, Clone)]
struct RequestEvent {
    timestamp: Instant,
    tool_name: String,
    arguments_hash: u64,
    user_session: Option<String>,
    response_time_ms: u64,
    cache_hit: bool,
}

#[derive(Debug, Clone)]
struct PredictedPattern {
    sequence: Vec<String>,
    probability: f32,
    avg_interval: Duration,
    last_seen: Instant,
    hit_count: u32,
}

impl PatternLearner {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            request_history: Arc::new(RwLock::new(VecDeque::new())),
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            max_history_size,
        }
    }

    /// Record a request event for pattern learning
    pub async fn record_request(
        &self,
        tool_name: String,
        arguments: &Value,
        user_session: Option<String>,
        response_time_ms: u64,
        cache_hit: bool,
    ) {
        let event = RequestEvent {
            timestamp: Instant::now(),
            tool_name,
            arguments_hash: self.hash_arguments(arguments),
            user_session,
            response_time_ms,
            cache_hit,
        };

        let mut history = self.request_history.write().await;
        history.push_back(event);

        // Maintain history size
        while history.len() > self.max_history_size {
            history.pop_front();
        }

        // Trigger pattern analysis periodically
        if history.len() % 100 == 0 {
            drop(history);
            self.analyze_patterns().await;
        }
    }

    /// Predict next likely requests based on current request
    pub async fn predict_next_requests(&self, current_tool: &str, user_session: Option<&str>) -> Vec<PredictionResult> {
        let patterns = self.pattern_cache.read().await;
        let mut predictions = Vec::new();

        for (pattern_key, pattern) in patterns.iter() {
            if let Some(index) = pattern.sequence.iter().position(|tool| tool == current_tool) {
                if index + 1 < pattern.sequence.len() {
                    let next_tool = &pattern.sequence[index + 1];
                    predictions.push(PredictionResult {
                        tool_name: next_tool.clone(),
                        probability: pattern.probability,
                        estimated_time_until: pattern.avg_interval,
                        pattern_key: pattern_key.clone(),
                    });
                }
            }
        }

        // Sort by probability
        predictions.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap());
        predictions.truncate(5); // Top 5 predictions

        predictions
    }

    async fn analyze_patterns(&self) {
        let history = self.request_history.read().await;
        let mut new_patterns = HashMap::new();

        // Analyze sequences of 2-5 requests
        for window_size in 2..=5 {
            for window in history.iter().collect::<Vec<_>>().windows(window_size) {
                let sequence: Vec<String> = window.iter().map(|e| e.tool_name.clone()).collect();
                let pattern_key = sequence.join("->");

                // Calculate timing intervals
                let intervals: Vec<Duration> = window.windows(2)
                    .map(|pair| pair[1].timestamp.duration_since(pair[0].timestamp))
                    .collect();

                let avg_interval = if intervals.is_empty() {
                    Duration::from_secs(1)
                } else {
                    Duration::from_nanos(
                        intervals.iter().map(|d| d.as_nanos()).sum::<u128>() as u64 / intervals.len() as u64
                    )
                };

                // Update or create pattern
                let pattern = new_patterns.entry(pattern_key.clone()).or_insert_with(|| PredictedPattern {
                    sequence: sequence.clone(),
                    probability: 0.0,
                    avg_interval,
                    last_seen: window.last().unwrap().timestamp,
                    hit_count: 0,
                });

                pattern.hit_count += 1;
                pattern.last_seen = window.last().unwrap().timestamp;
            }
        }

        // Calculate probabilities based on frequency
        let total_patterns = new_patterns.len() as f32;
        for pattern in new_patterns.values_mut() {
            pattern.probability = pattern.hit_count as f32 / total_patterns;
        }

        // Update pattern cache
        let mut pattern_cache = self.pattern_cache.write().await;
        *pattern_cache = new_patterns;
    }

    fn hash_arguments(&self, arguments: &Value) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        arguments.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone)]
pub struct PredictionResult {
    pub tool_name: String,
    pub probability: f32,
    pub estimated_time_until: Duration,
    pub pattern_key: String,
}

/// Predictive cache pre-warmer
pub struct CachePrewarmer {
    pattern_learner: Arc<PatternLearner>,
    prewarming_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl CachePrewarmer {
    pub fn new(pattern_learner: Arc<PatternLearner>) -> Self {
        Self {
            pattern_learner,
            prewarming_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start predictive pre-warming based on current request
    pub async fn prewarm_cache<F, Fut>(
        &self,
        current_tool: &str,
        user_session: Option<&str>,
        validator_fn: F,
    ) where
        F: Fn(Value) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<ValidationCacheEntry, ramparts_common::anyhow::Error>> + Send,
    {
        let predictions = self.pattern_learner.predict_next_requests(current_tool, user_session).await;

        for prediction in predictions {
            if prediction.probability > 0.3 { // Only prewarm high-probability predictions
                self.start_prewarming_task(prediction, validator_fn.clone()).await;
            }
        }
    }

    async fn start_prewarming_task<F, Fut>(
        &self,
        prediction: PredictionResult,
        validator_fn: F,
    ) where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ValidationCacheEntry, ramparts_common::anyhow::Error>> + Send,
    {
        let task_key = format!("{}_{}", prediction.tool_name, prediction.pattern_key);
        
        // Check if already prewarming
        {
            let tasks = self.prewarming_tasks.read().await;
            if tasks.contains_key(&task_key) {
                return;
            }
        }

        // Create synthetic request for prewarming
        let synthetic_request = self.create_synthetic_request(&prediction.tool_name);
        
        let handle = tokio::spawn(async move {
            // Wait for predicted time
            tokio::time::sleep(prediction.estimated_time_until.saturating_sub(Duration::from_millis(100))).await;
            
            // Pre-validate the request
            tracing::debug!("Pre-warming cache for tool: {}", prediction.tool_name);
            let _ = validator_fn(synthetic_request).await;
        });

        // Store the task handle
        let mut tasks = self.prewarming_tasks.write().await;
        tasks.insert(task_key, handle);

        // Clean up completed tasks
        tasks.retain(|_, handle| !handle.is_finished());
    }

    fn create_synthetic_request(&self, tool_name: &str) -> Value {
        // Create a minimal synthetic request for the predicted tool
        serde_json::json!({
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": self.get_common_arguments(tool_name)
            }
        })
    }

    fn get_common_arguments(&self, tool_name: &str) -> Value {
        // Return common/safe arguments for different tool types
        match tool_name {
            "read_file" => serde_json::json!({"path": "README.md"}),
            "list_files" => serde_json::json!({"path": "."}),
            "get_weather" => serde_json::json!({"location": "San Francisco"}),
            "search_web" => serde_json::json!({"query": "example"}),
            "calculate" => serde_json::json!({"expression": "2+2"}),
            _ => serde_json::json!({}),
        }
    }

    /// Get prewarming statistics
    pub async fn stats(&self) -> PrewarmingStats {
        let tasks = self.prewarming_tasks.read().await;
        PrewarmingStats {
            active_prewarming_tasks: tasks.len(),
            total_patterns_learned: {
                let patterns = self.pattern_learner.pattern_cache.read().await;
                patterns.len()
            },
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PrewarmingStats {
    pub active_prewarming_tasks: usize,
    pub total_patterns_learned: usize,
}

/// Smart cache with predictive capabilities
pub struct SmartCache {
    pattern_learner: Arc<PatternLearner>,
    prewarmer: CachePrewarmer,
    base_cache: Arc<crate::cache::ValidationCache>,
}

impl SmartCache {
    pub fn new(base_cache: Arc<crate::cache::ValidationCache>) -> Self {
        let pattern_learner = Arc::new(PatternLearner::new(10000));
        let prewarmer = CachePrewarmer::new(pattern_learner.clone());

        Self {
            pattern_learner,
            prewarmer,
            base_cache,
        }
    }

    /// Enhanced cache get with learning
    pub async fn get_with_learning(
        &self,
        request: &Value,
        user_session: Option<String>,
    ) -> Option<ValidationCacheEntry> {
        let tool_name = self.extract_tool_name(request);
        let start_time = Instant::now();
        
        let result = self.base_cache.get(request).await;
        let response_time = start_time.elapsed().as_millis() as u64;
        let cache_hit = result.is_some();

        // Record for learning
        if let Some(tool) = tool_name {
            self.pattern_learner.record_request(
                tool.clone(),
                request,
                user_session.clone(),
                response_time,
                cache_hit,
            ).await;

            // Start predictive prewarming
            if cache_hit {
                self.prewarmer.prewarm_cache(
                    &tool,
                    user_session.as_deref(),
                    |_req| async { 
                        // This would be the actual validator function
                        Ok(ValidationCacheEntry {
                            allowed: true,
                            reason: None,
                            confidence: Some(0.9),
                            timestamp: Instant::now(),
                        })
                    }
                ).await;
            }
        }

        result
    }

    fn extract_tool_name(&self, request: &Value) -> Option<String> {
        request
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
    }

    /// Get comprehensive cache statistics
    pub async fn comprehensive_stats(&self) -> SmartCacheStats {
        let base_stats = self.base_cache.stats().await;
        let prewarming_stats = self.prewarmer.stats().await;

        SmartCacheStats {
            base_cache_entries: base_stats.entries,
            patterns_learned: prewarming_stats.total_patterns_learned,
            active_prewarming: prewarming_stats.active_prewarming_tasks,
            prediction_accuracy: 0.0, // Would need to track this
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SmartCacheStats {
    pub base_cache_entries: u64,
    pub patterns_learned: usize,
    pub active_prewarming: usize,
    pub prediction_accuracy: f32,
}
