use crate::config::{ScannerConfig, ScannerConfigManager};
use crate::scanner::MCPScanner;
use crate::types::{scan_config_utils, ScanConfigBuilder, ScanOptions, ScanResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanRequest {
    pub url: String,
    pub timeout: Option<u64>,
    pub http_timeout: Option<u64>,
    pub detailed: Option<bool>,
    pub format: Option<String>,
    pub auth_headers: Option<HashMap<String, String>>,
    /// If true, do not call the LLM; return prompts instead
    pub return_prompts: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResponse {
    pub success: bool,
    pub result: Option<ScanResult>,
    pub error: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScanRequest {
    pub urls: Vec<String>,
    pub options: Option<ScanRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScanResponse {
    pub success: bool,
    pub results: Vec<ScanResponse>,
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub success: bool,
    pub valid: bool,
    pub error: Option<String>,
    pub timestamp: String,
}

pub struct MCPScannerCore {
    scanner: MCPScanner,
    config_manager: ScannerConfigManager,
}

impl MCPScannerCore {
    pub fn new() -> Result<Self> {
        let config_manager = ScannerConfigManager::new();
        let scanner_config = match config_manager.load_config() {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to load scanner config, using defaults: {}", e);
                ScannerConfig::default()
            }
        };

        Ok(Self {
            scanner: MCPScanner::with_timeout(scanner_config.scanner.http_timeout)?,
            config_manager,
        })
    }

    /// Parse scan options from request parameters
    fn parse_scan_options(&self, request: &ScanRequest) -> ScanOptions {
        let scanner_config = self.config_manager.load_config().unwrap_or_default();

        let mut builder = ScanConfigBuilder::new()
            .timeout(
                request
                    .timeout
                    .unwrap_or(scanner_config.scanner.scan_timeout),
            )
            .http_timeout(
                request
                    .http_timeout
                    .unwrap_or(scanner_config.scanner.http_timeout),
            )
            .detailed(request.detailed.unwrap_or(scanner_config.scanner.detailed))
            .format(
                request
                    .format
                    .clone()
                    .unwrap_or(scanner_config.scanner.format),
            );

        // Handle auth headers with minimal conversion for Javelin API key
        if let Some(auth_headers) = &request.auth_headers {
            let mut headers = auth_headers.clone();

            // If we have x-javelin-api-key, add the formats that work with Javelin MCP
            if let Some(api_key) = auth_headers.get("x-javelin-api-key") {
                // Only proceed if the API key is not empty
                if !api_key.trim().is_empty() {
                    // Add x-javelin-apikey format
                    headers.insert("x-javelin-apikey".to_string(), api_key.clone());

                    // Only add authorization header if one doesn't already exist (case-insensitive check)
                    let has_auth_header = headers
                        .keys()
                        .any(|key| key.to_lowercase() == "authorization");
                    if !has_auth_header {
                        headers.insert("authorization".to_string(), format!("Bearer {api_key}"));
                    }
                }
            }

            builder = builder.auth_headers(Some(headers));
        }

        if let Some(rp) = request.return_prompts {
            builder = builder.return_prompts(rp);
        }

        builder.build()
    }

    /// Perform a scan with the given options
    pub async fn scan(&self, request: ScanRequest) -> ScanResponse {
        let timestamp = chrono::Utc::now().to_rfc3339();

        match self.perform_scan_internal(request).await {
            Ok(result) => ScanResponse {
                success: true,
                result: Some(result),
                error: None,
                timestamp,
            },
            Err(e) => ScanResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                timestamp,
            },
        }
    }

    /// Internal scan implementation
    async fn perform_scan_internal(&self, request: ScanRequest) -> Result<ScanResult> {
        // Parse and validate options
        let scan_options = self.parse_scan_options(&request);

        // Validate configuration
        scan_config_utils::validate_scan_config(&scan_options)
            .map_err(|e| anyhow!("Configuration validation failed: {}", e))?;

        // Perform scan
        let result = self.scanner.scan_single(&request.url, scan_options).await?;
        Ok(result)
    }

    /// Validate scan configuration
    pub fn validate_config(&self, request: &ScanRequest) -> ValidationResponse {
        let timestamp = chrono::Utc::now().to_rfc3339();

        let options = self.parse_scan_options(request); // No conversion for validation
        match scan_config_utils::validate_scan_config(&options) {
            Ok(()) => ValidationResponse {
                success: true,
                valid: true,
                error: None,
                timestamp,
            },
            Err(e) => ValidationResponse {
                success: false,
                valid: false,
                error: Some(e.to_string()),
                timestamp,
            },
        }
    }

    /// Perform batch scan of multiple URLs
    pub async fn batch_scan(&self, request: BatchScanRequest) -> BatchScanResponse {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut results = Vec::new();

        // Process URLs sequentially to avoid overwhelming servers
        let default_options = request.options.clone().unwrap_or_default();
        for url in &request.urls {
            let scan_request = ScanRequest {
                url: url.clone(),
                ..default_options.clone()
            };

            let response = self.scan(scan_request).await;
            results.push(response);
        }

        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;

        BatchScanResponse {
            success: failed == 0, // Set success to false if any scans failed
            results,
            total: request.urls.len(),
            successful,
            failed,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_scan_request_creation() {
        let request = ScanRequest {
            url: "http://example.com".to_string(),
            timeout: Some(60),
            http_timeout: Some(30),
            detailed: Some(true),
            format: Some("json".to_string()),
            auth_headers: Some(HashMap::from([(
                "Authorization".to_string(),
                "Bearer token".to_string(),
            )])),
            return_prompts: Some(false),
        };

        assert_eq!(request.url, "http://example.com");
        assert_eq!(request.timeout, Some(60));
        assert_eq!(request.http_timeout, Some(30));
        assert_eq!(request.detailed, Some(true));
        assert_eq!(request.format, Some("json".to_string()));
        assert!(request.auth_headers.is_some());
    }

    #[test]
    fn test_scan_request_default() {
        let request = ScanRequest::default();
        assert_eq!(request.url, "");
        assert_eq!(request.timeout, None);
        assert_eq!(request.http_timeout, None);
        assert_eq!(request.detailed, None);
        assert_eq!(request.format, None);
        assert_eq!(request.auth_headers, None);
    }

    #[test]
    fn test_scan_response_creation() {
        let result = ScanResult::new("http://example.com".to_string());
        let response = ScanResponse {
            success: true,
            result: Some(result),
            error: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(response.success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.timestamp, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_scan_response_error() {
        let response = ScanResponse {
            success: false,
            result: None,
            error: Some("Test error".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(!response.success);
        assert!(response.result.is_none());
        assert_eq!(response.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_batch_scan_request() {
        let urls = vec![
            "http://example1.com".to_string(),
            "http://example2.com".to_string(),
        ];
        let options = ScanRequest {
            url: String::new(),
            timeout: Some(60),
            http_timeout: Some(30),
            detailed: Some(false),
            format: Some("text".to_string()),
            auth_headers: None,
            return_prompts: Some(false),
        };

        let request = BatchScanRequest {
            urls: urls.clone(),
            options: Some(options),
        };

        assert_eq!(request.urls.len(), 2);
        assert_eq!(request.urls[0], "http://example1.com");
        assert_eq!(request.urls[1], "http://example2.com");
        assert!(request.options.is_some());
    }

    #[test]
    fn test_batch_scan_response() {
        let results = vec![
            ScanResponse {
                success: true,
                result: Some(ScanResult::new("http://example1.com".to_string())),
                error: None,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            },
            ScanResponse {
                success: false,
                result: None,
                error: Some("Failed".to_string()),
                timestamp: "2024-01-01T00:00:01Z".to_string(),
            },
        ];

        let response = BatchScanResponse {
            success: false, // Should be false when there are failures
            results: results.clone(),
            total: 2,
            successful: 1,
            failed: 1,
            timestamp: "2024-01-01T00:00:02Z".to_string(),
        };

        assert!(!response.success); // Should be false when there are failures
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.total, 2);
        assert_eq!(response.successful, 1);
        assert_eq!(response.failed, 1);
    }

    #[test]
    fn test_batch_scan_response_all_successful() {
        let results = vec![
            ScanResponse {
                success: true,
                result: Some(ScanResult::new("http://example1.com".to_string())),
                error: None,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            },
            ScanResponse {
                success: true,
                result: Some(ScanResult::new("http://example2.com".to_string())),
                error: None,
                timestamp: "2024-01-01T00:00:01Z".to_string(),
            },
        ];

        let response = BatchScanResponse {
            success: true, // Should be true when all scans are successful
            results: results.clone(),
            total: 2,
            successful: 2,
            failed: 0,
            timestamp: "2024-01-01T00:00:02Z".to_string(),
        };

        assert!(response.success); // Should be true when all scans are successful
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.total, 2);
        assert_eq!(response.successful, 2);
        assert_eq!(response.failed, 0);
    }

    #[test]
    fn test_validation_response() {
        let response = ValidationResponse {
            success: true,
            valid: true,
            error: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(response.success);
        assert!(response.valid);
        assert!(response.error.is_none());
        assert_eq!(response.timestamp, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_validation_response_invalid() {
        let response = ValidationResponse {
            success: false,
            valid: false,
            error: Some("Invalid configuration".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(!response.success);
        assert!(!response.valid);
        assert_eq!(response.error, Some("Invalid configuration".to_string()));
    }

    #[test]
    fn test_mcp_scanner_core_creation() {
        let core = MCPScannerCore::new();
        assert!(core.is_ok());
    }

    #[test]
    fn test_parse_scan_options() {
        let core = MCPScannerCore::new().unwrap();
        let request = ScanRequest {
            url: "http://example.com".to_string(),
            timeout: Some(120),
            http_timeout: Some(60),
            detailed: Some(true),
            format: Some("json".to_string()),
            auth_headers: Some(HashMap::from([(
                "Authorization".to_string(),
                "Bearer token".to_string(),
            )])),
            return_prompts: Some(false),
        };

        let options = core.parse_scan_options(&request); // No conversion for test
        assert_eq!(options.timeout, 120);
        assert_eq!(options.http_timeout, 60);
        assert!(options.detailed);
        assert_eq!(options.format, "json");
        assert!(options.auth_headers.is_some());
    }

    #[test]
    fn test_parse_scan_options_with_defaults() {
        let core = MCPScannerCore::new().unwrap();
        let request = ScanRequest {
            url: "http://example.com".to_string(),
            timeout: None,
            http_timeout: None,
            detailed: None,
            format: None,
            auth_headers: None,
            return_prompts: None,
        };

        let options = core.parse_scan_options(&request); // No conversion for test
                                                         // These will use default values from config
        assert!(options.timeout > 0);
        assert!(options.http_timeout > 0);
        assert!(!options.detailed); // Default is false
        assert_eq!(options.format, "table"); // Default is table
        assert!(options.auth_headers.is_none());
    }
}
