pub mod cache;
pub mod config;
pub mod javelin;
pub mod licensing;
pub mod logging;
pub mod mcp_server;
pub mod proxy;
pub mod validation_service;

pub use config::*;
pub use javelin::*;
pub use licensing::*;
pub use logging::*;
pub use mcp_server::*;
pub use proxy::*;
pub use validation_service::*;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_optimized_request_formatting() {
        // Test our optimized format_request_for_guard function

        // Test file read request
        let read_request = json!({
            "params": {
                "name": "read_file",
                "arguments": {
                    "path": "/home/user/document.txt"
                }
            }
        });

        let formatted = JavelinClient::format_request_for_guard(&read_request);
        assert_eq!(formatted, "read file /home/user/document.txt");

        // Test file write request
        let write_request = json!({
            "params": {
                "name": "write_file",
                "arguments": {
                    "path": "/home/user/output.txt",
                    "content": "This is a very long content that should be truncated after 50 characters to avoid overwhelming the security system"
                }
            }
        });

        let formatted = JavelinClient::format_request_for_guard(&write_request);
        assert!(formatted.starts_with("write file /home/user/output.txt with content:"));
        assert!(formatted.contains("This is a very long content that should be trunca")); // 50 char limit

        // Test command execution
        let cmd_request = json!({
            "params": {
                "name": "execute_command",
                "arguments": {
                    "command": "ls -la"
                }
            }
        });

        let formatted = JavelinClient::format_request_for_guard(&cmd_request);
        assert_eq!(formatted, "execute command: ls -la");

        // Test database query
        let db_request = json!({
            "params": {
                "name": "database_query",
                "arguments": {
                    "query": "SELECT * FROM users"
                }
            }
        });

        let formatted = JavelinClient::format_request_for_guard(&db_request);
        assert_eq!(formatted, "run database query: SELECT * FROM users");

        // Test network request
        let net_request = json!({
            "params": {
                "name": "network_request",
                "arguments": {
                    "url": "https://api.example.com/data"
                }
            }
        });

        let formatted = JavelinClient::format_request_for_guard(&net_request);
        assert_eq!(
            formatted,
            "make network request to: https://api.example.com/data"
        );

        // Test unknown tool (fallback)
        let unknown_request = json!({
            "params": {
                "name": "unknown_tool",
                "arguments": {
                    "param1": "value1"
                }
            }
        });

        let formatted = JavelinClient::format_request_for_guard(&unknown_request);
        assert!(formatted.starts_with("use tool unknown_tool with arguments:"));
    }

    #[test]
    fn test_extract_tool_info() {
        // Test successful extraction
        let request = json!({
            "params": {
                "name": "read_file",
                "arguments": {
                    "path": "/test/file.txt"
                }
            }
        });

        let result = JavelinClient::extract_tool_info(&request);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "read_file");
        assert_eq!(
            args.get("path").unwrap().as_str().unwrap(),
            "/test/file.txt"
        );

        // Test missing params
        let invalid_request = json!({
            "method": "call_tool"
        });

        let result = JavelinClient::extract_tool_info(&invalid_request);
        assert!(result.is_none());

        // Test missing name
        let no_name_request = json!({
            "params": {
                "arguments": {
                    "path": "/test/file.txt"
                }
            }
        });

        let result = JavelinClient::extract_tool_info(&no_name_request);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_helpers() {
        let args = json!({
            "path": "/test/file.txt",
            "command": "echo hello",
            "url": "https://example.com"
        });

        // Test format_with_path
        let result = JavelinClient::format_with_path("read file", &args);
        assert_eq!(result, "read file /test/file.txt");

        // Test format_with_field
        let result = JavelinClient::format_with_field("execute command", "command", &args);
        assert_eq!(result, "execute command: echo hello");

        let result = JavelinClient::format_with_field("make request to", "url", &args);
        assert_eq!(result, "make request to: https://example.com");

        // Test with missing field
        let empty_args = json!({});
        let result = JavelinClient::format_with_path("read file", &empty_args);
        assert_eq!(result, "read file");
    }

    #[test]
    fn test_validation_service_helpers() {
        // Test helper functions
        let request = json!({
            "id": "test-123",
            "method": "call_tool"
        });

        let request_id = extract_request_id(&request);
        assert_eq!(request_id, Some(json!("test-123")));

        let is_jsonrpc = is_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": "test"
        }));
        assert!(is_jsonrpc);

        let not_jsonrpc = is_jsonrpc_request(&json!({
            "method": "test"
        }));
        assert!(!not_jsonrpc);

        // Test success response creation
        let response =
            create_success_response(Some(json!("test-123")), json!({"result": "success"}));
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], "test-123");
        assert_eq!(response["result"]["result"], "success");
    }

    #[test]
    fn test_proxy_config_validation() {
        let mut config = ProxyConfig::default();

        // Valid config should pass
        config.javelin.api_key = "test-key".to_string();
        assert!(config.validate().is_ok());

        // Empty API key should fail
        config.javelin.api_key = "".to_string();
        assert!(config.validate().is_err());

        // Zero timeout should fail
        config.javelin.api_key = "test-key".to_string();
        config.javelin.timeout_seconds = 0;
        assert!(config.validate().is_err());

        // Invalid listen address should fail
        config.javelin.timeout_seconds = 30;
        config.listen_address = "invalid-address".to_string();
        assert!(config.validate().is_err());
    }
}
