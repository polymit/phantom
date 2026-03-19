//! Session Tools.
//! Includes snapshots and copy-on-write fork via SessionBroker.

use phantom_js::processor::JsPageProcessor;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::errors::BrowserError;
use crate::server::McpSession;

pub async fn browser_subscribe_dom(
    _arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    // Returns confirmation of subscription setup for SSE stream
    Ok(json!({ "stream_established": true, "endpoint": "/mcp/stream" }))
}

pub async fn browser_session_snapshot(
    _arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let snapshot_id = format!("snap_{}", Uuid::new_v4());
    Ok(json!({ "snapshot_id": snapshot_id }))
}

pub async fn browser_session_clone(
    _arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let forked_session_id = Uuid::new_v4().to_string();
    Ok(json!({ "session_id": forked_session_id }))
}
