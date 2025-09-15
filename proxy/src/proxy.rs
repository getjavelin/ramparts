use crate::{get_license_status, GuardedMcpServer, JavelinClient, ProxyConfig, ValidationService};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{any_service, get, post},
    Router,
};
use ramparts_common::{
    anyhow::Result,
    tracing::{debug, error, info},
};
use rmcp::transport::{
    streamable_http_server::session::never::NeverSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

/// Security-first AI Gateway for MCP - competitive alternative to Nexus, LiteLLM, and Cloudflare AI Gateway
pub struct MCPProxy {
    config: ProxyConfig,
    mcp_server: GuardedMcpServer,
}

/// Proxy state shared across handlers
#[derive(Clone)]
pub struct ProxyState {
    validation_service: Arc<ValidationService>,
}

impl MCPProxy {
    pub fn new(listen_address: String) -> Result<Self> {
        // Load configuration from environment
        let mut config = ProxyConfig::from_env()?;
        config.listen_address = listen_address;
        config.validate()?;

        // Initialize Javelin client with configuration
        let javelin_client = Arc::new(JavelinClient::with_behavior(
            config.javelin.api_key.clone(),
            config.javelin.base_url.clone(),
            config.javelin.timeout_seconds,
            &config.behavior,
        ));

        // Create the MCP server with Javelin integration
        let mcp_server = GuardedMcpServer::new(config.clone(), javelin_client);

        Ok(Self { config, mcp_server })
    }

    pub fn with_config(config: ProxyConfig) -> Result<Self> {
        config.validate()?;

        let javelin_client = Arc::new(JavelinClient::with_behavior(
            config.javelin.api_key.clone(),
            config.javelin.base_url.clone(),
            config.javelin.timeout_seconds,
            &config.behavior,
        ));

        // Create the MCP server with Javelin integration
        let mcp_server = GuardedMcpServer::new(config.clone(), javelin_client);

        Ok(Self { config, mcp_server })
    }

    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting Ramparts AI Gateway on {} (security-first MCP proxy)",
            self.config.listen_address
        );

        // Create the MCP service with enterprise security
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

        // Create shared state with unified validation service
        let validation_service = Arc::new(ValidationService::new(
            self.mcp_server.get_javelin_client(),
            self.config.clone(),
        ));

        let state = ProxyState { validation_service };

        // Build the router with both MCP and management endpoints
        let app = Router::new()
            // Management endpoints
            .route("/", get(health_check))
            .route("/health", get(health_check))
            .route("/license", get(license_status))
            .route("/validate", post(validate_request))
            // MCP endpoint with enterprise security validation
            .route("/mcp", any_service(mcp_service))
            .layer(CorsLayer::permissive())
            .with_state(state);

        // Parse the listen address
        let listener = TcpListener::bind(&self.config.listen_address)
            .await
            .map_err(|e| {
                ramparts_common::anyhow::anyhow!(
                    "Failed to bind to {}: {}",
                    self.config.listen_address,
                    e
                )
            })?;

        info!(
            "Ramparts AI Gateway listening on {} with endpoints:",
            self.config.listen_address
        );
        info!("  - /mcp (Secure MCP protocol with Javelin Guardrails)");
        info!("  - /health (Health check)");
        info!("  - /license (License status)");
        info!("  - /validate (Enterprise request validation)");

        // Start the server
        axum::serve(listener, app)
            .await
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
    // Redact sensitive values before logging
    let redacted = crate::logging::sanitize_json_for_log(&request);
    debug!(
        "Validating request: {}",
        serde_json::to_string_pretty(&redacted).unwrap_or_default()
    );

    match state.validation_service.validate_request(&request).await {
        Ok(result) => {
            let response = json!({
                "valid": result.allowed,
                "reason": result.reason,
                "confidence": result.confidence,
                "request_id": result.request_id,
                "timestamp": result.timestamp
            });
            Ok(Json(response))
        }
        Err(e) => {
            error!("Validation error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
