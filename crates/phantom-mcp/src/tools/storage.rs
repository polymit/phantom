//! Storage Tools.
//! Cookie management.

use phantom_js::processor::JsPageProcessor;
use serde_json::{json, Value};
use std::sync::Arc;
use url;

use crate::errors::BrowserError;
use crate::server::McpSession;

pub async fn browser_get_cookies(
    _arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let jar = session.cookie_jar.read();
    let cookies: Vec<_> = jar
        .iter_any()
        .map(|c| {
            json!({
                "name": c.name(),
                "value": c.value(),
                "domain": c.domain(),
                "path": c.path(),
                "secure": c.secure(),
                "http_only": c.http_only(),
            })
        })
        .collect();

    Ok(json!({ "cookies": cookies }))
}

pub async fn browser_set_cookie(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let name = arguments
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Internal(crate::errors::InternalError::ChannelSend(
                "Missing 'name'".to_string(),
            ))
        })?;
    let value = arguments
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Internal(crate::errors::InternalError::ChannelSend(
                "Missing 'value'".to_string(),
            ))
        })?;

    let url = session.active_url.as_deref().unwrap_or("https://localhost");
    let parsed_url = url::Url::parse(url).map_err(|e| {
        BrowserError::Internal(crate::errors::InternalError::ChannelSend(format!(
            "Invalid URL: {e}"
        )))
    })?;

    let cookie_str = format!("{}={}", name, value);
    let mut jar = session.cookie_jar.write();
    jar.parse(&cookie_str, &parsed_url).map_err(|e| {
        BrowserError::Internal(crate::errors::InternalError::ChannelSend(format!(
            "Cookie parse error: {e}"
        )))
    })?;

    Ok(json!({ "success": true }))
}

pub async fn browser_clear_cookies(
    _arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let mut jar = session.cookie_jar.write();
    *jar = cookie_store::CookieStore::default();
    Ok(json!({ "success": true }))
}
