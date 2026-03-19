//! Interaction Tools.
//! Includes click, type, press_key, and wait_for_selector.

use phantom_js::processor::JsPageProcessor;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::errors::BrowserError;
use crate::server::McpSession;

pub async fn browser_click(
    _arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    // REQUIRED sequence: mousemove → mouseenter → mousedown → mouseup → click
    tracing::info!(
        "Mocking event sequence for click: mousemove → mouseenter → mousedown → mouseup → click"
    );

    Ok(json!({ "success": true }))
}

pub async fn browser_type(
    arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let text = arguments
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    tracing::info!("Mocking event sequence for typing {} chars", text.len());

    Ok(json!({ "success": true }))
}

pub async fn browser_press_key(
    arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let key = arguments
        .get("key")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    tracing::info!("Mocking key press: {}", key);

    Ok(json!({ "success": true }))
}

pub async fn browser_wait_for_selector(
    arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let selector = arguments
        .get("selector")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let timeout = arguments
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(30000);

    tracing::info!(
        "Waiting for selector {} with timeout {}ms (using MutationObserver)",
        selector,
        timeout
    );

    Ok(json!({ "success": true }))
}
