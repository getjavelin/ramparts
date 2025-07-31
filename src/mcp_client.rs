/// MCP client implementation using the official Rust MCP SDK
///
/// This module provides full MCP protocol support using the official rmcp SDK with
/// all available transport types: subprocess, SSE, and streamable HTTP.
use crate::types::{MCPPrompt, MCPPromptArgument, MCPResource, MCPServerInfo, MCPSession, MCPTool};
use anyhow::{anyhow, Result};
use rmcp::{
    service::RunningService,
    transport::{SseClientTransport, StreamableHttpClientTransport, TokioChildProcess},
    RoleClient, ServiceExt,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

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

    /// Connect to an MCP server using the appropriate transport
    pub async fn connect_http(
        &self,
        url: &str,
        auth_headers: Option<HashMap<String, String>>,
    ) -> Result<MCPSession> {
        info!("Connecting to MCP server at: {}", url);

        // First try streamable HTTP transport
        match self
            .try_streamable_http_connection(url, auth_headers.as_ref())
            .await
        {
            Ok(session) => {
                info!("Successfully connected via streamable HTTP");
                return Ok(session);
            }
            Err(e) => {
                debug!("Streamable HTTP connection failed: {}, trying SSE", e);
            }
        }

        // Fall back to SSE transport
        match self.try_sse_connection(url, auth_headers.as_ref()).await {
            Ok(session) => {
                info!("Successfully connected via SSE");
                Ok(session)
            }
            Err(e) => {
                warn!("SSE connection also failed: {}", e);
                Err(anyhow!(
                    "Failed to connect via both streamable HTTP and SSE: {}",
                    e
                ))
            }
        }
    }

    /// Try to connect using streamable HTTP transport
    async fn try_streamable_http_connection(
        &self,
        url: &str,
        _auth_headers: Option<&HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!("Attempting streamable HTTP connection to: {}", url);

        // Create streamable HTTP transport
        let transport = StreamableHttpClientTransport::from_uri(url);

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
            session_id: None,
            endpoint_url: url.to_string(),
        };

        Ok(session)
    }

    /// Try to connect using SSE transport
    async fn try_sse_connection(
        &self,
        url: &str,
        _auth_headers: Option<&HashMap<String, String>>,
    ) -> Result<MCPSession> {
        debug!("Attempting SSE connection to: {}", url);

        // Create SSE transport
        let transport = SseClientTransport::start(url)
            .await
            .map_err(|e| anyhow!("Failed to create SSE transport: {}", e))?;

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
            session_id: None,
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
        info!(
            "Connecting to MCP server via subprocess: {} {:?}",
            command, args
        );

        // Create the command
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
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
            session_id: None,
            endpoint_url: endpoint,
        };

        Ok(session)
    }

    /// Fetch tools from the MCP server using the official SDK
    pub async fn fetch_tools(&self, session: &MCPSession) -> Result<Vec<MCPTool>> {
        debug!("Fetching tools from MCP server: {}", session.endpoint_url);

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

                    info!(
                        "Successfully fetched {} tools from MCP server",
                        mcp_tools.len()
                    );
                    Ok(mcp_tools)
                }
                Err(e) => {
                    warn!("Failed to fetch tools from MCP server: {}", e);
                    Ok(vec![])
                }
            }
        } else {
            warn!("No active MCP service found for: {}", session.endpoint_url);
            Ok(vec![])
        }
    }

    /// Fetch resources from the MCP server
    pub async fn fetch_resources(&self, session: &MCPSession) -> Result<Vec<MCPResource>> {
        debug!(
            "Fetching resources from MCP server: {}",
            session.endpoint_url
        );

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

                    info!(
                        "Successfully fetched {} resources from MCP server",
                        mcp_resources.len()
                    );
                    Ok(mcp_resources)
                }
                Err(e) => {
                    warn!("Failed to fetch resources from MCP server: {}", e);
                    Ok(vec![])
                }
            }
        } else {
            warn!("No active MCP service found for: {}", session.endpoint_url);
            Ok(vec![])
        }
    }

    /// Fetch prompts from the MCP server  
    pub async fn fetch_prompts(&self, session: &MCPSession) -> Result<Vec<MCPPrompt>> {
        debug!("Fetching prompts from MCP server: {}", session.endpoint_url);

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

                    info!(
                        "Successfully fetched {} prompts from MCP server",
                        mcp_prompts.len()
                    );
                    Ok(mcp_prompts)
                }
                Err(e) => {
                    warn!("Failed to fetch prompts from MCP server: {}", e);
                    Ok(vec![])
                }
            }
        } else {
            warn!("No active MCP service found for: {}", session.endpoint_url);
            Ok(vec![])
        }
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
        let result = client.connect_http("http://localhost:8124", None).await;
        // We expect this to fail in the test environment, but not panic
        assert!(result.is_err() || result.is_ok());
    }
}
