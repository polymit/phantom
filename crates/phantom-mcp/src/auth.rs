//! Authentication middleware for the Phantom Engine MCP Server.

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashSet;
use std::sync::Arc;

/// Reject a request with a JSON 401 Unauthorized response.
fn reject_unauthorized() -> Response {
    let body = serde_json::json!({
        "error": {
            "code": "unauthorized",
            "message": "Valid X-API-Key header is required"
        }
    });
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

use axum::extract::State;

/// Extractor for checking API keys against the server's allowed set.
#[axum::debug_middleware]
pub async fn require_auth(
    State(api_keys): State<Arc<HashSet<String>>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // If no API keys are configured, auth is disabled (dev mode)
    if api_keys.is_empty() {
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim());

    if let Some(key) = auth_header {
        if api_keys.contains(key) {
            return next.run(request).await;
        }
    }

    reject_unauthorized()
}
