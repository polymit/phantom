//! MCP Server Core using Axum and Tokio.
//!
//! Implements the JSON-RPC 2.0 transport for the Model Context Protocol.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use cookie_store;
use indextree;
use parking_lot::RwLock;
use phantom_js::processor::JsPageProcessor;
use phantom_session::broker::SessionBroker;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::auth::require_auth;
use crate::dispatcher::dispatch;
use crate::errors::BrowserError;
use crate::metrics::init_metrics;
use metrics_exporter_prometheus::PrometheusHandle;
pub use phantom_session::circuit_breaker::CircuitState;

// ============================================================================
// JSON-RPC 2.0 Types
// ============================================================================

/// A JSON-RPC 2.0 request representing an MCP tool call.
#[derive(Deserialize, Debug)]
pub struct McpRequest {
    /// Must be exactly "2.0"
    pub jsonrpc: String,
    /// Must be "tools/call"
    pub method: String,
    /// Tool call parameters
    pub params: McpParams,
    /// Request ID for response correlation
    pub id: serde_json::Value,
}

/// Parameters for a tool call.
#[derive(Deserialize, Debug)]
pub struct McpParams {
    /// The name of the tool to execute (e.g., "browser_navigate").
    pub name: String,
    /// Arguments for the tool (JSON object).
    pub arguments: serde_json::Value,
    /// Optional session ID for tools that require existing state.
    pub session_id: Option<Uuid>,
}

/// A JSON-RPC 2.0 response wrapper.
#[derive(Serialize, Debug)]
pub struct McpResponse {
    /// Must be exactly "2.0"
    pub jsonrpc: String,
    /// The successful result payload, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// The error payload, if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
    /// Request ID from the incoming call.
    pub id: serde_json::Value,
}

/// An MCP-compliant error structure.
#[derive(Serialize, Debug)]
pub struct McpError {
    /// Canonical error code (e.g., "element_not_found").
    pub code: String,
    /// Human readable error description.
    pub message: String,
    /// Optional extended error context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl From<BrowserError> for McpError {
    fn from(err: BrowserError) -> Self {
        Self {
            code: err.to_mcp_code(),
            message: err.to_string(),
            details: None,
        }
    }
}

impl McpResponse {
    /// Create a success response.
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response.
    pub fn error(id: serde_json::Value, err: McpError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(err),
            id,
        }
    }
}

// ============================================================================
// Server State & Session Model
// ============================================================================

/// Represents a single browser tab.
#[derive(Debug, Clone)]
pub struct TabState {
    /// Tab UUID.
    pub id: String,
    /// Current URL loaded in the tab.
    pub url: String,
    /// Current page title.
    pub title: String,
}

/// Shared state for an active agent session.
pub struct McpSession {
    /// Unique session identifier.
    pub id: Uuid,
    /// When the session was established.
    pub created_at: Instant,
    /// Currently active URL.
    pub active_url: Option<String>,
    pub history: Vec<String>,
    /// Open tabs in this session.
    pub tabs: HashMap<String, TabState>,
    /// The ID of the currently active tab.
    pub active_tab: String,
    /// Persisted DOM tree for the active tab (Bug 9).
    pub dom: Option<phantom_core::dom::DomTree>,
    /// Layout bounds for the active tab (Bug 9).
    pub bounds: Option<HashMap<indextree::NodeId, phantom_core::layout::ViewportBounds>>,
    /// Multi-tenant cookie storage (Bug 11).
    pub cookie_jar: Arc<RwLock<cookie_store::CookieStore>>,
}

impl Default for McpSession {
    fn default() -> Self {
        let initial_tab = Uuid::new_v4().to_string();
        let mut tabs = HashMap::new();
        tabs.insert(
            initial_tab.clone(),
            TabState {
                id: initial_tab.clone(),
                url: "about:blank".to_string(),
                title: "New Tab".to_string(),
            },
        );
        Self {
            id: Uuid::new_v4(),
            created_at: Instant::now(),
            active_url: None,
            history: Vec::new(),
            tabs,
            active_tab: initial_tab,
            dom: None,
            bounds: None,
            cookie_jar: Arc::new(RwLock::new(cookie_store::CookieStore::default())),
        }
    }
}

/// The core MCP Server instance.
pub struct McpServer {
    pub engine: Arc<JsPageProcessor>,
    /// The session broker managing lifecycles and pooling.
    pub broker: Arc<SessionBroker>,
    /// Active sessions mapped by UUID.
    pub sessions: Arc<RwLock<HashMap<Uuid, Arc<Mutex<McpSession>>>>>,
    /// Allowed API keys for authentication.
    pub api_keys: Arc<HashSet<String>>,
    /// Prometheus handle for rendering metrics.
    pub prometheus_handle: PrometheusHandle,
}

impl McpServer {
    /// Create a new MCP Server with the given authorized API keys.
    #[tracing::instrument(skip(keys))]
    pub fn new(keys: Vec<String>) -> Self {
        let mut api_keys = HashSet::new();
        for k in keys {
            api_keys.insert(k.trim().to_string());
        }

        Self {
            engine: Arc::new(JsPageProcessor::new()),
            broker: Arc::new(SessionBroker::new()),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(api_keys),
            prometheus_handle: init_metrics(),
        }
    }

    /// Build the Axum router with all endpoints and middleware.
    pub fn router(self: Arc<Self>) -> Router {
        let keys = self.api_keys.clone();
        let auth_layer = axum::middleware::from_fn(move |req, next| {
            let keys = keys.clone();
            async move { require_auth(State(keys), req, next).await }
        });

        Router::new()
            // Authenticated routes
            .route("/mcp", post(handle_tool_call))
            .route("/mcp/stream", get(sse_handler))
            .route("/health", get(health_check))
            .route(
                "/mock",
                get(|| async {
                    (
                        StatusCode::OK,
                        [("content-type", "text/html")],
                        "<html><head><title>Mock Page</title></head><body><div id='main-body' width='500' height='500'><h1 width='100' height='50'>Hello</h1></div></body></html>",
                    )
                }),
            )
            // Public unauthenticated routes
            .route("/metrics", get(metrics_handler))
            .layer(auth_layer)
            // State extension
            .with_state(self)
    }

    /// Start the HTTP server on the given port. Blocks until shutdown.
    #[tracing::instrument(skip(self))]
    pub async fn start(self: Arc<Self>, port: u16) {
        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(&addr).await.expect("Failed to bind port");
        tracing::info!("Phantom MCP Server listening on {}", addr);

        axum::serve(listener, self.router())
            .await
            .expect("Server failed");
    }

    /// Get or create a session.
    #[tracing::instrument(skip(self))]
    pub fn get_or_create_session(
        &self,
        required_id: Option<Uuid>,
    ) -> (Uuid, Arc<Mutex<McpSession>>) {
        if let Some(id) = required_id {
            let reader = self.sessions.read();
            if let Some(session) = reader.get(&id) {
                return (id, session.clone());
            }
            drop(reader);
            // Fallthrough: requested ID missing, create new.
            // In a strict implementation we might reject this, but for agents
            // it's safer to auto-vivify if they restart.
            let mut writer = self.sessions.write();
            let new_session = McpSession {
                id,
                ..Default::default()
            };
            let arc = Arc::new(Mutex::new(new_session));
            writer.insert(id, arc.clone());
            return (id, arc);
        }

        // Create entirely new
        let mut writer = self.sessions.write();
        let session = McpSession::default();
        let id = session.id;
        let arc = Arc::new(Mutex::new(session));
        writer.insert(id, arc.clone());
        (id, arc)
    }
}

// ============================================================================
// Endpoint Handlers
// ============================================================================

/// Handle an incoming POST /mcp JSON-RPC tool call.
async fn handle_tool_call(
    State(server): State<Arc<McpServer>>,
    Json(req): Json<McpRequest>,
) -> impl IntoResponse {
    // 1. Validate JSON-RPC basics
    if req.jsonrpc != "2.0" {
        return Json(McpResponse::error(
            req.id,
            McpError {
                code: "invalid_request".to_string(),
                message: "jsonrpc must be '2.0'".to_string(),
                details: None,
            },
        ));
    }

    if req.method != "tools/call" {
        return Json(McpResponse::error(
            req.id,
            McpError {
                code: "method_not_found".to_string(),
                message: format!("Command not supported: {}", req.method),
                details: None,
            },
        ));
    }

    // 2. Resolve session context
    let (session_id, session_mutex) = server.get_or_create_session(req.params.session_id);
    let mut session = session_mutex.lock().await;

    // 3. Dispatch tool execution
    let start_time = Instant::now();
    metrics::counter!("phantom_mcp_tool_calls_total", "tool" => req.params.name.clone())
        .increment(1);

    let result = dispatch(
        &req.params.name,
        req.params.arguments,
        &mut session,
        &server.engine,
    )
    .await;

    metrics::histogram!("phantom_mcp_tool_latency_ms", "tool" => req.params.name.clone())
        .record(start_time.elapsed().as_secs_f64() * 1000.0);

    // 4. Format and return
    match result {
        Ok(mut val) => {
            // Automatically inject the session ID into successful tool responses
            // so the agent knows its state reference for subsequent calls.
            if let Some(obj) = val.as_object_mut() {
                if !obj.contains_key("session_id") {
                    obj.insert(
                        "session_id".to_string(),
                        serde_json::json!(session_id.to_string()),
                    );
                }
            }
            Json(McpResponse::success(req.id, val))
        }
        Err(err) => Json(McpResponse::error(req.id, err.into())),
    }
}

/// GET /health
async fn health_check(State(server): State<Arc<McpServer>>) -> impl IntoResponse {
    let cb_state = server.broker.isolate_pool.circuit_breaker.state();

    let status = if cb_state == CircuitState::Open {
        "degraded"
    } else {
        "ok"
    };

    Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "circuit_breaker": format!("{:?}", cb_state)
    }))
}

/// GET /metrics
async fn metrics_handler(State(server): State<Arc<McpServer>>) -> impl IntoResponse {
    server.prometheus_handle.render()
}

/// GET /mcp/stream placeholder
async fn sse_handler(State(_server): State<Arc<McpServer>>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, "SSE not implemented yet")
}
