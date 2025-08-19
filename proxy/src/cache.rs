use std::time::Duration;
use moka::future::Cache;
use serde_json::Value;
use sha2::{Sha256, Digest};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::Instant;

/// Cache entry for validation results
#[derive(Clone, Debug)]
pub struct ValidationCacheEntry {
    pub allowed: bool,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub timestamp: Instant,
}

/// High-performance cache for Javelin validation results
pub struct ValidationCache {
    // Fast in-memory cache for recent results
    memory_cache: Cache<String, ValidationCacheEntry>,
    
    // Request deduplication to prevent duplicate API calls
    pending_requests: Arc<DashMap<String, tokio::sync::broadcast::Sender<ValidationCacheEntry>>>,
    
    // Configuration
    config: CacheConfig,
}

#[derive(Clone)]
pub struct CacheConfig {
    pub max_entries: u64,
    pub ttl: Duration,
    pub enable_deduplication: bool,
    pub hash_sensitive_data: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl: Duration::from_secs(300), // 5 minutes
            enable_deduplication: true,
            hash_sensitive_data: true,
        }
    }
}

impl ValidationCache {
    pub fn new(config: CacheConfig) -> Self {
        let memory_cache = Cache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl)
            .build();

        Self {
            memory_cache,
            pending_requests: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Generate a cache key for a request
    pub fn generate_cache_key(&self, request: &Value) -> String {
        if self.config.hash_sensitive_data {
            // Hash the request for privacy and consistent key length
            let mut hasher = Sha256::new();
            hasher.update(serde_json::to_string(request).unwrap_or_default().as_bytes());
            format!("req_{:x}", hasher.finalize())
        } else {
            // Use request content directly (for debugging)
            format!("req_{}", serde_json::to_string(request).unwrap_or_default())
        }
    }

    /// Get cached validation result
    pub async fn get(&self, request: &Value) -> Option<ValidationCacheEntry> {
        let key = self.generate_cache_key(request);
        
        if let Some(entry) = self.memory_cache.get(&key).await {
            // Check if entry is still fresh (additional TTL check)
            if entry.timestamp.elapsed() < self.config.ttl {
                tracing::debug!("Cache HIT for request key: {}", &key[..16]);
                return Some(entry);
            } else {
                // Remove expired entry
                self.memory_cache.invalidate(&key).await;
            }
        }
        
        tracing::debug!("Cache MISS for request key: {}", &key[..16]);
        None
    }

    /// Store validation result in cache
    pub async fn set(&self, request: &Value, result: ValidationCacheEntry) {
        let key = self.generate_cache_key(request);
        self.memory_cache.insert(key.clone(), result).await;
        tracing::debug!("Cache SET for request key: {}", &key[..16]);
    }

    /// Get or compute validation result with deduplication
    pub async fn get_or_compute<F, Fut>(&self, request: &Value, compute_fn: F) -> Result<ValidationCacheEntry, ramparts_common::anyhow::Error>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<ValidationCacheEntry, ramparts_common::anyhow::Error>>,
    {
        // First check cache
        if let Some(cached) = self.get(request).await {
            return Ok(cached);
        }

        let key = self.generate_cache_key(request);

        // If deduplication is enabled, check for pending requests
        if self.config.enable_deduplication {
            // Check if there's already a pending request for this key
            if let Some(sender) = self.pending_requests.get(&key) {
                tracing::debug!("Request deduplication: waiting for pending request {}", &key[..16]);
                
                let mut receiver = sender.subscribe();
                match receiver.recv().await {
                    Ok(result) => return Ok(result),
                    Err(_) => {
                        // Sender was dropped, continue with new request
                        tracing::debug!("Pending request sender dropped, making new request");
                    }
                }
            }

            // Create a new broadcast channel for this request
            let (sender, _) = tokio::sync::broadcast::channel(1);
            self.pending_requests.insert(key.clone(), sender.clone());

            // Compute the result
            let result = compute_fn().await;

            // Remove from pending requests
            self.pending_requests.remove(&key);

            match result {
                Ok(validation_result) => {
                    // Cache the result
                    self.set(request, validation_result.clone()).await;
                    
                    // Broadcast to any waiting requests
                    let _ = sender.send(validation_result.clone());
                    
                    Ok(validation_result)
                }
                Err(e) => {
                    // Don't cache errors, just return them
                    Err(e)
                }
            }
        } else {
            // No deduplication, just compute and cache
            let result = compute_fn().await?;
            self.set(request, result.clone()).await;
            Ok(result)
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.memory_cache.entry_count(),
            pending_requests: self.pending_requests.len(),
            max_capacity: self.config.max_entries,
            ttl_seconds: self.config.ttl.as_secs(),
        }
    }

    /// Clear all cached entries
    pub async fn clear(&self) {
        self.memory_cache.invalidate_all();
        self.pending_requests.clear();
        tracing::info!("Validation cache cleared");
    }

    /// Remove expired entries (called periodically)
    pub async fn cleanup(&self) {
        // Moka handles TTL automatically, but we can force cleanup
        self.memory_cache.run_pending_tasks().await;
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CacheStats {
    pub entries: u64,
    pub pending_requests: usize,
    pub max_capacity: u64,
    pub ttl_seconds: u64,
}

/// Batch validation request for multiple requests
#[derive(Debug)]
pub struct BatchValidationRequest {
    pub requests: Vec<(String, Value)>, // (id, request)
}

#[derive(Debug)]
pub struct BatchValidationResponse {
    pub results: Vec<(String, ValidationCacheEntry)>, // (id, result)
}

impl ValidationCache {
    /// Batch validation for multiple requests (future optimization)
    pub async fn batch_validate<F, Fut>(&self, batch: BatchValidationRequest, compute_fn: F) -> Result<BatchValidationResponse, ramparts_common::anyhow::Error>
    where
        F: FnOnce(Vec<(String, Value)>) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<(String, ValidationCacheEntry)>, ramparts_common::anyhow::Error>>,
    {
        let mut cached_results = Vec::new();
        let mut uncached_requests = Vec::new();

        // Check cache for each request
        for (id, request) in batch.requests {
            if let Some(cached) = self.get(&request).await {
                cached_results.push((id, cached));
            } else {
                uncached_requests.push((id, request));
            }
        }

        // If all requests were cached, return immediately
        if uncached_requests.is_empty() {
            return Ok(BatchValidationResponse {
                results: cached_results,
            });
        }

        // Compute uncached requests
        let computed_results = compute_fn(uncached_requests.clone()).await?;

        // Cache the computed results
        for (id, result) in &computed_results {
            if let Some((_, request)) = uncached_requests.iter().find(|(req_id, _)| req_id == id) {
                self.set(request, result.clone()).await;
            }
        }

        // Combine cached and computed results
        let mut all_results = cached_results;
        all_results.extend(computed_results);

        Ok(BatchValidationResponse {
            results: all_results,
        })
    }
}
