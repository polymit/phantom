//! Location binding for QuickJS.
//!
//! Implements `window.location` properties and navigation methods.

/// Parsed URL components for the location binding.
#[derive(Debug, Clone, Default)]
pub struct LocationState {
    /// Full URL (e.g., "https://example.com/path?q=1").
    pub href: String,
    /// Hostname (e.g., "example.com").
    pub hostname: String,
    /// Pathname (e.g., "/path").
    pub pathname: String,
    /// Protocol (e.g., "https:").
    pub protocol: String,
    /// Host (e.g., "example.com:443").
    pub host: String,
    /// Port (e.g., "443" or "").
    pub port: String,
    /// Search string (e.g., "?q=1").
    pub search: String,
    /// Hash fragment (e.g., "#section").
    pub hash: String,
    /// Origin (e.g., "https://example.com").
    pub origin: String,
}

impl LocationState {
    /// Parse data from a full URL string using the `url` crate (Bug 8).
    pub fn from_url(url_str: &str) -> Self {
        let parsed = match url::Url::parse(url_str) {
            Ok(u) => u,
            Err(_) => {
                return Self {
                    href: url_str.to_string(),
                    ..Default::default()
                };
            }
        };

        Self {
            href: url_str.to_string(),
            protocol: format!("{}:", parsed.scheme()),
            host: parsed
                .host_str()
                .map(|h| {
                    if let Some(p) = parsed.port() {
                        format!("{}:{}", h, p)
                    } else {
                        h.to_string()
                    }
                })
                .unwrap_or_default(),
            hostname: parsed.host_str().unwrap_or_default().to_string(),
            port: parsed.port().map(|p| p.to_string()).unwrap_or_default(),
            pathname: parsed.path().to_string(),
            search: if let Some(q) = parsed.query() {
                format!("?{}", q)
            } else {
                String::new()
            },
            hash: if let Some(f) = parsed.fragment() {
                format!("#{}", f)
            } else {
                String::new()
            },
            origin: format!(
                "{}://{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or_default()
            ),
        }
    }
}
