//! Navigator binding for QuickJS.
//!
//! CRITICAL: `navigator.webdriver` MUST return `undefined`.
//! Not `false`. Not `null`. `undefined`.
//! This is enforced via `Object.defineProperty` in the shims layer.

/// Persona data used for navigator properties.
///
/// Decision D-14: per-session consistent persona to avoid
/// cross-session fingerprint tracking.
#[derive(Debug, Clone)]
pub struct Persona {
    /// User-agent string (e.g., Chrome 130 on Windows).
    pub user_agent: String,
    /// Browser language (e.g., "en-US").
    pub language: String,
    /// Hardware concurrency — MUST be 4, 8, or 16. Never 1. Never 128.
    pub hardware_concurrency: u8,
    /// Platform string (e.g., "Win32", "MacIntel", "Linux x86_64").
    pub platform: String,
    /// Screen width in pixels.
    pub screen_width: u32,
    /// Screen height in pixels.
    pub screen_height: u32,
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36".to_string(),
            language: "en-US".to_string(),
            hardware_concurrency: 8,
            platform: "Win32".to_string(),
            screen_width: 1920,
            screen_height: 1080,
        }
    }
}

impl Persona {
    /// Validate that hardware_concurrency is a realistic value.
    ///
    /// Must be 4, 8, or 16. Anything else is a bot fingerprint.
    pub fn validate(&self) -> bool {
        matches!(self.hardware_concurrency, 4 | 8 | 16)
    }
}
