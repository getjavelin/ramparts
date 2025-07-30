/// Constants used throughout the Ramparts MCP scanner
///
/// This module centralizes commonly used constants to reduce duplication
/// and improve maintainability.
/// Default batch size for LLM API calls
pub const DEFAULT_LLM_BATCH_SIZE: usize = 10;

/// Common error and status messages
pub mod messages {
    pub const OPENAI_NOT_CONFIGURED: &str = "OpenAI API not configured, returning empty result";
    pub const YARA_PRE_SCAN_LOADED: &str = "YARA pre-scan scanner loaded successfully";
    pub const YARA_PRE_SCAN_FAILED: &str = "Failed to load YARA pre-scan scanner";
    pub const YARA_POST_SCAN_LOADED: &str = "YARA post-scan scanner loaded successfully";
    pub const YARA_POST_SCAN_FAILED: &str = "Failed to load YARA post-scan scanner";
}

/// Common JSON-RPC and MCP protocol constants
pub mod protocol {
    #[allow(dead_code)]
    pub const JSONRPC_VERSION: &str = "2.0";
    #[allow(dead_code)]
    pub const MCP_PROTOCOL_VERSIONS: &[&str] =
        &["2025-06-18", "2024-11-05", "2024-11-01", "2024-10-01"];
    #[allow(dead_code)]
    pub const CLIENT_NAME: &str = "ramparts";
    #[allow(dead_code)]
    pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const USER_AGENT: &str = concat!("ramparts/", env!("CARGO_PKG_VERSION"));
}
