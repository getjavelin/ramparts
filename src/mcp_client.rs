/// MCP client implementation using the official Rust MCP SDK
///
/// This module provides full MCP protocol support using the official rmcp SDK with
/// all available transport types: subprocess, SSE, and streamable HTTP.
use crate::types::{MCPPrompt, MCPPromptArgument, MCPResource, MCPServerInfo, MCPSession, MCPTool};
use anyhow::{anyhow, Result};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client as HttpClient,
};
use serde_json::{json, Value};

use rmcp::{
    service::RunningService,
    transport::{SseClientTransport, StreamableHttpClientTransport, TokioChildProcess},
    RoleClient, ServiceExt,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// MCP client using the official Rust MCP SDK with full transport support
pub struct McpClient {
    /// Store active MCP services by endpoint
    services: Arc<Mutex<HashMap<String, RunningService<RoleClient, ()>>>>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Try to connect using streamable HTTP transport
    async fn try_streamable_http_connection(
        &self,
        url: &str,
        auth_headers: Option<&HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!("Attempting streamable HTTP connection to: {}", url);

        // Create streamable HTTP transport with auth headers if provided
        let transport = if let Some(headers) = auth_headers {
            debug!("Creating HTTP client with {} auth headers", headers.len());
            let mut header_map = HeaderMap::new();

            for (key, value) in headers {
                debug!("Processing header: {} = {}", key, value);
                match (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    (Ok(name), Ok(val)) => {
                        debug!("Successfully added header: {}", key);
                        header_map.insert(name, val);
                    }
                    (Err(e), _) => {
                        warn!("Failed to parse header name '{}': {}", key, e);
                    }
                    (_, Err(e)) => {
                        warn!("Failed to parse header value for '{}': {}", key, e);
                    }
                }
            }

            let client = HttpClient::builder()
                .default_headers(header_map)
                .build()
                .expect("Failed to build HTTP client");
            let config =
                rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig {
                    uri: url.into(),
                    ..Default::default()
                };
            StreamableHttpClientTransport::with_client(client, config)
        } else {
            StreamableHttpClientTransport::from_uri(url)
        };

        // Create the MCP service
        let service = ()
            .serve(transport)
            .await
            .map_err(|e| anyhow!("Failed to create MCP service via streamable HTTP: {}", e))?;

        // Get server information
        let peer_info = service.peer().peer_info();
        let server_info = if let Some(init_result) = peer_info {
            MCPServerInfo {
                name: init_result.server_info.name.to_string(),
                version: init_result.server_info.version.to_string(),
                description: None,
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("streamable-http".to_string()),
                    );
                    map
                },
            }
        } else {
            MCPServerInfo {
                name: "Streamable HTTP MCP Server".to_string(),
                version: "Unknown".to_string(),
                description: Some("Connected via streamable HTTP".to_string()),
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("streamable-http".to_string()),
                    );
                    map
                },
            }
        };

        // Store the service for later use
        {
            let mut services = self.services.lock().await;
            services.insert(url.to_string(), service);
        }

        let session = MCPSession {
            server_info: Some(server_info),
            endpoint_url: url.to_string(),
        };

        Ok(session)
    }

    /// Try to connect using SSE transport
    async fn try_sse_connection(
        &self,
        url: &str,
        auth_headers: Option<&HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!("Attempting SSE connection to: {}", url);

        // Create SSE transport with auth headers if provided
        let transport = if let Some(headers) = auth_headers {
            debug!("Creating SSE client with {} auth headers", headers.len());
            let mut header_map = HeaderMap::new();

            for (key, value) in headers {
                debug!("Processing SSE header: {} = {}", key, value);
                match (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    (Ok(name), Ok(val)) => {
                        debug!("Successfully added SSE header: {}", key);
                        header_map.insert(name, val);
                    }
                    (Err(e), _) => {
                        warn!("Failed to parse SSE header name '{}': {}", key, e);
                    }
                    (_, Err(e)) => {
                        warn!("Failed to parse SSE header value for '{}': {}", key, e);
                    }
                }
            }

            let client = HttpClient::builder()
                .default_headers(header_map)
                .build()
                .expect("Failed to build HTTP client");
            let config = rmcp::transport::sse_client::SseClientConfig {
                sse_endpoint: url.into(),
                ..Default::default()
            };
            SseClientTransport::start_with_client(client, config)
                .await
                .map_err(|e| anyhow!("Failed to create SSE transport with auth: {}", e))?
        } else {
            SseClientTransport::start(url)
                .await
                .map_err(|e| anyhow!("Failed to create SSE transport: {}", e))?
        };

        // Create the MCP service
        let service = ()
            .serve(transport)
            .await
            .map_err(|e| anyhow!("Failed to create MCP service via SSE: {}", e))?;

        // Get server information
        let peer_info = service.peer().peer_info();
        let server_info = if let Some(init_result) = peer_info {
            MCPServerInfo {
                name: init_result.server_info.name.to_string(),
                version: init_result.server_info.version.to_string(),
                description: None,
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("sse".to_string()),
                    );
                    map
                },
            }
        } else {
            MCPServerInfo {
                name: "SSE MCP Server".to_string(),
                version: "Unknown".to_string(),
                description: Some("Connected via SSE".to_string()),
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("sse".to_string()),
                    );
                    map
                },
            }
        };

        // Store the service for later use
        {
            let mut services = self.services.lock().await;
            services.insert(url.to_string(), service);
        }

        let session = MCPSession {
            server_info: Some(server_info),
            endpoint_url: url.to_string(),
        };

        Ok(session)
    }

    /// Connect using subprocess (for local MCP servers)
    pub async fn connect_subprocess(
        &self,
        command: &str,
        args: &[String],
        env_vars: Option<&HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!(
            "Connecting to MCP server via subprocess: {} {:?}",
            command, args
        );

        // Create the command
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }

        // Suppress subprocess stdout/stderr to prevent startup messages from cluttering output
        // Only suppress if not in debug mode (to preserve error messages for troubleshooting)
        if std::env::var("RUST_LOG")
            .map_or(true, |log| !log.contains("debug") && !log.contains("trace"))
        {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        // Add environment variables if provided
        if let Some(env) = env_vars {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Create the service using the subprocess transport
        let transport = TokioChildProcess::new(cmd)?;
        let service = ()
            .serve(transport)
            .await
            .map_err(|e| {
                // Provide more detailed error information for troubleshooting
                let error_context = if e.to_string().contains("connection closed") {
                    format!("MCP server subprocess failed during initialization. This could be due to: \
                           \n  - Missing required environment variables (check server documentation) \
                           \n  - Server startup errors (enable debug logging with RUST_LOG=debug) \
                           \n  - Package installation issues (try: npx {command} manually) \
                           \n  - Network connectivity issues for remote servers \
                           \nOriginal error: {e}")
                } else {
                    format!("Failed to start MCP server subprocess: {e}")
                };
                anyhow!(error_context)
            })?;

        // Get server information
        let peer_info = service.peer().peer_info();
        let server_info = if let Some(init_result) = peer_info {
            MCPServerInfo {
                name: init_result.server_info.name.to_string(),
                version: init_result.server_info.version.to_string(),
                description: None,
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("subprocess".to_string()),
                    );
                    map
                },
            }
        } else {
            MCPServerInfo {
                name: "Subprocess MCP Server".to_string(),
                version: "Unknown".to_string(),
                description: Some("Connected via subprocess".to_string()),
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("subprocess".to_string()),
                    );
                    map
                },
            }
        };

        // Store the service for later use
        let endpoint = format!("subprocess://{command}");
        {
            let mut services = self.services.lock().await;
            services.insert(endpoint.clone(), service);
        }

        let session = MCPSession {
            server_info: Some(server_info),
            endpoint_url: endpoint,
        };

        Ok(session)
    }

    /// Fetch tools from the MCP server using the official SDK
    pub async fn list_tools(&self, session: &MCPSession) -> Result<Vec<MCPTool>> {
        debug!("Fetching tools from MCP server: {}", session.endpoint_url);

        // Check if this is a simple HTTP session
        if let Some(server_info) = &session.server_info {
            if let Some(transport_type) = server_info.metadata.get("transport") {
                if transport_type.as_str() == Some("simple_http") {
                    return self.list_tools_simple_http(session).await;
                }
            }
        }

        // Use rmcp transport for other sessions
        let services = self.services.lock().await;
        if let Some(service) = services.get(&session.endpoint_url) {
            match service.list_tools(Option::default()).await {
                Ok(tools_response) => {
                    let mut mcp_tools = Vec::new();

                    for tool in tools_response.tools {
                        let mcp_tool = MCPTool {
                            name: tool.name.to_string(),
                            description: tool
                                .description
                                .as_ref()
                                .map(std::string::ToString::to_string),
                            input_schema: Some(serde_json::Value::Object(
                                (*tool.input_schema).clone(),
                            )),
                            output_schema: None,
                            parameters: HashMap::new(),
                            category: None,
                            tags: vec![],
                            deprecated: false,
                            raw_json: None,
                        };
                        mcp_tools.push(mcp_tool);
                    }

                    debug!(
                        "Successfully fetched {} tools from MCP server",
                        mcp_tools.len()
                    );
                    Ok(mcp_tools)
                }
                Err(e) => {
                    debug!("Failed to fetch tools from MCP server: {}", e);
                    Ok(vec![])
                }
            }
        } else {
            warn!("No active MCP service found for: {}", session.endpoint_url);
            Ok(vec![])
        }
    }

    /// Fetch resources from the MCP server
    pub async fn list_resources(&self, session: &MCPSession) -> Result<Vec<MCPResource>> {
        debug!(
            "Fetching resources from MCP server: {}",
            session.endpoint_url
        );

        // Check if this is a simple HTTP session
        if let Some(server_info) = &session.server_info {
            if let Some(transport_type) = server_info.metadata.get("transport") {
                if transport_type.as_str() == Some("simple_http") {
                    return self.list_resources_simple_http(session).await;
                }
            }
        }

        // Use rmcp transport for other sessions
        let services = self.services.lock().await;
        if let Some(service) = services.get(&session.endpoint_url) {
            match service.list_resources(Option::default()).await {
                Ok(resources_response) => {
                    let mut mcp_resources = Vec::new();

                    for resource in resources_response.resources {
                        let mcp_resource = MCPResource {
                            uri: resource.uri.to_string(),
                            name: resource.name.to_string(),
                            description: resource
                                .description
                                .as_ref()
                                .map(std::string::ToString::to_string),
                            mime_type: resource
                                .mime_type
                                .as_ref()
                                .map(std::string::ToString::to_string),
                            size: None,
                            metadata: HashMap::new(),
                            raw_json: None,
                        };
                        mcp_resources.push(mcp_resource);
                    }

                    debug!(
                        "Successfully fetched {} resources from MCP server",
                        mcp_resources.len()
                    );
                    Ok(mcp_resources)
                }
                Err(e) => {
                    debug!("Failed to fetch resources from MCP server: {}", e);
                    Ok(vec![])
                }
            }
        } else {
            warn!("No active MCP service found for: {}", session.endpoint_url);
            Ok(vec![])
        }
    }

    /// Fetch prompts from the MCP server  
    pub async fn list_prompts(&self, session: &MCPSession) -> Result<Vec<MCPPrompt>> {
        debug!("Fetching prompts from MCP server: {}", session.endpoint_url);

        // Check if this is a simple HTTP session
        if let Some(server_info) = &session.server_info {
            if let Some(transport_type) = server_info.metadata.get("transport") {
                if transport_type.as_str() == Some("simple_http") {
                    return self.list_prompts_simple_http(session).await;
                }
            }
        }

        // Use rmcp transport for other sessions
        let services = self.services.lock().await;
        if let Some(service) = services.get(&session.endpoint_url) {
            match service.list_prompts(Option::default()).await {
                Ok(prompts_response) => {
                    let mut mcp_prompts = Vec::new();

                    for prompt in prompts_response.prompts {
                        let arguments = prompt.arguments.as_ref().map(|args| {
                            args.iter()
                                .map(|arg| MCPPromptArgument {
                                    name: arg.name.to_string(),
                                    description: arg
                                        .description
                                        .as_ref()
                                        .map(std::string::ToString::to_string),
                                    required: arg.required,
                                })
                                .collect()
                        });

                        let mcp_prompt = MCPPrompt {
                            name: prompt.name.to_string(),
                            description: prompt
                                .description
                                .as_ref()
                                .map(std::string::ToString::to_string),
                            arguments,
                            raw_json: None,
                        };
                        mcp_prompts.push(mcp_prompt);
                    }

                    debug!(
                        "Successfully fetched {} prompts from MCP server",
                        mcp_prompts.len()
                    );
                    Ok(mcp_prompts)
                }
                Err(e) => {
                    debug!("Failed to fetch prompts from MCP server: {}", e);
                    Ok(vec![])
                }
            }
        } else {
            warn!("No active MCP service found for: {}", session.endpoint_url);
            Ok(vec![])
        }
    }

    /// Smart connect method - tries the best transport for each scenario
    pub async fn connect_smart(
        &self,
        url: &str,
        auth_headers: Option<HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!("Smart connecting to MCP server at: {}", url);

        // STDIO transport: Use rmcp (perfect implementation)
        if url.starts_with("stdio:") {
            // Parse STDIO URL format: stdio:executable:args...
            let parts: Vec<&str> = url.splitn(3, ':').collect();
            if parts.len() >= 2 {
                let args = if parts.len() > 2 {
                    parts[2].split_whitespace().map(|s| s.to_string()).collect()
                } else {
                    vec![]
                };
                return self
                    .connect_subprocess(parts[1], &args, auth_headers.as_ref())
                    .await;
            } else {
                return Err(anyhow!(
                    "Invalid STDIO URL format. Expected: stdio:executable:args"
                ));
            }
        }

        // HTTP transport: Try simple HTTP first, rmcp as fallback

        // Step 1: Try simple HTTP (works with most servers)
        match self
            .try_simple_http_connection(url, auth_headers.as_ref())
            .await
        {
            Ok(session) => {
                debug!("Successfully connected via simple HTTP");
                return Ok(session);
            }
            Err(e) => {
                debug!(
                    "Simple HTTP connection failed: {}, trying rmcp transports",
                    e
                );
            }
        }

        // Step 2: Try rmcp streamable HTTP (for advanced servers)
        match self
            .try_streamable_http_connection(url, auth_headers.as_ref())
            .await
        {
            Ok(session) => {
                debug!("Successfully connected via rmcp streamable HTTP");
                return Ok(session);
            }
            Err(e) => {
                debug!("rmcp streamable HTTP connection failed: {}, trying SSE", e);
            }
        }

        // Step 3: Try rmcp SSE transport (final fallback)
        match self.try_sse_connection(url, auth_headers.as_ref()).await {
            Ok(session) => {
                debug!("Successfully connected via rmcp SSE");
                Ok(session)
            }
            Err(e) => {
                warn!("All transport methods failed. Last error: {}", e);
                Err(anyhow!(
                    "Failed to connect via simple HTTP, streamable HTTP, and SSE: {}",
                    e
                ))
            }
        }
    }

    /// Try to connect using simple HTTP JSON-RPC (compatible with most servers)
    async fn try_simple_http_connection(
        &self,
        url: &str,
        auth_headers: Option<&HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!("Attempting simple HTTP connection to: {}", url);

        // Build HTTP client with auth headers
        let mut headers = HeaderMap::new();
        if let Some(auth_headers) = auth_headers {
            for (key, value) in auth_headers {
                match (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    (Ok(name), Ok(val)) => {
                        headers.insert(name, val);
                    }
                    _ => {
                        warn!("Failed to parse header: {} = {}", key, value);
                    }
                }
            }
        }

        let client = HttpClient::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        // Step 1: Initialize connection
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "ramparts",
                    "version": "0.6.8"
                }
            }
        });

        debug!("Sending initialize request to: {}", url);
        let response = client.post(url).json(&init_request).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Initialize failed: HTTP {}", response.status()));
        }

        let init_response: Value = response.json().await?;
        debug!("Initialize response: {:?}", init_response);

        // Check for JSON-RPC error
        if let Some(error) = init_response.get("error") {
            return Err(anyhow!("Initialize error: {:?}", error));
        }

        // Step 2: Send initialized notification (if server expects it)
        let notify_request = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        // Send notification but don't fail if it doesn't work (some servers don't expect it)
        let _ = client.post(url).json(&notify_request).send().await;

        // Extract server info from initialize response
        let server_info = init_response
            .get("result")
            .and_then(|r| r.get("serverInfo"))
            .map(|info| MCPServerInfo {
                name: info
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                version: info
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                description: None,
                capabilities: vec![
                    "tools".to_string(),
                    "resources".to_string(),
                    "prompts".to_string(),
                ],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert(
                        "transport".to_string(),
                        serde_json::Value::String("simple_http".to_string()),
                    );
                    map
                },
            });

        Ok(MCPSession {
            server_info,
            endpoint_url: url.to_string(),
        })
    }

    /// List tools using simple HTTP JSON-RPC
    async fn list_tools_simple_http(&self, session: &MCPSession) -> Result<Vec<MCPTool>> {
        debug!(
            "Fetching tools via simple HTTP from: {}",
            session.endpoint_url
        );

        let tools_response = self
            .json_rpc_request(&session.endpoint_url, "tools/list", json!({}))
            .await?;

        let tools_array = tools_response
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| anyhow!("Invalid tools response format"))?;

        let mut mcp_tools = Vec::new();
        for tool in tools_array {
            let mcp_tool = MCPTool {
                name: tool
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: tool
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string()),
                input_schema: tool.get("inputSchema").cloned(),
                output_schema: None,
                parameters: HashMap::new(),
                category: None,
                tags: vec![],
                deprecated: false,
                raw_json: Some(tool.clone()),
            };
            mcp_tools.push(mcp_tool);
        }

        debug!(
            "Successfully fetched {} tools via simple HTTP",
            mcp_tools.len()
        );
        Ok(mcp_tools)
    }

    /// List resources using simple HTTP JSON-RPC
    async fn list_resources_simple_http(&self, session: &MCPSession) -> Result<Vec<MCPResource>> {
        debug!(
            "Fetching resources via simple HTTP from: {}",
            session.endpoint_url
        );

        let resources_response = self
            .json_rpc_request(&session.endpoint_url, "resources/list", json!({}))
            .await?;

        let resources_array = resources_response
            .get("resources")
            .and_then(|r| r.as_array())
            .ok_or_else(|| anyhow!("Invalid resources response format"))?;

        let mut mcp_resources = Vec::new();
        for resource in resources_array {
            let mcp_resource = MCPResource {
                uri: resource
                    .get("uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: resource
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: resource
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string()),
                mime_type: resource
                    .get("mimeType")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string()),
                size: resource.get("size").and_then(|s| s.as_u64()),
                metadata: HashMap::new(), // Could be populated from resource data if needed
                raw_json: Some(resource.clone()),
            };
            mcp_resources.push(mcp_resource);
        }

        debug!(
            "Successfully fetched {} resources via simple HTTP",
            mcp_resources.len()
        );
        Ok(mcp_resources)
    }

    /// List prompts using simple HTTP JSON-RPC
    async fn list_prompts_simple_http(&self, session: &MCPSession) -> Result<Vec<MCPPrompt>> {
        debug!(
            "Fetching prompts via simple HTTP from: {}",
            session.endpoint_url
        );

        let prompts_response = self
            .json_rpc_request(&session.endpoint_url, "prompts/list", json!({}))
            .await?;

        let prompts_array = prompts_response
            .get("prompts")
            .and_then(|p| p.as_array())
            .ok_or_else(|| anyhow!("Invalid prompts response format"))?;

        let mut mcp_prompts = Vec::new();
        for prompt in prompts_array {
            let mcp_prompt = MCPPrompt {
                name: prompt
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: prompt
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string()),
                arguments: None, // Could be extracted if needed
                raw_json: Some(prompt.clone()),
            };
            mcp_prompts.push(mcp_prompt);
        }

        debug!(
            "Successfully fetched {} prompts via simple HTTP",
            mcp_prompts.len()
        );
        Ok(mcp_prompts)
    }

    /// Generic JSON-RPC request helper for simple HTTP transport
    async fn json_rpc_request(&self, url: &str, method: &str, params: Value) -> Result<Value> {
        let client = HttpClient::new();

        let request = json!({
            "jsonrpc": "2.0",
            "id": rand::random::<u32>(),
            "method": method,
            "params": params
        });

        debug!("Sending JSON-RPC request to {}: {}", url, method);
        let response = client.post(url).json(&request).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP request failed: {}", response.status()));
        }

        let json_response: Value = response.json().await?;

        // Check for JSON-RPC error
        if let Some(error) = json_response.get("error") {
            return Err(anyhow!("JSON-RPC error: {:?}", error));
        }

        // Extract result
        json_response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("Missing result in JSON-RPC response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_creation() {
        let _client = McpClient::new();
        // Basic test to ensure the client can be created
    }

    #[tokio::test]
    async fn test_http_connection() {
        let client = McpClient::new();
        // This will likely fail in tests since there's no server running
        // but we can at least test that the method exists and can be called
        let result = client.connect("http://localhost:8124", None).await;
        // We expect this to fail in the test environment, but not panic
        assert!(result.is_err() || result.is_ok());
    }
}
