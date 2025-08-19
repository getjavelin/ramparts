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

/// Common HTTP and protocol constants
pub mod protocol {
    pub const USER_AGENT: &str = concat!("ramparts/", env!("CARGO_PKG_VERSION"));
}
