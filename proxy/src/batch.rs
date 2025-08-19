use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{interval, Instant};
use serde_json::Value;
use crate::cache::ValidationCacheEntry;
use ramparts_common::anyhow::Result;

/// Batch validation request
#[derive(Debug)]
pub struct BatchRequest {
    pub id: String,
    pub request: Value,
    pub response_tx: oneshot::Sender<Result<ValidationCacheEntry>>,
}

/// Configuration for request batching
#[derive(Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait_time: Duration,
    pub enable_batching: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10,
            max_wait_time: Duration::from_millis(50), // 50ms max wait
            enable_batching: true,
        }
    }
}

/// Request batcher for optimizing API calls
pub struct RequestBatcher {
    config: BatchConfig,
    pending_requests: Arc<Mutex<Vec<BatchRequest>>>,
    request_tx: mpsc::UnboundedSender<BatchRequest>,
    _handle: tokio::task::JoinHandle<()>,
}

impl RequestBatcher {
    pub fn new<F, Fut>(config: BatchConfig, batch_processor: F) -> Self
    where
        F: Fn(Vec<(String, Value)>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Vec<(String, ValidationCacheEntry)>>> + Send,
    {
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<BatchRequest>();
        let pending_requests = Arc::new(Mutex::new(Vec::new()));
        let pending_requests_clone = pending_requests.clone();
        let config_clone = config.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(config_clone.max_wait_time);
            
            loop {
                tokio::select! {
                    // New request received
                    Some(request) = request_rx.recv() => {
                        let mut pending = pending_requests_clone.lock().await;
                        pending.push(request);
                        
                        // Check if we should process the batch
                        if pending.len() >= config_clone.max_batch_size {
                            let batch = std::mem::take(&mut *pending);
                            drop(pending);
                            
                            Self::process_batch(batch, &batch_processor).await;
                        }
                    }
                    
                    // Timer expired, process any pending requests
                    _ = interval.tick() => {
                        let mut pending = pending_requests_clone.lock().await;
                        if !pending.is_empty() {
                            let batch = std::mem::take(&mut *pending);
                            drop(pending);
                            
                            Self::process_batch(batch, &batch_processor).await;
                        }
                    }
                }
            }
        });

        Self {
            config,
            pending_requests,
            request_tx,
            _handle: handle,
        }
    }

    async fn process_batch<F, Fut>(batch: Vec<BatchRequest>, processor: &F)
    where
        F: Fn(Vec<(String, Value)>) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<(String, ValidationCacheEntry)>>>,
    {
        if batch.is_empty() {
            return;
        }

        tracing::debug!("Processing batch of {} requests", batch.len());
        let start_time = Instant::now();

        // Extract requests for processing
        let requests: Vec<(String, Value)> = batch
            .iter()
            .map(|req| (req.id.clone(), req.request.clone()))
            .collect();

        // Process the batch
        match processor(requests).await {
            Ok(results) => {
                // Create a map for quick lookup
                let result_map: HashMap<String, ValidationCacheEntry> = results.into_iter().collect();
                
                // Send responses back to waiting requests
                for batch_req in batch {
                    let result = result_map.get(&batch_req.id).cloned()
                        .ok_or_else(|| ramparts_common::anyhow::anyhow!("Missing result for request {}", batch_req.id));
                    
                    let _ = batch_req.response_tx.send(result);
                }
            }
            Err(e) => {
                // Send error to all waiting requests
                for batch_req in batch {
                    let _ = batch_req.response_tx.send(Err(ramparts_common::anyhow::anyhow!("Batch processing failed: {}", e)));
                }
            }
        }

        let duration = start_time.elapsed();
        tracing::debug!("Batch processing completed in {:?}", duration);
    }

    /// Submit a request for batched processing
    pub async fn submit_request(&self, id: String, request: Value) -> Result<ValidationCacheEntry> {
        if !self.config.enable_batching {
            return Err(ramparts_common::anyhow::anyhow!("Batching is disabled"));
        }

        let (response_tx, response_rx) = oneshot::channel();
        
        let batch_request = BatchRequest {
            id,
            request,
            response_tx,
        };

        self.request_tx.send(batch_request)
            .map_err(|_| ramparts_common::anyhow::anyhow!("Failed to submit request to batch processor"))?;

        response_rx.await
            .map_err(|_| ramparts_common::anyhow::anyhow!("Failed to receive batch response"))?
    }

    /// Get current batch statistics
    pub async fn stats(&self) -> BatchStats {
        let pending = self.pending_requests.lock().await;
        BatchStats {
            pending_requests: pending.len(),
            max_batch_size: self.config.max_batch_size,
            max_wait_time_ms: self.config.max_wait_time.as_millis() as u64,
            batching_enabled: self.config.enable_batching,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BatchStats {
    pub pending_requests: usize,
    pub max_batch_size: usize,
    pub max_wait_time_ms: u64,
    pub batching_enabled: bool,
}

/// Circuit breaker for handling API failures
pub struct CircuitBreaker {
    failure_count: Arc<Mutex<u32>>,
    last_failure: Arc<Mutex<Option<Instant>>>,
    config: CircuitBreakerConfig,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            failure_count: Arc::new(Mutex::new(0)),
            last_failure: Arc::new(Mutex::new(None)),
            config,
        }
    }

    pub async fn state(&self) -> CircuitState {
        let failure_count = *self.failure_count.lock().await;
        let last_failure = *self.last_failure.lock().await;

        if failure_count < self.config.failure_threshold {
            return CircuitState::Closed;
        }

        if let Some(last_failure_time) = last_failure {
            if last_failure_time.elapsed() > self.config.recovery_timeout {
                return CircuitState::HalfOpen;
            }
        }

        CircuitState::Open
    }

    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        match self.state().await {
            CircuitState::Open => {
                return Err(ramparts_common::anyhow::anyhow!("Circuit breaker is open"));
            }
            CircuitState::Closed | CircuitState::HalfOpen => {
                match operation().await {
                    Ok(result) => {
                        self.record_success().await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.record_failure().await;
                        Err(e)
                    }
                }
            }
        }
    }

    async fn record_success(&self) {
        let mut failure_count = self.failure_count.lock().await;
        *failure_count = 0;
        
        let mut last_failure = self.last_failure.lock().await;
        *last_failure = None;
    }

    async fn record_failure(&self) {
        let mut failure_count = self.failure_count.lock().await;
        *failure_count += 1;
        
        let mut last_failure = self.last_failure.lock().await;
        *last_failure = Some(Instant::now());
    }

    pub async fn stats(&self) -> CircuitBreakerStats {
        let failure_count = *self.failure_count.lock().await;
        let state = self.state().await;
        
        CircuitBreakerStats {
            state: format!("{:?}", state),
            failure_count,
            failure_threshold: self.config.failure_threshold,
            recovery_timeout_secs: self.config.recovery_timeout.as_secs(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CircuitBreakerStats {
    pub state: String,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub recovery_timeout_secs: u64,
}
