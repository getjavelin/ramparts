use ramparts_common::{anyhow::Result, tracing::debug};
use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for the MCP proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Listen address for the proxy server
    pub listen_address: String,
    
    /// Javelin API configuration
    pub javelin: JavelinConfig,
    
    /// Proxy behavior configuration
    pub behavior: ProxyBehavior,
}

/// Javelin API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavelinConfig {
    /// API key for Javelin Guardrails
    pub api_key: String,
    
    /// Base URL for Javelin API
    pub base_url: String,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Whether to fail open (allow) or closed (deny) when API is unavailable
    pub fail_open: bool,
}

/// Proxy behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyBehavior {
    /// Whether to log all requests and responses
    pub log_requests: bool,
    
    /// Whether to cache validation results
    pub cache_validations: bool,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Maximum request size in bytes
    pub max_request_size: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1:8080".to_string(),
            javelin: JavelinConfig::default(),
            behavior: ProxyBehavior::default(),
        }
    }
}

impl Default for JavelinConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.getjavelin.com".to_string(),
            timeout_seconds: 30,
            fail_open: true, // Default to fail-open for better availability
        }
    }
}

impl Default for ProxyBehavior {
    fn default() -> Self {
        Self {
            log_requests: true,
            cache_validations: false, // Disabled by default for security
            cache_ttl_seconds: 300,   // 5 minutes
            max_request_size: 1024 * 1024, // 1MB
        }
    }
}

impl ProxyConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        debug!("Loading proxy configuration from environment");
        
        let mut config = Self::default();
        
        // Load listen address
        if let Ok(addr) = env::var("PROXY_LISTEN_ADDRESS") {
            config.listen_address = addr;
        }
        
        // Load Javelin configuration - optional for testing
        config.javelin.api_key = env::var("JAVELIN_API_KEY")
            .unwrap_or_else(|_| {
                debug!("JAVELIN_API_KEY not set, using test mode");
                "test-mode".to_string()
            });
        
        if let Ok(url) = env::var("JAVELIN_API_URL") {
            config.javelin.base_url = url;
        }
        
        if let Ok(timeout) = env::var("JAVELIN_TIMEOUT_SECONDS") {
            config.javelin.timeout_seconds = timeout.parse()
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid timeout value: {}", e))?;
        }
        
        if let Ok(fail_open) = env::var("JAVELIN_FAIL_OPEN") {
            config.javelin.fail_open = fail_open.parse()
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid fail_open value: {}", e))?;
        }
        
        // Load behavior configuration
        if let Ok(log_requests) = env::var("PROXY_LOG_REQUESTS") {
            config.behavior.log_requests = log_requests.parse()
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid log_requests value: {}", e))?;
        }
        
        if let Ok(cache) = env::var("PROXY_CACHE_VALIDATIONS") {
            config.behavior.cache_validations = cache.parse()
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid cache_validations value: {}", e))?;
        }
        
        if let Ok(ttl) = env::var("PROXY_CACHE_TTL_SECONDS") {
            config.behavior.cache_ttl_seconds = ttl.parse()
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid cache_ttl_seconds value: {}", e))?;
        }
        
        if let Ok(size) = env::var("PROXY_MAX_REQUEST_SIZE") {
            config.behavior.max_request_size = size.parse()
                .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid max_request_size value: {}", e))?;
        }
        
        Ok(config)
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.javelin.api_key.is_empty() {
            return Err(ramparts_common::anyhow::anyhow!(
                "Javelin API key must not be empty"
            ));
        }
        if self.javelin.api_key == "test-mode" {
            debug!("Running in test mode without Javelin validation");
        }
        
        if self.javelin.timeout_seconds == 0 {
            return Err(ramparts_common::anyhow::anyhow!("Timeout must be greater than 0"));
        }
        
        if self.behavior.max_request_size == 0 {
            return Err(ramparts_common::anyhow::anyhow!("Max request size must be greater than 0"));
        }
        
        // Validate listen address format strictly
        if self.listen_address.parse::<std::net::SocketAddr>().is_err() {
            return Err(ramparts_common::anyhow::anyhow!(
                "Invalid listen address format; expected host:port"
            ));
        }
        
        Ok(())
    }
}
