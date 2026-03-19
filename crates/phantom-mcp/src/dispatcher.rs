//! Tool Dispatcher for the Phantom Engine MCP Server.

use phantom_js::processor::JsPageProcessor;
use serde_json::Value;
use std::sync::Arc;

use crate::errors::BrowserError;
use crate::server::McpSession;
use crate::tools;

/// Route an incoming tool call to the appropriate handler.
#[tracing::instrument(skip(session, engine, arguments))]
pub async fn dispatch(
    tool_name: &str,
    arguments: Value,
    session: &mut McpSession,
    engine: &Arc<JsPageProcessor>,
) -> Result<Value, BrowserError> {
    match tool_name {
        // Navigation (4)
        "browser_navigate" => tools::navigation::browser_navigate(arguments, session, engine).await,
        "browser_go_back" => tools::navigation::browser_go_back(arguments, session, engine).await,
        "browser_go_forward" => {
            tools::navigation::browser_go_forward(arguments, session, engine).await
        }
        "browser_refresh" => tools::navigation::browser_refresh(arguments, session, engine).await,

        // Interaction (4)
        "browser_click" => tools::interaction::browser_click(arguments, session, engine).await,
        "browser_type" => tools::interaction::browser_type(arguments, session, engine).await,
        "browser_press_key" => {
            tools::interaction::browser_press_key(arguments, session, engine).await
        }
        "browser_wait_for_selector" => {
            tools::interaction::browser_wait_for_selector(arguments, session, engine).await
        }

        // Perception (3)
        "browser_get_scene_graph" => {
            tools::perception::browser_get_scene_graph(arguments, session, engine).await
        }
        "browser_snapshot" => tools::perception::browser_snapshot(arguments, session, engine).await,
        "browser_evaluate" => tools::perception::browser_evaluate(arguments, session, engine).await,

        // Tabs (4)
        "browser_new_tab" => tools::tabs::browser_new_tab(arguments, session, engine).await,
        "browser_switch_tab" => tools::tabs::browser_switch_tab(arguments, session, engine).await,
        "browser_list_tabs" => tools::tabs::browser_list_tabs(arguments, session, engine).await,
        "browser_close_tab" => tools::tabs::browser_close_tab(arguments, session, engine).await,

        // Storage (3)
        "browser_get_cookies" => {
            tools::storage::browser_get_cookies(arguments, session, engine).await
        }
        "browser_set_cookie" => {
            tools::storage::browser_set_cookie(arguments, session, engine).await
        }
        "browser_clear_cookies" => {
            tools::storage::browser_clear_cookies(arguments, session, engine).await
        }

        // Session (3)
        "browser_subscribe_dom" => {
            tools::session::browser_subscribe_dom(arguments, session, engine).await
        }
        "browser_session_snapshot" => {
            tools::session::browser_session_snapshot(arguments, session, engine).await
        }
        "browser_session_clone" => {
            tools::session::browser_session_clone(arguments, session, engine).await
        }

        _ => Err(BrowserError::Internal(
            crate::errors::InternalError::ChannelSend(format!("Unknown tool: {}", tool_name)),
        )),
    }
}
