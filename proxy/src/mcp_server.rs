use ramparts_common::{anyhow::Result, tracing::{info, debug, warn, error}};
use crate::{JavelinClient, ProxyConfig};
use rmcp::{
    RoleServer, ServerHandler,
    model::{
        CallToolRequestMethod, CallToolRequestParam, CallToolResult, Content, ErrorData,
        GetPromptRequestParam, GetPromptResult, Implementation, ListPromptsResult, ListResourcesResult,
        ListToolsResult, PaginatedRequestParam, ReadResourceRequestParam, ReadResourceResult, ServerCapabilities,
        ServerInfo, Tool, ProtocolVersion,
    },
    service::RequestContext,
};
use serde_json::{json, Value};
use std::{sync::Arc, collections::HashMap};

/// MCP Server with Javelin Guardrails integration
#[derive(Clone)]
pub struct GuardedMcpServer {
    shared: Arc<GuardedMcpServerInner>,
}

struct GuardedMcpServerInner {
    info: ServerInfo,
    javelin_client: Arc<JavelinClient>,
    config: ProxyConfig,
    // Target MCP servers to proxy to (planned for future use)
    #[allow(dead_code)]
    target_servers: HashMap<String, String>, // server_name -> endpoint_url
    // Available tools (aggregated from target servers)
    tools: Vec<Tool>,
}

impl GuardedMcpServer {
    pub fn new(config: ProxyConfig, javelin_client: Arc<JavelinClient>) -> Self {
        let server_info = Implementation {
            name: "Ramparts Proxy with Javelin Guardrails".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let info = ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .build(),
            server_info,
            instructions: Some(generate_proxy_instructions()),
        };

        // Initialize with basic validation tools
        let tools = vec![
            create_validate_tool(),
            create_proxy_tool(),
        ];

        let inner = GuardedMcpServerInner {
            info,
            javelin_client,
            config,
            target_servers: HashMap::new(),
            tools,
        };

        Self {
            shared: Arc::new(inner),
        }
    }

    /// Add a target MCP server to proxy to
    pub fn add_target_server(&mut self, name: String, endpoint_url: String) {
        // Note: This would require making the inner mutable in a real implementation
        // For now, we'll handle this through configuration
        debug!("Would add target server: {} -> {}", name, endpoint_url);
    }

    /// Get the Javelin client for reuse (eliminates duplicate client creation)
    pub fn get_javelin_client(&self) -> Arc<JavelinClient> {
        self.shared.javelin_client.clone()
    }

    /// Validate a request with Javelin Guardrails
    async fn validate_request(&self, request: &Value) -> Result<bool, ErrorData> {
        debug!("Validating request with Javelin Guardrails");
        
        match self.shared.javelin_client.validate_request(request).await {
            Ok(is_valid) => {
                if is_valid {
                    info!("Request approved by Javelin Guardrails");
                } else {
                    warn!("Request blocked by Javelin Guardrails");
                }
                Ok(is_valid)
            }
            Err(e) => {
                error!("Javelin Guardrails validation error: {}", e);
                
                // Fail open/closed based on configuration
                if self.shared.config.javelin.fail_open {
                    warn!("Failing open due to validation error");
                    Ok(true)
                } else {
                    warn!("Failing closed due to validation error");
                    Ok(false)
                }
            }
        }
    }

    /// Proxy a request to a target MCP server
    async fn proxy_to_target(&self, target: &str, request: CallToolRequestParam) -> Result<CallToolResult, ErrorData> {
        debug!("Proxying request to target: {}", target);
        
        // TODO: Implement actual proxying to target MCP server
        // For now, return a mock response
        Ok(CallToolResult {
            content: vec![Content::text(format!(
                "Request proxied to {} with tool {} (validated by Javelin Guardrails)",
                target,
                request.name
            ))],
            is_error: None,
        })
    }
}

impl ServerHandler for GuardedMcpServer {
    fn get_info(&self) -> ServerInfo {
        self.shared.info.clone()
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParam>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        debug!("Listing available proxy tools");
        
        Ok(ListToolsResult {
            next_cursor: None,
            tools: self.shared.tools.clone(),
        })
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        info!("Processing tool call: {}", params.name);

        match params.name.as_ref() {
            "validate" => {
                debug!("Executing validation tool");
                
                // Extract request from arguments
                let request_value = params.arguments
                    .as_ref()
                    .and_then(|args| args.get("request"))
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'request' parameter", None))?;

                let is_valid = self.validate_request(request_value).await?;

                let content = Content::json(json!({
                    "valid": is_valid,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "validated_by": "Javelin Guardrails"
                })).map_err(|e| ErrorData::internal_error(format!("Failed to create JSON content: {}", e), None))?;

                Ok(CallToolResult {
                    content: vec![content],
                    is_error: None,
                })
            }
            "proxy" => {
                debug!("Executing proxy tool");
                
                // Extract target and request from arguments
                let args = params.arguments.as_ref()
                    .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;
                
                let target = args.get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'target' parameter", None))?;
                
                let request_value = args.get("request")
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'request' parameter", None))?;

                // First validate the request
                let is_valid = self.validate_request(request_value).await?;
                
                if !is_valid {
                    return Ok(CallToolResult {
                        content: vec![Content::text("Request blocked by Javelin Guardrails")],
                        is_error: Some(true),
                    });
                }

                // Parse the request as a tool call
                let tool_request: CallToolRequestParam = serde_json::from_value(request_value.clone())
                    .map_err(|e| ErrorData::invalid_params(format!("Invalid tool request: {}", e), None))?;

                // Proxy to target
                self.proxy_to_target(target, tool_request).await
            }
            _ => {
                warn!("Unknown tool requested: {}", params.name);
                Err(ErrorData::method_not_found::<CallToolRequestMethod>())
            }
        }
    }

    async fn list_prompts(
        &self,
        _: Option<PaginatedRequestParam>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        debug!("Listing prompts (none available in proxy mode)");
        
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _params: GetPromptRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        Err(ErrorData::method_not_found::<rmcp::model::GetPromptRequestMethod>())
    }

    async fn list_resources(
        &self,
        _: Option<PaginatedRequestParam>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        debug!("Listing resources (none available in proxy mode)");
        
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _params: ReadResourceRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        Err(ErrorData::method_not_found::<rmcp::model::ReadResourceRequestMethod>())
    }
}

/// Create the validate tool definition
fn create_validate_tool() -> Tool {
    use std::sync::Arc;
    use serde_json::Map;

    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), json!({
        "request": {
            "type": "object",
            "description": "The MCP request to validate"
        }
    }));
    schema.insert("required".to_string(), json!(["request"]));

    Tool {
        name: "validate".into(),
        description: Some("Validate a request against Javelin Guardrails".into()),
        input_schema: Arc::new(schema),
        annotations: None,
    }
}

/// Create the proxy tool definition
fn create_proxy_tool() -> Tool {
    use std::sync::Arc;
    use serde_json::Map;

    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), json!({
        "target": {
            "type": "string",
            "description": "Target MCP server identifier"
        },
        "request": {
            "type": "object",
            "description": "The MCP request to proxy"
        }
    }));
    schema.insert("required".to_string(), json!(["target", "request"]));

    Tool {
        name: "proxy".into(),
        description: Some("Proxy a validated request to a target MCP server".into()),
        input_schema: Arc::new(schema),
        annotations: None,
    }
}

/// Generate instructions for the proxy server
fn generate_proxy_instructions() -> String {
    r#"
This is a Ramparts MCP Proxy with Javelin Guardrails integration.

**Available Tools:**
1. **validate** - Validate any MCP request against Javelin Guardrails security policies
2. **proxy** - Validate and proxy requests to target MCP servers

**Security Features:**
- All requests are validated against Javelin Guardrails
- Malicious or policy-violating requests are blocked
- Transparent proxying maintains MCP protocol compatibility
- Configurable fail-open/fail-closed behavior

**Usage:**
1. Use `validate` to check if a request would pass security policies
2. Use `proxy` to validate and forward requests to target MCP servers

**Licensing:**
This proxy component requires a valid Javelin API key and uses proprietary licensing.
"#.to_string()
}
