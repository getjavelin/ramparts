use ramparts_common::{
    anyhow::Result,
    tracing::{error, info, warn},
};
use std::env;

/// License validation result
#[derive(Debug)]
pub struct LicenseInfo {
    pub api_key: String,
    pub key_source: String,
    pub is_valid: bool,
}

/// Validate Javelin API key for proxy licensing
pub fn validate_license() -> Result<String> {
    let license_info = validate_license_detailed()?;

    if !license_info.is_valid {
        return Err(ramparts_common::anyhow::anyhow!(
            "Invalid license. Proxy functionality requires a valid Javelin API key."
        ));
    }

    Ok(license_info.api_key)
}

/// Detailed license validation with comprehensive checks
pub fn validate_license_detailed() -> Result<LicenseInfo> {
    info!("Validating Javelin proxy license...");

    // Check for Javelin API key
    let (api_key, key_source) = if let Ok(key) = env::var("JAVELIN_API_KEY") {
        (key, "JAVELIN_API_KEY".to_string())
    } else {
        error!("No Javelin API key found in environment variables");
        return Err(ramparts_common::anyhow::anyhow!(
            "Proxy functionality requires a valid Javelin API key.\n\
            Set the JAVELIN_API_KEY environment variable.\n\n\
            To obtain a Javelin API key, visit: https://www.getjavelin.com"
        ));
    };

    // Basic validation checks
    let is_valid = validate_api_key_format(&api_key)?;

    if !is_valid {
        error!("API key format validation failed");
        return Ok(LicenseInfo {
            api_key: String::new(),
            key_source,
            is_valid: false,
        });
    }

    // Display licensing information
    display_license_info(&key_source);

    Ok(LicenseInfo {
        api_key,
        key_source,
        is_valid: true,
    })
}

/// Validate API key format
fn validate_api_key_format(api_key: &str) -> Result<bool> {
    if api_key.is_empty() {
        return Ok(false);
    }

    // Basic format validation
    if api_key.len() < 10 {
        warn!("API key appears to be too short");
        return Ok(false);
    }

    // Check for common invalid patterns
    if api_key.contains(' ') || api_key.contains('\n') || api_key.contains('\t') {
        warn!("API key contains invalid characters");
        return Ok(false);
    }

    Ok(true)
}

/// Display licensing information to the user
fn display_license_info(key_source: &str) {
    warn!("╔══════════════════════════════════════════════════════════════════════════════╗");
    warn!("║                          JAVELIN PROXY LICENSE                              ║");
    warn!("╠══════════════════════════════════════════════════════════════════════════════╣");
    warn!("║ The ramparts-proxy component uses proprietary licensing.                    ║");
    warn!("║ Usage requires a valid Javelin API key and compliance with terms.          ║");
    warn!("║                                                                              ║");
    warn!("║ API Key Source: {:<57} ║", key_source);
    warn!("║ License: Javelin Proprietary License (see LICENSE-PROXY)                   ║");
    warn!("║ Terms: https://www.getjavelin.com/terms                                     ║");
    warn!("║                                                                              ║");
    warn!("║ For questions: legal@getjavelin.com                                         ║");
    warn!("║ For API access: https://www.getjavelin.com                                  ║");
    warn!("╚══════════════════════════════════════════════════════════════════════════════╝");
}

/// Check if the current environment has a valid license
pub fn has_valid_license() -> bool {
    validate_license_detailed()
        .map(|info| info.is_valid)
        .unwrap_or(false)
}

/// Get license status information
pub fn get_license_status() -> Result<String> {
    match validate_license_detailed() {
        Ok(info) => {
            if info.is_valid {
                Ok(format!("Valid license using {}", info.key_source))
            } else {
                Ok("Invalid license".to_string())
            }
        }
        Err(e) => Ok(format!("License error: {}", e)),
    }
}
