//! Security utilities for CheckStream proxy
//!
//! Provides URL validation, SSRF protection, and other security helpers.

use std::net::IpAddr;
use thiserror::Error;
use url::Url;

/// Security-related errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("URL scheme '{0}' is not allowed, only HTTPS is permitted")]
    InvalidScheme(String),

    #[error("Host '{0}' is blocked: internal/private IP addresses are not allowed")]
    BlockedHost(String),

    #[error("URL must have a host")]
    MissingHost,
}

/// Blocked hostnames that should never be used as backend URLs
const BLOCKED_HOSTNAMES: &[&str] = &[
    "localhost",
    "localhost.localdomain",
    "ip6-localhost",
    "ip6-loopback",
    // Cloud metadata services
    "metadata.google.internal",
    "metadata.goog",
    "169.254.169.254",  // AWS/GCP/Azure metadata
    "fd00:ec2::254",    // AWS IPv6 metadata
];

/// Configuration for URL validation
#[derive(Debug, Clone)]
pub struct UrlValidationConfig {
    /// Allow HTTP scheme (not recommended for production)
    pub allow_http: bool,
    /// Allow localhost/loopback addresses (for development only)
    pub allow_localhost: bool,
    /// Allow private/internal IP ranges (RFC 1918, etc.)
    pub allow_private_ips: bool,
    /// Optional list of allowed domains (if set, only these domains are permitted)
    pub allowed_domains: Option<Vec<String>>,
}

impl Default for UrlValidationConfig {
    fn default() -> Self {
        Self {
            allow_http: false,
            allow_localhost: false,
            allow_private_ips: false,
            allowed_domains: None,
        }
    }
}

impl UrlValidationConfig {
    /// Development configuration that allows localhost
    pub fn development() -> Self {
        Self {
            allow_http: true,
            allow_localhost: true,
            allow_private_ips: true,
            allowed_domains: None,
        }
    }
}

/// Validates a backend URL to prevent SSRF attacks.
///
/// This function checks:
/// - URL scheme (only HTTPS allowed by default)
/// - Host is not a blocked hostname (localhost, metadata services)
/// - Host is not a private/internal IP address
/// - Optionally validates against an allowlist of domains
pub fn validate_backend_url(url_str: &str, config: &UrlValidationConfig) -> Result<Url, SecurityError> {
    let url = Url::parse(url_str)?;

    // Check scheme
    match url.scheme() {
        "https" => {}
        "http" if config.allow_http => {}
        scheme => return Err(SecurityError::InvalidScheme(scheme.to_string())),
    }

    // Check host exists
    let host = url.host_str().ok_or(SecurityError::MissingHost)?;

    // Check against blocked hostnames
    if !config.allow_localhost {
        let host_lower = host.to_lowercase();
        for blocked in BLOCKED_HOSTNAMES {
            if host_lower == *blocked || host_lower.ends_with(&format!(".{}", blocked)) {
                return Err(SecurityError::BlockedHost(host.to_string()));
            }
        }
    }

    // Check if it's an IP address and validate it
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !config.allow_localhost && is_loopback(&ip) {
            return Err(SecurityError::BlockedHost(host.to_string()));
        }
        if !config.allow_private_ips && is_private_ip(&ip) {
            return Err(SecurityError::BlockedHost(host.to_string()));
        }
        if is_link_local(&ip) {
            return Err(SecurityError::BlockedHost(host.to_string()));
        }
    }

    // Check against allowlist if configured
    if let Some(ref allowed) = config.allowed_domains {
        let host_lower = host.to_lowercase();
        let is_allowed = allowed.iter().any(|domain| {
            let domain_lower = domain.to_lowercase();
            host_lower == domain_lower || host_lower.ends_with(&format!(".{}", domain_lower))
        });
        if !is_allowed {
            return Err(SecurityError::BlockedHost(format!(
                "{} is not in the allowed domains list",
                host
            )));
        }
    }

    Ok(url)
}

/// Check if an IP address is a loopback address
fn is_loopback(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// Check if an IP address is in a private range (RFC 1918, RFC 4193, etc.)
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            // RFC 1918 private ranges
            // 10.0.0.0/8
            // 172.16.0.0/12
            // 192.168.0.0/16
            v4.is_private()
                // 100.64.0.0/10 (Carrier-grade NAT)
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
                // 0.0.0.0/8 (current network)
                || v4.octets()[0] == 0
        }
        IpAddr::V6(v6) => {
            // fc00::/7 (Unique Local Address)
            let segments = v6.segments();
            (segments[0] & 0xfe00) == 0xfc00
                // fe80::/10 is handled by is_link_local
                // ::1 is handled by is_loopback
        }
    }
}

/// Check if an IP address is link-local
fn is_link_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            // 169.254.0.0/16 (APIPA) - includes cloud metadata endpoint!
            v4.octets()[0] == 169 && v4.octets()[1] == 254
        }
        IpAddr::V6(v6) => {
            // fe80::/10
            let segments = v6.segments();
            (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_https_url() {
        let config = UrlValidationConfig::default();
        let result = validate_backend_url("https://api.openai.com/v1", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_blocked_by_default() {
        let config = UrlValidationConfig::default();
        let result = validate_backend_url("http://api.example.com", &config);
        assert!(matches!(result, Err(SecurityError::InvalidScheme(_))));
    }

    #[test]
    fn test_http_allowed_when_configured() {
        let config = UrlValidationConfig {
            allow_http: true,
            ..Default::default()
        };
        let result = validate_backend_url("http://api.example.com", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_localhost_blocked() {
        let config = UrlValidationConfig::default();
        let result = validate_backend_url("https://localhost:8080", &config);
        assert!(matches!(result, Err(SecurityError::BlockedHost(_))));
    }

    #[test]
    fn test_loopback_ip_blocked() {
        let config = UrlValidationConfig::default();
        let result = validate_backend_url("https://127.0.0.1:8080", &config);
        assert!(matches!(result, Err(SecurityError::BlockedHost(_))));
    }

    #[test]
    fn test_aws_metadata_blocked() {
        let config = UrlValidationConfig::default();
        let result = validate_backend_url("http://169.254.169.254/latest/meta-data/", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_private_ip_blocked() {
        let config = UrlValidationConfig::default();

        // 10.x.x.x
        let result = validate_backend_url("https://10.0.0.1:8080", &config);
        assert!(matches!(result, Err(SecurityError::BlockedHost(_))));

        // 192.168.x.x
        let result = validate_backend_url("https://192.168.1.1:8080", &config);
        assert!(matches!(result, Err(SecurityError::BlockedHost(_))));

        // 172.16.x.x
        let result = validate_backend_url("https://172.16.0.1:8080", &config);
        assert!(matches!(result, Err(SecurityError::BlockedHost(_))));
    }

    #[test]
    fn test_allowlist() {
        let config = UrlValidationConfig {
            allowed_domains: Some(vec![
                "api.openai.com".to_string(),
                "api.anthropic.com".to_string(),
            ]),
            ..Default::default()
        };

        // Allowed domain works
        let result = validate_backend_url("https://api.openai.com/v1", &config);
        assert!(result.is_ok());

        // Subdomain of allowed domain works
        let result = validate_backend_url("https://east.api.openai.com/v1", &config);
        assert!(result.is_ok());

        // Non-allowed domain blocked
        let result = validate_backend_url("https://api.example.com/v1", &config);
        assert!(matches!(result, Err(SecurityError::BlockedHost(_))));
    }

    #[test]
    fn test_development_config_allows_localhost() {
        let config = UrlValidationConfig::development();
        let result = validate_backend_url("http://localhost:8080", &config);
        assert!(result.is_ok());
    }
}
