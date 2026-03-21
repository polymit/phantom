//! Perception Tools.
//! Includes scene graph retrieval, snapshots, and JS evaluation.

use phantom_js::processor::JsPageProcessor;
use phantom_serializer;
use serde_json::{json, Value};
use std::sync::Arc;

use phantom_js::quickjs::runtime::QuickJsRuntime;
use phantom_js::quickjs::runtime::JsError as QuickJsError;

use crate::errors::{BrowserError, JsError, InternalError};
use crate::server::McpSession;

pub async fn browser_get_scene_graph(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let format = arguments
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("cct");

    // Ensure we have a DOM in the session (Bug 1)
    let dom = session.dom.as_ref().ok_or_else(|| {
        BrowserError::Internal(crate::errors::InternalError::ChannelSend(
            "No active page. Call browser_navigate first.".to_string(),
        ))
    })?;

    let bounds = session.bounds.as_ref().ok_or_else(|| {
        BrowserError::Internal(crate::errors::InternalError::ChannelSend(
            "Page layout not computed.".to_string(),
        ))
    })?;

    let scene_graph = if format == "json" {
        // Fallback or JSON implementation if specified
        serde_json::to_string(&json!({ "type": "document", "node_count": dom.node_count() }))
            .unwrap()
    } else {
        // REAL CCT serialization (Bug 1b)
        let serializer = phantom_serializer::HeadlessSerializer::default();
        serializer.serialize(dom, bounds)
    };

    Ok(json!({
        "scene_graph": scene_graph,
        "node_count": dom.node_count()
    }))
}

pub async fn browser_snapshot(
    _arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    // 1px transparent PNG base64 placeholder
    let placeholder = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    Ok(json!({
        "image": placeholder
    }))
}

pub async fn browser_evaluate(
    arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let script = arguments
        .get("script")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Internal(InternalError::ChannelSend(
                "Missing 'script' argument".to_string(),
            ))
        })?
        .to_string();

    let runtime = QuickJsRuntime::new().map_err(|e| {
        BrowserError::JavaScript(JsError::Evaluation(e.to_string()))
    })?;

    let result = runtime
        .execute(&script, "browser_evaluate")
        .map_err(|e| match e {
            QuickJsError::Timeout { timeout_ms } => {
                BrowserError::JavaScript(JsError::Timeout { timeout_ms })
            }
            QuickJsError::UncaughtException { message, stack } => {
                BrowserError::JavaScript(JsError::UncaughtException { message, stack })
            }
            other => BrowserError::JavaScript(JsError::Evaluation(other.to_string())),
        })?;

    runtime.dispose();

    Ok(json!({ "result": result }))
}
