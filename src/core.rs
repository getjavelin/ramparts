use crate::config::ScannerConfigManager;
use crate::scanner::MCPScanner;
use crate::types::{config_utils, ScanConfigBuilder, ScanOptions, ScanResult};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanRequest {
    pub url: String,
    pub timeout: Option<u64>,
    pub http_timeout: Option<u64>,
    pub detailed: Option<bool>,
    pub format: Option<String>,
    pub auth_headers: Option<HashMap<String, String>>,
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
        let scanner_config = config_manager.load_config().unwrap_or_default();

        Ok(Self {
            scanner: MCPScanner::new_with_timeout(scanner_config.scanner.http_timeout),
            config_manager,
        })
    }

    /// Parse scan options from request parameters
    fn parse_scan_options(&self, request: &ScanRequest) -> Result<ScanOptions> {
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

        if let Some(auth_headers) = &request.auth_headers {
            builder = builder.auth_headers(Some(auth_headers.clone()));
        }

        Ok(builder.build())
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
        let scan_options = self.parse_scan_options(&request)?;

        // Validate configuration
        config_utils::validate_scan_config(&scan_options)
            .map_err(|e| anyhow!("Configuration validation failed: {}", e))?;

        // Perform scan
        let result = self.scanner.scan_single(&request.url, scan_options).await?;
        Ok(result)
    }

    /// Validate scan configuration
    pub fn validate_config(&self, request: &ScanRequest) -> ValidationResponse {
        let timestamp = chrono::Utc::now().to_rfc3339();

        match self.parse_scan_options(request) {
            Ok(options) => match config_utils::validate_scan_config(&options) {
                Ok(_) => ValidationResponse {
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
            success: true,
            results,
            total: request.urls.len(),
            successful,
            failed,
            timestamp,
        }
    }
}
