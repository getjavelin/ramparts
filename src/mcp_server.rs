use std::sync::Arc;

use crate::core::{MCPScannerCore, ScanRequest};
use crate::scanner::MCPScanner;
use crate::types::ScanConfigBuilder;
use rmcp::handler::server::tool::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::{
        io::stdio,
        sse_server::SseServer,
        streamable_http_server::{
            session::local::LocalSessionManager, tower::StreamableHttpService,
            StreamableHttpServerConfig,
        },
    },
    ErrorData, ServiceExt,
};
use serde::Deserialize;
use std::future::Future;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct ScanParams {
    url: String,
    #[serde(default)]
    detailed: bool,
    #[serde(default)]
    auth_headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default, rename = "httpTimeout")]
    http_timeout: Option<u64>,
    /// If true, do not call the LLM; return prompts instead
    #[serde(default, rename = "returnPrompts")]
    return_prompts: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct ScanConfigParams {
    #[serde(default)]
    detailed: Option<bool>,
    #[serde(default)]
    auth_headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default, rename = "httpTimeout")]
    http_timeout: Option<u64>,
    /// If true, do not call the LLM; return prompts instead
    #[serde(default, rename = "returnPrompts")]
    return_prompts: Option<bool>,
}

/// Minimal MCP server that exposes basic tools. Extend to integrate scanner endpoints as tools.
#[derive(Clone)]
pub struct RampartsMcpServer {
    tool_router: ToolRouter<Self>,
    counter: Arc<Mutex<i32>>,
    core: Arc<MCPScannerCore>,
}

#[tool_router]
impl RampartsMcpServer {
    pub fn new() -> Self {
        let core = MCPScannerCore::new().expect("core init");
        Self {
            tool_router: Self::tool_router(),
            counter: Arc::new(Mutex::new(0)),
            core: Arc::new(core),
        }
    }

    #[tool(description = "Healthcheck for the Ramparts MCP server")]
    async fn health(&self) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    #[tool(description = "Increment an internal counter and return its value")]
    async fn increment_counter(&self) -> Result<CallToolResult, ErrorData> {
        let mut guard = self.counter.lock().await;
        *guard += 1;
        Ok(CallToolResult::success(vec![Content::text(
            guard.to_string(),
        )]))
    }

    #[tool(
        name = "scan",
        description = "Scan an MCP server URL and return security findings as JSON"
    )]
    async fn scan(&self, params: Parameters<ScanParams>) -> Result<CallToolResult, ErrorData> {
        let p = params.0;
        let request = ScanRequest {
            url: p.url,
            timeout: p.timeout,
            http_timeout: p.http_timeout,
            detailed: Some(p.detailed),
            format: p.format,
            auth_headers: p.auth_headers,
            // Default to returning prompts (no LLM call) for MCP tool flow
            return_prompts: Some(p.return_prompts.unwrap_or(true)),
        };

        let resp = self.core.scan(request).await;
        if let Some(result) = resp.result {
            let json = serde_json::to_string(&result)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        } else {
            Err(ErrorData::invalid_request(
                resp.error.unwrap_or_else(|| "scan failed".to_string()),
                None,
            ))
        }
    }

    // New: scan-config tool
    #[tool(
        name = "scan-config",
        description = "Scan MCP servers from IDE configuration files and return results as JSON"
    )]
    async fn scan_config(
        &self,
        params: Parameters<ScanConfigParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let p = params.0;
        let mut builder = ScanConfigBuilder::new();
        if let Some(d) = p.detailed {
            builder = builder.detailed(d);
        }
        if let Some(fmt) = p.format {
            builder = builder.format(fmt);
        }
        if let Some(t) = p.timeout {
            builder = builder.timeout(t);
        }
        if let Some(ht) = p.http_timeout {
            builder = builder.http_timeout(ht);
        }
        if let Some(headers) = p.auth_headers {
            builder = builder.auth_headers(Some(headers));
        }
        // Default to returning prompts (no LLM call) for MCP tool flow
        builder = builder.return_prompts(p.return_prompts.unwrap_or(true));

        let options = builder.build();
        let scanner = MCPScanner::with_timeout(options.http_timeout)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let results = scanner
            .scan_config_by_ide(options)
            .await
            .map_err(|e| ErrorData::invalid_request(e.to_string(), None))?;
        let json = serde_json::to_string(&results)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for RampartsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Ramparts MCP server".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Run the MCP server over stdio transport.
pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    let handler = RampartsMcpServer::new();
    let service = handler.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Run the MCP server over SSE transport (HTTP SSE endpoint)
pub async fn run_sse_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let bind: std::net::SocketAddr = format!("{host}:{port}").parse()?;
    let sse = SseServer::serve(bind).await?;
    let _ct = sse.with_service(RampartsMcpServer::new);
    // Keep the server alive until process exit
    futures_util::future::pending::<()>().await;
    Ok(())
}

/// Run the MCP server over streamable HTTP transport using a Tower service (Axum-compatible)
pub async fn run_streamable_http_server(
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum::routing::any_service;
    use axum::Router;
    use tower::ServiceBuilder;

    let bind: std::net::SocketAddr = format!("{host}:{port}").parse()?;
    let service: StreamableHttpService<RampartsMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(RampartsMcpServer::new()),
            std::sync::Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default(),
        );

    let app = Router::new().route("/", any_service(ServiceBuilder::new().service(service)));
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
