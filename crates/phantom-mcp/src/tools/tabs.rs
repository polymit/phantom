//! Tab Management Tools.

use phantom_js::processor::JsPageProcessor;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::errors::{BrowserError, SessionError};
use crate::server::{McpSession, TabState};

pub async fn browser_new_tab(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let url = arguments
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("about:blank")
        .to_string();

    let tab_id = Uuid::new_v4().to_string();

    session.tabs.insert(
        tab_id.clone(),
        TabState {
            id: tab_id.clone(),
            url,
            title: "New Tab".to_string(),
        },
    );

    session.active_tab = tab_id.clone();

    Ok(json!({ "tabId": tab_id }))
}

pub async fn browser_switch_tab(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let tab_id = arguments
        .get("tabId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrowserError::Internal(crate::errors::InternalError::ChannelSend(
                "Missing 'tabId' argument".to_string(),
            ))
        })?;

    if !session.tabs.contains_key(tab_id) {
        return Err(BrowserError::Session(SessionError::TabNotFound {
            tab_id: tab_id.to_string(),
        }));
    }

    session.active_tab = tab_id.to_string();
    if let Some(tab) = session.tabs.get(tab_id) {
        session.active_url = Some(tab.url.clone());
    }

    Ok(json!({ "success": true }))
}

pub async fn browser_list_tabs(
    _arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let tabs: Vec<Value> = session
        .tabs
        .values()
        .map(|t| {
            json!({
                "tabId": t.id,
                "url": t.url,
                "title": t.title
            })
        })
        .collect();

    Ok(json!({ "tabs": tabs }))
}

pub async fn browser_close_tab(
    arguments: Value,
    session: &mut McpSession,
    _engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    let tab_id = arguments
        .get("tabId")
        .and_then(|v| v.as_str())
        .unwrap_or(&session.active_tab)
        .to_string();

    if session.tabs.remove(&tab_id).is_none() {
        return Err(BrowserError::Session(SessionError::TabNotFound { tab_id }));
    }

    // fallback active tab if we closed the active one
    if session.active_tab == tab_id {
        if let Some(first_key) = session.tabs.keys().next() {
            session.active_tab = first_key.clone();
            if let Some(tab) = session.tabs.get(first_key) {
                session.active_url = Some(tab.url.clone());
            }
        } else {
            // User closed the last tab, maybe create a blank one or leave empty
            let fallback_id = Uuid::new_v4().to_string();
            session.tabs.insert(
                fallback_id.clone(),
                TabState {
                    id: fallback_id.clone(),
                    url: "about:blank".to_string(),
                    title: "New Tab".to_string(),
                },
            );
            session.active_tab = fallback_id;
            session.active_url = Some("about:blank".to_string());
        }
    }

    Ok(json!({ "success": true }))
}
