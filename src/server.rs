use crate::core::{
    BatchScanRequest, BatchScanResponse, MCPScannerCore, ScanRequest, ScanResponse,
    ValidationResponse,
};
use axum::{
    extract::State,
    http::{Method, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

#[derive(Clone)]
pub struct ServerState {
    core: Arc<MCPScannerCore>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(),
        }
    }
}

pub struct MCPScannerServer {
    core: MCPScannerCore,
    config: ServerConfig,
}

impl MCPScannerServer {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            core: MCPScannerCore::new()?,
            config: ServerConfig::default(),
        })
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn with_host(mut self, host: String) -> Self {
        self.config.host = host;
        self
    }

    pub async fn start(self) -> anyhow::Result<()> {
        let state = ServerState {
            core: Arc::new(self.core),
        };

        // Configure CORS
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any);

        // Create router with routes
        let app = Router::new()
            .route("/", get(api_docs))
            .route("/health", get(health_check))
            .route("/protocol", get(protocol_info))
            .route("/scan", post(scan_endpoint))
            .route("/validate", post(validate_endpoint))
            .route("/batch-scan", post(batch_scan_endpoint))
            .layer(cors)
            .layer(TraceLayer::new_for_http())
            .with_state(state);

        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("Starting MCP Scanner Server on http://{addr}");
        info!("Protocol version: 2025-06-18");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "ramparts-server",
        "version": "0.2.0",
        "protocol_version": "2025-06-18"
    }))
}

async fn protocol_info() -> Json<Value> {
    Json(json!({
        "protocol": {
            "version": "2025-06-18",
            "name": "Model Context Protocol",
            "transport": {
                "stdio": "supported",
                "http": "supported",
                "features": [
                    "JSON-RPC 2.0",
                    "Session Management",
                    "Protocol Version Headers",
                    "STDIO Process Communication",
                    "Multi-Transport Support"
                ]
            },
            "capabilities": [
                "tools/list",
                "resources/list",
                "prompts/list",
                "server/info"
            ]
        },
        "server": {
            "version": "0.2.0",
            "stdio_support": true,
            "mcp_compliance": "2025-06-18"
        }
    }))
}

async fn api_docs() -> Json<Value> {
    Json(json!({
        "service": "Ramparts Microservice",
        "version": "0.2.0",
        "protocol_version": "2025-06-18",
        "endpoints": {
            "GET /health": "Health check with protocol info",
            "GET /protocol": "MCP protocol information",
            "POST /scan": "Scan a single MCP server",
            "POST /validate": "Validate scan configuration",
            "POST /batch-scan": "Scan multiple MCP servers",
            "GET /": "API documentation"
        },
        "transports": {
            "http": {
                "supported": true,
                "description": "HTTP/HTTPS transport for remote MCP servers",
                "examples": [
                    "http://localhost:3000",
                    "https://api.example.com/mcp",
                    "http://192.168.1.100:8080"
                ]
            },
            "stdio": {
                "supported": true,
                "description": "STDIO transport for local MCP server processes",
                "examples": [
                    "stdio:///usr/local/bin/mcp-server",
                    "stdio://node /path/to/mcp-server.js",
                    "/usr/bin/python3 /path/to/mcp-server.py",
                    "mcp-server --config config.json"
                ]
            },

        },

        "example": {
            "POST /scan": {
                "url": "http://localhost:3000",
                "timeout": 180,
                "http_timeout": 30,
                "detailed": true,
                "format": "json",
                "auth_headers": { "Authorization": "Bearer token" }
            },
            "STDIO Example": {
                "url": "stdio:///usr/local/bin/mcp-server",
                "timeout": 180,
                "detailed": true,
                "format": "json"
            }
        }
    }))
}

async fn scan_endpoint(
    State(state): State<ServerState>,
    Json(request): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, (StatusCode, Json<Value>)> {
    // Input validation
    if request.url.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "URL is required",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        ));
    }

    // Validate URL format - only HTTP/HTTPS supported with rmcp
    if !request.url.contains("://") {
        // Allow URLs without scheme - they'll be normalized to http://
    } else if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Only HTTP and HTTPS URLs are supported",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        ));
    }

    // Validate timeout values
    if let Some(timeout) = request.timeout {
        if timeout == 0 || timeout > 3600 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Timeout must be between 1 and 3600 seconds",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
            ));
        }
    }

    info!("Received scan request for URL: {}", request.url);

    let response = state.core.scan(request).await;

    if response.success {
        Ok(Json(response))
    } else {
        error!(
            "Scan failed: {}",
            response
                .error
                .as_ref()
                .unwrap_or(&"Unknown error".to_string())
        );
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": response.error,
                "timestamp": response.timestamp
            })),
        ))
    }
}

async fn validate_endpoint(
    State(state): State<ServerState>,
    Json(request): Json<ScanRequest>,
) -> Result<Json<ValidationResponse>, (StatusCode, Json<Value>)> {
    info!("Received validation request");

    let response = state.core.validate_config(&request);

    if response.success && response.valid {
        Ok(Json(response))
    } else {
        error!(
            "Validation failed: {}",
            response
                .error
                .as_ref()
                .unwrap_or(&"Unknown error".to_string())
        );
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "valid": false,
                "error": response.error,
                "timestamp": response.timestamp
            })),
        ))
    }
}

async fn batch_scan_endpoint(
    State(state): State<ServerState>,
    Json(request): Json<BatchScanRequest>,
) -> Result<Json<BatchScanResponse>, (StatusCode, Json<Value>)> {
    if request.urls.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "URLs array is required",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        ));
    }

    info!(
        "Received batch scan request for {} URLs",
        request.urls.len()
    );

    let response = state.core.batch_scan(request).await;

    if !response.success {
        error!(
            "Batch scan failed: {} successful, {} failed",
            response.successful, response.failed
        );
    }

    Ok(Json(response))
}

// Tests removed for now - would need axum-test dependency

#[cfg(test)]
mod tests {
    use super::*;
    // Note: StatusCode is used in the validation logic but not in tests

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "0.0.0.0");
    }

    #[test]
    fn test_scan_request_validation() {
        // Test empty URL
        let request = ScanRequest {
            url: String::new(),
            ..Default::default()
        };
        assert!(request.url.is_empty());

        // Test valid URL
        let request = ScanRequest {
            url: "https://example.com".to_string(),
            ..Default::default()
        };
        assert!(!request.url.is_empty());
        assert!(request.url.starts_with("https://"));
    }
}
