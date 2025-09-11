use crate::types::MCPTool;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Cache entry for MCP tools with TTL support
#[derive(Debug, Clone)]
#[allow(dead_code)] // Future feature - will be used when cache is integrated
pub struct CacheEntry {
    pub tools: Vec<MCPTool>,
    pub cached_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

#[allow(dead_code)] // Future feature - will be used when cache is integrated
impl CacheEntry {
    /// Create a new cache entry with the specified TTL
    pub fn new(tools: Vec<MCPTool>, ttl_seconds: u64) -> Self {
        Self {
            tools,
            cached_at: Utc::now(),
            ttl_seconds,
        }
    }

    /// Check if the cache entry has expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        let expiry_time = self.cached_at + chrono::Duration::seconds(self.ttl_seconds as i64);
        now > expiry_time
    }

    /// Get the remaining TTL in seconds
    pub fn remaining_ttl(&self) -> i64 {
        let now = Utc::now();
        let expiry_time = self.cached_at + chrono::Duration::seconds(self.ttl_seconds as i64);
        (expiry_time - now).num_seconds()
    }
}

/// Tool cache with TTL support for MCP server tool descriptions
#[derive(Debug, Clone)]
#[allow(dead_code)] // Future feature - will be used when cache is integrated
pub struct ToolCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    default_ttl: u64,
}

#[allow(dead_code)] // Future feature - will be used when cache is integrated
impl ToolCache {
    /// Create a new tool cache with the specified default TTL in seconds
    pub fn new(default_ttl: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Get tools from cache if available and not expired
    pub async fn get(&self, url: &str) -> Option<Vec<MCPTool>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(url) {
            if !entry.is_expired() {
                debug!(
                    "Cache hit for {}: {} tools, TTL remaining: {}s",
                    url,
                    entry.tools.len(),
                    entry.remaining_ttl()
                );
                return Some(entry.tools.clone());
            } else {
                debug!("Cache entry for {} has expired", url);
            }
        }
        None
    }

    /// Store tools in cache with default TTL
    pub async fn put(&self, url: String, tools: Vec<MCPTool>) {
        self.put_with_ttl(url, tools, self.default_ttl).await;
    }

    /// Store tools in cache with custom TTL
    pub async fn put_with_ttl(&self, url: String, tools: Vec<MCPTool>, ttl_seconds: u64) {
        let entry = CacheEntry::new(tools.clone(), ttl_seconds);
        let mut cache = self.cache.write().await;
        cache.insert(url.clone(), entry);
        debug!(
            "Cached {} tools for {} with TTL {}s",
            tools.len(),
            url,
            ttl_seconds
        );
    }

    /// Remove a specific entry from cache
    pub async fn remove(&self, url: &str) -> bool {
        let mut cache = self.cache.write().await;
        let removed = cache.remove(url).is_some();
        if removed {
            debug!("Removed cache entry for {}", url);
        }
        removed
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        debug!("Cleared {} cache entries", count);
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_count = cache.len();

        cache.retain(|url, entry| {
            if entry.is_expired() {
                debug!("Removing expired cache entry for {}", url);
                false
            } else {
                true
            }
        });

        let removed_count = initial_count - cache.len();
        if removed_count > 0 {
            debug!("Cleaned up {} expired cache entries", removed_count);
        }
        removed_count
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let mut expired_entries = 0;
        let mut total_tools = 0;

        for entry in cache.values() {
            if entry.is_expired() {
                expired_entries += 1;
            }
            total_tools += entry.tools.len();
        }

        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
            total_tools,
            default_ttl: self.default_ttl,
        }
    }

    /// Check if cache contains a specific URL (regardless of expiry)
    pub async fn contains(&self, url: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(url)
    }

    /// Get all cached URLs
    pub async fn get_cached_urls(&self) -> Vec<String> {
        let cache = self.cache.read().await;
        cache.keys().cloned().collect()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
#[allow(dead_code)] // Future feature - will be used when cache is integrated
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub total_tools: usize,
    pub default_ttl: u64,
}

impl Default for ToolCache {
    fn default() -> Self {
        // Default TTL of 1 hour (3600 seconds)
        Self::new(3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MCPTool;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    fn create_test_tool(name: &str) -> MCPTool {
        MCPTool {
            name: name.to_string(),
            description: Some(format!("Test tool {name}")),
            input_schema: None,
            output_schema: None,
            parameters: HashMap::new(),
            category: None,
            tags: vec![],
            deprecated: false,
            raw_json: None,
        }
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = ToolCache::new(60); // 1 minute TTL
        let url = "http://example.com";
        let tools = vec![create_test_tool("test1"), create_test_tool("test2")];

        // Initially empty
        assert!(cache.get(url).await.is_none());

        // Store and retrieve
        cache.put(url.to_string(), tools.clone()).await;
        let cached_tools = cache.get(url).await.unwrap();
        assert_eq!(cached_tools.len(), 2);
        assert_eq!(cached_tools[0].name, "test1");

        // Remove
        assert!(cache.remove(url).await);
        assert!(cache.get(url).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let cache = ToolCache::new(1); // 1 second TTL
        let url = "http://example.com";
        let tools = vec![create_test_tool("test")];

        cache.put(url.to_string(), tools).await;

        // Should be available immediately
        assert!(cache.get(url).await.is_some());

        // Wait for expiry
        sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get(url).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = ToolCache::new(60);
        let tools = vec![create_test_tool("test")];

        cache.put("url1".to_string(), tools.clone()).await;
        cache.put("url2".to_string(), tools).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
        assert_eq!(stats.expired_entries, 0);
        assert_eq!(stats.total_tools, 2);
    }
}
