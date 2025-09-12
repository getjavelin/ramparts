use ramparts_common::{
    anyhow::Result,
    tracing::{debug, error, info, warn},
};
use ramparts_proxy::{JavelinClient, ProxyConfig, ValidationService};
use serde_json::{json, Value};
use std::{env, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{Child, Command},
    sync::Mutex,
};
use tracing_subscriber;

/// Lightweight stdio proxy that intercepts MCP JSON-RPC requests/responses
/// and validates them through Javelin Guardrails before forwarding
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--self-check" {
        return self_check().await;
    }

    info!(
        "Starting Ramparts MCP Proxy Stdio v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Get target command from environment
    let target_cmd = env::var("RAMPARTS_TARGET_CMD").map_err(|_| {
        ramparts_common::anyhow::anyhow!("RAMPARTS_TARGET_CMD environment variable required")
    })?;

    let target_args: Vec<String> = env::var("RAMPARTS_TARGET_ARGS")
        .unwrap_or_else(|_| "[]".to_string())
        .parse::<Value>()
        .map_err(|e| ramparts_common::anyhow::anyhow!("Invalid RAMPARTS_TARGET_ARGS JSON: {}", e))?
        .as_array()
        .ok_or_else(|| {
            ramparts_common::anyhow::anyhow!("RAMPARTS_TARGET_ARGS must be a JSON array")
        })?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    debug!("Target command: {} {:?}", target_cmd, target_args);

    // Check for bypass
    if env::var("RAMPARTS_BYPASS").unwrap_or_default() == "true" {
        warn!("Ramparts bypass enabled - running target directly without validation");
        return run_target_directly(&target_cmd, &target_args).await;
    }

    // Initialize validation service
    let config = ProxyConfig::from_env()?;
    let javelin_client = Arc::new(JavelinClient::with_behavior(
        config.javelin.api_key.clone(),
        config.javelin.base_url.clone(),
        config.javelin.timeout_seconds,
        &config.behavior,
    ));
    let validation_service = Arc::new(ValidationService::new(javelin_client, config));

    // Spawn target MCP server
    let mut child = spawn_target_server(&target_cmd, &target_args).await?;

    // Get handles to child's stdin/stdout
    let child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| ramparts_common::anyhow::anyhow!("Failed to get child stdin"))?;
    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| ramparts_common::anyhow::anyhow!("Failed to get child stdout"))?;

    // Create shared state for request tracking
    let request_tracker = Arc::new(Mutex::new(std::collections::HashMap::<Value, Value>::new()));

    // Create bidirectional proxy tasks
    let validation_service_clone = validation_service.clone();
    let request_tracker_clone = request_tracker.clone();

    // Task 1: Client stdin -> Child stdin (with request validation)
    let stdin_task = tokio::spawn(async move {
        proxy_client_to_server(validation_service_clone, request_tracker_clone, child_stdin).await
    });

    // Task 2: Child stdout -> Client stdout (with response validation)
    let stdout_task = tokio::spawn(async move {
        proxy_server_to_client(validation_service, request_tracker, child_stdout).await
    });

    // Wait for either task to complete (or fail)
    tokio::select! {
        result = stdin_task => {
            if let Err(e) = result? {
                error!("Stdin proxy task failed: {}", e);
            }
        }
        result = stdout_task => {
            if let Err(e) = result? {
                error!("Stdout proxy task failed: {}", e);
            }
        }
    }

    // Clean up child process
    if let Err(e) = child.kill().await {
        warn!("Failed to kill child process: {}", e);
    }

    Ok(())
}

/// Proxy requests from client to server with validation
async fn proxy_client_to_server(
    validation_service: Arc<ValidationService>,
    request_tracker: Arc<Mutex<std::collections::HashMap<Value, Value>>>,
    mut child_stdin: tokio::process::ChildStdin,
) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut writer = BufWriter::new(&mut child_stdin);

    loop {
        match read_jsonrpc_message(&mut reader).await {
            None => break, // EOF
            Some(Ok(payload)) => {
                // Log redacted request preview
                if let Ok(json) = serde_json::from_str::<Value>(&payload) {
                    let redacted = ramparts_proxy::logging::sanitize_json_for_log(&json);
                    if let Ok(s) = serde_json::to_string(&redacted) {
                        debug!("Received request: {}", ramparts_proxy::logging::truncate_for_log(&s));
                    } else {
                        debug!("Received request (redacted)");
                    }
                } else {
                    debug!("Received request (non-JSON, {} bytes)", payload.len());
                }

                // Parse JSON-RPC request
                match serde_json::from_str::<Value>(&payload) {
                    Ok(request) => {
                        // Validate request
                        match validation_service.validate_request(&request).await {
                            Ok(validation_result) => {
                                if validation_result.allowed {
                                    // Request approved - forward to child
                                    debug!("Request approved, forwarding to target server");

                                    // Track request for response correlation
                                    if let Some(id) = request.get("id") {
                                        let mut tracker = request_tracker.lock().await;
                                        tracker.insert(id.clone(), request.clone());
                                    }

                                    write_jsonrpc_message(&mut writer, &payload).await?;
                                    writer.flush().await?;
                                } else {
                                    // Request blocked - return error to client
                                    warn!(
                                        "Request blocked by validation service: {:?}",
                                        validation_result.reason
                                    );
                                    let error_response = json!({
                                        "jsonrpc": "2.0",
                                        "id": request.get("id"),
                                        "error": {
                                            "code": -32603,
                                            "message": "Request blocked by Ramparts security",
                                            "data": {
                                                "reason": validation_result.reason,
                                                "blocked_by": "ramparts-mcp-proxy-stdio"
                                            }
                                        }
                                    });
                                    let out = serde_json::to_string(&error_response)?;
                                    write_jsonrpc_message(&mut tokio::io::stdout(), &out).await?;
                                }
                            }
                            Err(e) => {
                                warn!("Validation error: {}", e);
                                if validation_service.is_fail_open() {
                                    // Forward request on validation error (fail-open policy)
                                    write_jsonrpc_message(&mut writer, &payload).await?;
                                    writer.flush().await?;
                                } else {
                                    // Return an error to client (fail-closed policy)
                                    let error_response = validation_service
                                        .create_error_response(&request, &e.to_string());
                                    let out = serde_json::to_string(&error_response)?;
                                    write_jsonrpc_message(&mut tokio::io::stdout(), &out).await?;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC request: {}", e);
                        // Forward malformed requests as-is (let the target server handle them)
                        write_jsonrpc_message(&mut writer, &payload).await?;
                        writer.flush().await?;
                    }
                }
            }
            Some(Err(e)) => {
                error!("Failed to read from stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Proxy responses from server to client with validation
async fn proxy_server_to_client(
    validation_service: Arc<ValidationService>,
    request_tracker: Arc<Mutex<std::collections::HashMap<Value, Value>>>,
    child_stdout: tokio::process::ChildStdout,
) -> Result<()> {
    let mut reader = BufReader::new(child_stdout);
    loop {
        match read_jsonrpc_message(&mut reader).await {
            None => break,
            Some(Ok(payload)) => {
                // Log truncated response preview to avoid leaking sensitive content
                debug!(
                    "Received response (preview): {}",
                    ramparts_proxy::logging::truncate_for_log(&payload)
                );

                // Parse JSON-RPC response
                match serde_json::from_str::<Value>(&payload) {
                    Ok(response) => {
                        // Get original request context if available
                        let _original_request = if let Some(id) = response.get("id") {
                            let mut tracker = request_tracker.lock().await;
                            tracker.remove(id)
                        } else {
                            None
                        };

                        // Validate response (optional for MVP)
                        match validation_service.validate_response(&response).await {
                            Ok(validation_result) => {
                                if validation_result.allowed {
                                    // Response approved - forward to client
                                    debug!("Response approved, forwarding to client");
                                    write_jsonrpc_message(&mut tokio::io::stdout(), &payload).await?;
                                } else {
                                    // Response blocked
                                    warn!(
                                        "Response blocked by validation service: {:?}",
                                        validation_result.reason
                                    );
                                    let error_response = json!({
                                        "jsonrpc": "2.0",
                                        "id": response.get("id"),
                                        "error": {
                                            "code": -32603,
                                            "message": "Response blocked by Ramparts security",
                                            "data": {
                                                "reason": validation_result.reason,
                                                "blocked_by": "ramparts-mcp-proxy-stdio"
                                            }
                                        }
                                    });
                                    let out = serde_json::to_string(&error_response)?;
                                    write_jsonrpc_message(&mut tokio::io::stdout(), &out).await?;
                                }
                            }
                            Err(e) => {
                                warn!("Response validation failed: {}", e);
                                // Forward response on validation error (fail-open for responses)
                                write_jsonrpc_message(&mut tokio::io::stdout(), &payload).await?;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC response: {}", e);
                        // Forward malformed responses as-is
                        write_jsonrpc_message(&mut tokio::io::stdout(), &payload).await?;
                    }
                }
            }
            Some(Err(e)) => {
                error!("Failed to read from child stdout: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Read a single JSON-RPC message supporting Content-Length framing and newline JSON fallback.
async fn read_jsonrpc_message<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
) -> Option<Result<String, ramparts_common::anyhow::Error>> {
    let mut header = String::new();
    let mut content_length: Option<usize> = None;

    loop {
        header.clear();
        match reader.read_line(&mut header).await {
            Ok(0) => return None,
            Ok(_) => {
                let line = header.trim_end();
                if line.is_empty() {
                    break; // end of headers
                }
                if let Some(rest) = line.strip_prefix("Content-Length:") {
                    content_length = rest.trim().parse::<usize>().ok();
                }
            }
            Err(e) => {
                return Some(Err(ramparts_common::anyhow::anyhow!(
                    "read header failed: {}",
                    e
                )));
            }
        }
    }

    if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        if let Err(e) = reader.read_exact(&mut buf).await {
            return Some(Err(ramparts_common::anyhow::anyhow!(
                "read body failed: {}",
                e
            )));
        }
        let payload = String::from_utf8_lossy(&buf).to_string();
        Some(Ok(payload))
    } else {
        // Fallback: read a single JSON line
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => None,
            Ok(_) => Some(Ok(line.trim().to_string())),
            Err(e) => Some(Err(ramparts_common::anyhow::anyhow!(
                "read line failed: {}",
                e
            ))),
        }
    }
}

/// Write a JSON-RPC message using Content-Length framing.
async fn write_jsonrpc_message<W: AsyncWriteExt + Unpin>(writer: &mut W, payload: &str) -> Result<()> {
    let bytes = payload.as_bytes();
    let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(bytes).await?;
    Ok(())
}

/// Spawn the target MCP server process
async fn spawn_target_server(command: &str, args: &[String]) -> Result<Child> {
    debug!("Spawning target server: {} {:?}", command, args);

    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // Let stderr pass through for debugging
        .kill_on_drop(true);

    // Pass through environment variables (except Ramparts-specific ones)
    for (key, value) in env::vars() {
        if !key.starts_with("RAMPARTS_") {
            cmd.env(key, value);
        }
    }

    cmd.spawn().map_err(|e| {
        ramparts_common::anyhow::anyhow!("Failed to spawn target server '{}': {}", command, e)
    })
}

/// Run target server directly without validation (bypass mode)
async fn run_target_directly(command: &str, args: &[String]) -> Result<()> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status().await?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Self-check command for diagnostics
async fn self_check() -> Result<()> {
    println!("Ramparts MCP Proxy Stdio v{}", env!("CARGO_PKG_VERSION"));
    println!("Self-check: OK");

    // Check if Javelin client can be initialized
    match ProxyConfig::from_env() {
        Ok(config) => {
            println!("Configuration: OK");
            let _client = JavelinClient::new(
                config.javelin.api_key.clone(),
                Some(config.javelin.base_url.clone()),
            );
            println!("Javelin client: OK");
        }
        Err(e) => println!("Configuration: ERROR - {}", e),
    }

    Ok(())
}
