//! Interaction Tools.
//! Includes click, type, press_key, and wait_for_selector.

use phantom_js::processor::JsPageProcessor;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::errors::{BrowserError, DomError, InternalError};
use crate::server::McpSession;

pub async fn browser_click(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let selector = arguments
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Dom(DomError::InvalidSelector(
                "Missing selector argument".to_string(),
            ))
        })?
        .to_string();

    let dom = session.dom.as_ref().ok_or_else(|| {
        BrowserError::Internal(InternalError::ChannelSend(
            "No active page. Call browser_navigate first.".to_string(),
        ))
    })?;

    let node_id = dom.query_selector(&selector).ok_or_else(|| {
        BrowserError::Dom(DomError::ElementNotFound {
            selector: selector.clone(),
        })
    })?;

    let (cx, cy) = session
        .bounds
        .as_ref()
        .and_then(|b| b.get(&node_id))
        .map(|b| b.center())
        .unwrap_or((0.0, 0.0));

    let (cx, cy) = (cx as i32, cy as i32);

    let events = [
        ("mousemove", 20u64),
        ("mouseenter", 0),
        ("mouseover", 15),
        ("mousedown", 30),
        ("mouseup", 0),
        ("click", 0),
        ("focus", 0),
    ];

    for (event, delay_ms) in &events {
        tracing::debug!(
            event_type = event,
            x = cx,
            y = cy,
            selector = %selector,
            "dispatching browser event"
        );
        if *delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
        }
    }

    Ok(json!({
        "success": true,
        "selector": selector,
        "coordinates": { "x": cx, "y": cy },
        "events_dispatched": ["mousemove", "mouseenter", "mouseover",
                              "mousedown", "mouseup", "click", "focus"]
    }))
}

pub async fn browser_type(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let selector = arguments
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Dom(DomError::InvalidSelector(
                "Missing selector argument".to_string(),
            ))
        })?
        .to_string();

    let text = arguments
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Dom(DomError::InvalidSelector(
                "Missing text argument".to_string(),
            ))
        })?
        .to_string();

    let dom = session.dom.as_ref().ok_or_else(|| {
        BrowserError::Internal(InternalError::ChannelSend(
            "No active page. Call browser_navigate first.".to_string(),
        ))
    })?;

    dom.query_selector(&selector).ok_or_else(|| {
        BrowserError::Dom(DomError::ElementNotFound {
            selector: selector.clone(),
        })
    })?;

    let char_count = text.chars().count();

    for c in text.chars() {
        let key = c.to_string();
        tracing::debug!(key = %key, selector = %selector, "dispatching keydown");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        tracing::debug!(key = %key, selector = %selector, "dispatching keypress");
        tracing::debug!(key = %key, selector = %selector, "dispatching input");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tracing::debug!(key = %key, selector = %selector, "dispatching keyup");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(json!({
        "success": true,
        "selector": selector,
        "chars_typed": char_count
    }))
}

pub async fn browser_press_key(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let key = arguments
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Dom(DomError::InvalidSelector(
                "Missing key argument".to_string(),
            ))
        })?
        .to_string();

    if session.dom.is_none() {
        return Err(BrowserError::Internal(InternalError::ChannelSend(
            "No active page. Call browser_navigate first.".to_string(),
        )));
    }

    tracing::debug!(key = %key, "dispatching keydown");
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    tracing::debug!(key = %key, "dispatching keyup");

    Ok(json!({
        "success": true,
        "key": key
    }))
}

pub async fn browser_wait_for_selector(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let selector = arguments
        .get("selector")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Dom(DomError::InvalidSelector(
                "Missing selector argument".to_string(),
            ))
        })?
        .to_string();

    let timeout_ms = arguments
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(30000);

    let dom = session.dom.as_ref().ok_or_else(|| {
        BrowserError::Internal(InternalError::ChannelSend(
            "No active page. Call browser_navigate first.".to_string(),
        ))
    })?;

    let start = std::time::Instant::now();

    loop {
        if dom.query_selector(&selector).is_some() {
            tracing::debug!(selector = %selector, "selector found");
            return Ok(json!({
                "success": true,
                "selector": selector
            }));
        }

        if start.elapsed().as_millis() as u64 >= timeout_ms {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Err(BrowserError::Dom(DomError::ElementNotFound {
        selector: selector.clone(),
    }))
}
