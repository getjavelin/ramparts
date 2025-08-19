use ramparts_common::{anyhow::Result, tracing::{info, debug, warn, error}};
use crate::{JavelinClient, ProxyConfig, get_license_status, GuardedMcpServer};
use axum::{
    extract::{State, Path},
    http::{StatusCode, HeaderMap},
    response::Json,
    routing::{get, post, any_service},
    Router,
};
use rmcp::{
    transport::{
        StreamableHttpServerConfig, StreamableHttpService, streamable_http_server::session::never::NeverSessionManager,
    },
};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

/// MCP Proxy server implementation using Grafbase Nexus patterns
pub struct MCPProxy {
    config: ProxyConfig,
    mcp_server: GuardedMcpServer,
}

/// Proxy state shared across handlers
#[derive(Clone)]
pub struct ProxyState {
    javelin_client: Arc<JavelinClient>,
    config: ProxyConfig,
}

impl MCPProxy {
    pub fn new(listen_address: String) -> Result<Self> {
        // Load configuration from environment
        let mut config = ProxyConfig::from_env()?;
        config.listen_address = listen_address;
        config.validate()?;

        // Initialize Javelin client with configuration
        let javelin_client = Arc::new(JavelinClient::with_config(
            config.javelin.api_key.clone(),
            config.javelin.base_url.clone(),
            config.javelin.timeout_seconds,
        ));

        // Create the MCP server with Javelin integration
        let mcp_server = GuardedMcpServer::new(config.clone(), javelin_client);

        Ok(Self {
            config,
            mcp_server,
        })
    }

    pub fn with_config(config: ProxyConfig) -> Result<Self> {
        config.validate()?;

        let javelin_client = Arc::new(JavelinClient::with_config(
            config.javelin.api_key.clone(),
            config.javelin.base_url.clone(),
            config.javelin.timeout_seconds,
        ));

        // Create the MCP server with Javelin integration
        let mcp_server = GuardedMcpServer::new(config.clone(), javelin_client);

        Ok(Self {
            config,
            mcp_server,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting MCP proxy on {} (Grafbase Nexus pattern)", self.config.listen_address);

        // Create the MCP service using Grafbase Nexus patterns
        let mcp_service = StreamableHttpService::new(
            {
                let server = self.mcp_server.clone();
                move || Ok(server.clone())
            },
            Arc::new(NeverSessionManager::default()),
            StreamableHttpServerConfig {
                sse_keep_alive: Some(Duration::from_secs(5)),
                stateful_mode: false,
            },
        );

        // Create shared state for additional endpoints
        let state = ProxyState {
            javelin_client: Arc::new(JavelinClient::with_config(
                self.config.javelin.api_key.clone(),
                self.config.javelin.base_url.clone(),
                self.config.javelin.timeout_seconds,
            )),
            config: self.config.clone(),
        };

        // Build the router with both MCP and management endpoints
        let app = Router::new()
            // Management endpoints
            .route("/", get(health_check))
            .route("/health", get(health_check))
            .route("/license", get(license_status))
            .route("/validate", post(validate_request))
            // Legacy proxy endpoint for backward compatibility
            .route("/proxy/:target", post(proxy_mcp_request))
            // MCP endpoint using Grafbase Nexus patterns
            .route("/mcp", any_service(mcp_service))
            .layer(CorsLayer::permissive())
            .with_state(state);

        // Parse the listen address
        let listener = TcpListener::bind(&self.config.listen_address).await
            .map_err(|e| ramparts_common::anyhow::anyhow!("Failed to bind to {}: {}", self.config.listen_address, e))?;

        info!("MCP proxy listening on {} with endpoints:", self.config.listen_address);
        info!("  - /mcp (MCP protocol with Javelin Guardrails)");
        info!("  - /health (Health check)");
        info!("  - /license (License status)");
        info!("  - /validate (Request validation)");
        info!("  - /proxy/:target (Legacy proxy endpoint)");

        // Start the server
        axum::serve(listener, app).await
            .map_err(|e| ramparts_common::anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

/// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "ramparts-proxy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// License status endpoint
async fn license_status() -> Json<Value> {
    let license_status = get_license_status().unwrap_or_else(|e| format!("Error: {}", e));

    Json(json!({
        "license": {
            "status": license_status,
            "component": "ramparts-proxy",
            "license_type": "Javelin Proprietary License",
            "requires_api_key": true,
            "contact": "legal@getjavelin.com"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Validate a request against Javelin Guardrails
async fn validate_request(
    State(state): State<ProxyState>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Validating request: {}", serde_json::to_string_pretty(&request).unwrap_or_default());

    match state.javelin_client.validate_request(&request).await {
        Ok(is_valid) => {
            let response = json!({
                "valid": is_valid,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "request_id": uuid::Uuid::new_v4().to_string()
            });
            Ok(Json(response))
        }
        Err(e) => {
            error!("Validation error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Proxy MCP requests through Javelin Guardrails
async fn proxy_mcp_request(
    State(state): State<ProxyState>,
    Path(target): Path<String>,
    _headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Proxying MCP request to target: {}", target);
    debug!("Request: {}", serde_json::to_string_pretty(&request).unwrap_or_default());

    // First, validate the request with Javelin Guardrails
    match state.javelin_client.validate_request(&request).await {
        Ok(true) => {
            info!("Request validated successfully, forwarding to target");
            // TODO: Forward the request to the actual MCP server
            // For now, return a mock response
            let response = json!({
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "result": {
                    "status": "proxied",
                    "target": target,
                    "validated": true,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            });
            Ok(Json(response))
        }
        Ok(false) => {
            warn!("Request failed Javelin Guardrails validation");
            let error_response = json!({
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "error": {
                    "code": -32600,
                    "message": "Request blocked by Javelin Guardrails",
                    "data": {
                        "reason": "Security policy violation",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }
                }
            });
            Ok(Json(error_response))
        }
        Err(e) => {
            error!("Guardrails validation error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
