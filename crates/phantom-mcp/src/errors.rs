//! Error hierarchy for the Phantom Engine MCP Server.
//!
//! Implements the EXACT error hierarchy specified in `references/error-types.md`.
//! Uses `thiserror` for all types. Maps Rust errors to MCP error codes.

use thiserror::Error;

/// Top-level error type used at the MCP server boundary.
#[derive(Error, Debug)]
pub enum BrowserError {
    /// Network-related failure.
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    /// DOM interaction or query failure.
    #[error("DOM error: {0}")]
    Dom(#[from] DomError),

    /// JavaScript execution failure.
    #[error("JavaScript error: {0}")]
    JavaScript(#[from] JsError),

    /// Page navigation failure.
    #[error("navigation error: {0}")]
    Navigation(#[from] NavigationError),

    /// Browser session management failure.
    #[error("session error: {0}")]
    Session(#[from] SessionError),

    /// Internal engine error.
    #[error("internal error: {0}")]
    Internal(#[from] InternalError),
}

/// Errors related to network requests.
#[derive(Error, Debug)]
pub enum NetworkError {
    /// DNS resolution failed.
    #[error("DNS resolution failed for {host}")]
    Dns {
        /// The host that failed to resolve.
        host: String,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// TLS handshake failed.
    #[error("TLS handshake failed: {0}")]
    Tls(String),

    /// Request timed out.
    #[error("request timeout after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// HTTP error response.
    #[error("HTTP error {status}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Optional response body.
        body: Option<String>,
    },

    /// Connection was refused.
    #[error("connection refused: {0}")]
    ConnectionRefused(String),
}

/// Errors related to DOM querying and interaction.
#[derive(Error, Debug)]
pub enum DomError {
    /// Element could not be found.
    #[error("element not found: selector '{selector}'")]
    ElementNotFound {
        /// The selector that yielded no results.
        selector: String,
    },

    /// Element is no longer attached to the DOM.
    #[error("stale element reference: '{selector}'")]
    StaleElement {
        /// The selector of the stale element.
        selector: String,
    },

    /// The provided selector is syntactically invalid.
    #[error("invalid selector: {0}")]
    InvalidSelector(String),

    /// Element was found but cannot be interacted with (e.g., invisible, disabled).
    #[error("element not interactable: {reason}")]
    NotInteractable {
        /// Explanation of why interaction failed.
        reason: String,
        /// The selector of the target element.
        selector: String,
    },
}

/// Errors related to JavaScript execution.
#[derive(Error, Debug)]
pub enum JsError {
    /// An uncaught JavaScript exception.
    #[error("uncaught exception: {message}\nstack: {stack}")]
    UncaughtException {
        /// Exception message.
        message: String,
        /// Exception stack trace.
        stack: String,
    },

    /// Script execution exceeded the hard timeout.
    #[error("script execution timed out after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// QuickJS heap out of memory.
    #[error("JavaScript heap out of memory")]
    OutOfMemory,

    /// Error during script evaluation (e.g., syntax error).
    #[error("evaluation error: {0}")]
    Evaluation(String),
}

/// Errors related to page navigation.
#[derive(Error, Debug)]
pub enum NavigationError {
    /// Navigation caught in an infinite redirect loop.
    #[error("redirect loop detected")]
    RedirectLoop,

    /// Navigation blocked by a policy (e.g., CSP or generic blocklist).
    #[error("navigation blocked by policy: {reason}")]
    Blocked {
        /// Reason for the block.
        reason: String,
    },

    /// Unsupported URL scheme.
    #[error("unsupported protocol: {protocol}")]
    UnsupportedProtocol {
        /// The unsupported protocol.
        protocol: String,
    },
}

/// Errors related to session management.
#[derive(Error, Debug)]
pub enum SessionError {
    /// The requested session has expired or been destroyed.
    #[error("session expired: {session_id}")]
    Expired {
        /// The ID of the expired session.
        session_id: String,
    },

    /// Session exceeded resource limits.
    #[error("resource budget exceeded: {resource} used {used} of {limit}")]
    BudgetExceeded {
        /// the resource that was exceeded.
        resource: String,
        /// amount used.
        used: u64,
        /// maximum allowed limit.
        limit: u64,
    },

    /// The requested tab does not exist in the session.
    #[error("tab not found: {tab_id}")]
    TabNotFound {
        /// The ID of the missing tab.
        tab_id: String,
    },
}

/// Internal engine errors.
#[derive(Error, Debug)]
pub enum InternalError {
    /// No isolates available in the pool.
    #[error("isolate pool exhausted (max {max_isolates})")]
    IsolatePoolExhausted {
        /// The maximum number of isolates.
        max_isolates: usize,
    },

    /// Failed to send message over internal channel.
    #[error("channel send error: {0}")]
    ChannelSend(String),

    /// A background thread or task panicked.
    #[error("engine panicked: {0}")]
    Panic(String),
}

impl BrowserError {
    /// Map a Rust error variant to its canonical MCP error code.
    ///
    /// The resulting string is suitable for use in the `code` field of an `McpError` response.
    pub fn to_mcp_code(&self) -> String {
        match self {
            BrowserError::Dom(DomError::ElementNotFound { .. }) => "element_not_found".to_string(),
            BrowserError::Dom(DomError::StaleElement { .. }) => "stale_element".to_string(),
            BrowserError::Dom(DomError::NotInteractable { .. }) => "not_interactable".to_string(),
            BrowserError::Dom(DomError::InvalidSelector(_)) => "invalid_selector".to_string(),
            BrowserError::JavaScript(JsError::Timeout { .. }) => "js_timeout".to_string(),
            BrowserError::JavaScript(JsError::OutOfMemory) => "js_oom".to_string(),
            BrowserError::JavaScript(JsError::UncaughtException { .. }) => {
                "js_exception".to_string()
            }
            BrowserError::JavaScript(JsError::Evaluation(_)) => "js_evaluation_failed".to_string(),
            BrowserError::Network(NetworkError::Timeout { .. }) => "network_timeout".to_string(),
            BrowserError::Network(NetworkError::Dns { .. }) => "dns_failed".to_string(),
            BrowserError::Network(NetworkError::Tls(_)) => "tls_failed".to_string(),
            BrowserError::Network(NetworkError::Http { .. }) => "http_error".to_string(),
            BrowserError::Network(NetworkError::ConnectionRefused(_)) => {
                "connection_refused".to_string()
            }
            BrowserError::Navigation(NavigationError::RedirectLoop) => "redirect_loop".to_string(),
            BrowserError::Navigation(NavigationError::Blocked { .. }) => {
                "navigation_blocked".to_string()
            }
            BrowserError::Navigation(NavigationError::UnsupportedProtocol { .. }) => {
                "unsupported_protocol".to_string()
            }
            BrowserError::Session(SessionError::Expired { .. }) => "session_expired".to_string(),
            BrowserError::Session(SessionError::BudgetExceeded { .. }) => {
                "budget_exceeded".to_string()
            }
            BrowserError::Session(SessionError::TabNotFound { .. }) => "tab_not_found".to_string(),
            BrowserError::Internal(InternalError::IsolatePoolExhausted { .. }) => {
                "pool_exhausted".to_string()
            }
            BrowserError::Internal(InternalError::ChannelSend(_)) => {
                "internal_channel_error".to_string()
            }
            BrowserError::Internal(InternalError::Panic(_)) => "engine_panic".to_string(),
        }
    }
}
