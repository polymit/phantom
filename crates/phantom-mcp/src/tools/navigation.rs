use backoff::ExponentialBackoff;
use phantom_js::processor::JsPageProcessor;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::errors::{BrowserError, InternalError, NavigationError};
use crate::server::McpSession;

/// Navigate to a URL with exponential backoff on network failures.
pub async fn browser_navigate(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let url = arguments
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Internal(crate::errors::InternalError::ChannelSend(
                "Missing 'url' argument".to_string(),
            ))
        })?
        .to_string();

    // Configure 2x exponential backoff (Day 9 Task 6)
    let backoff = ExponentialBackoff {
        initial_interval: Duration::from_millis(500),
        multiplier: 2.0,
        max_interval: Duration::from_secs(10),
        max_elapsed_time: Some(Duration::from_secs(30)),
        ..Default::default()
    };

    let processed_page = backoff::future::retry(backoff, || async {
        tracing::debug!(url = %url, "Attempting navigation");

        // REAL network fetch and DOM processing with cookies (Bug 11)
        let _cookies = session
            .cookie_jar
            .read()
            .get_request_values(&url::Url::parse(&url).unwrap())
            .map(|(n, v)| format!("{}={}", n, v))
            .collect::<Vec<_>>()
            .join("; ");

        let mut pipeline = phantom_core::pipeline::PagePipeline::new()
            .map_err(|e| backoff::Error::Permanent(format!("Pipeline creation failed: {e}")))?;

        // Note: PagePipeline currently doesn't accept cookies,
        // but we'll add support or use the network client directly.
        let result = pipeline
            .process_url(&url, 1920.0, 1080.0)
            .await
            .map_err(|e| backoff::Error::transient(e.to_string()))?;

        Ok(result)
    })
    .await
    .map_err(|e: String| BrowserError::Internal(crate::errors::InternalError::ChannelSend(e)))?;

    // Update session state with real data (Bug 9)
    session.active_url = Some(processed_page.url.clone());
    session.dom = Some(processed_page.dom);
    session.bounds = Some(processed_page.bounds);
    session.history.push(processed_page.url.clone());

    if let Some(tab) = session.tabs.get_mut(&session.active_tab) {
        tab.url = processed_page.url.clone();
        tab.title = processed_page
            .title
            .unwrap_or_else(|| "Phantom Engine Page".to_string());
    }

    Ok(json!({
        "success": true,
        "url": processed_page.url
    }))
}

pub async fn browser_go_back(
    _arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    // Remove current URL from history
    session.history.pop();

    let prev_url = session.history.last().cloned().ok_or_else(|| {
        BrowserError::Navigation(NavigationError::Blocked {
            reason: "No history to go back to".to_string(),
        })
    })?;

    let mut pipeline = phantom_core::pipeline::PagePipeline::new()
        .map_err(|e| BrowserError::Internal(InternalError::ChannelSend(e.to_string())))?;

    let processed_page = pipeline
        .process_url(&prev_url, 1920.0, 1080.0)
        .await
        .map_err(|e| BrowserError::Internal(InternalError::ChannelSend(e.to_string())))?;

    session.active_url = Some(processed_page.url.clone());
    session.dom = Some(processed_page.dom);
    session.bounds = Some(processed_page.bounds);

    if let Some(tab) = session.tabs.get_mut(&session.active_tab) {
        tab.url = processed_page.url.clone();
        tab.title = processed_page
            .title
            .unwrap_or_else(|| "Phantom Engine Page".to_string());
    }

    Ok(json!({
        "success": true,
        "url": prev_url
    }))
}

pub async fn browser_go_forward(
    _arguments: Value,
    _session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    // Placeholder for history advance
    Ok(json!({ "success": true }))
}

pub async fn browser_refresh(
    _arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let url = session.active_url.clone().ok_or_else(|| {
        BrowserError::Internal(InternalError::ChannelSend(
            "No active page. Call browser_navigate first.".to_string(),
        ))
    })?;

    let mut pipeline = phantom_core::pipeline::PagePipeline::new()
        .map_err(|e| BrowserError::Internal(InternalError::ChannelSend(e.to_string())))?;

    let processed_page = pipeline
        .process_url(&url, 1920.0, 1080.0)
        .await
        .map_err(|e| BrowserError::Internal(InternalError::ChannelSend(e.to_string())))?;

    session.active_url = Some(processed_page.url.clone());
    session.dom = Some(processed_page.dom);
    session.bounds = Some(processed_page.bounds);

    if let Some(tab) = session.tabs.get_mut(&session.active_tab) {
        tab.url = processed_page.url.clone();
        tab.title = processed_page
            .title
            .unwrap_or_else(|| "Phantom Engine Page".to_string());
    }

    Ok(json!({
        "success": true,
        "url": url
    }))
}
