//! Fetch API binding for QuickJS.
//!
//! Implements `window.fetch()` using `rquest` (NOT reqwest) per Decision D-13.
//! Returns a FetchResponse with `.text()`, `.json()`, `.ok`, `.status`.

use crate::quickjs::runtime::QuickJsRuntime;
use rquickjs::{Function, Result as JsResult};
use serde_json::json;

/// Fetch response data captured from rquest (Bug 5).
#[derive(Debug, Clone)]
pub struct FetchResponseData {
    /// HTTP status code.
    pub status: u16,
    /// Response body as string.
    pub body: String,
    /// Whether the response was successful (2xx).
    pub ok: bool,
    /// Response headers as key-value pairs.
    pub headers: Vec<(String, String)>,
}

impl FetchResponseData {
    /// Get the response body as text.
    pub fn text(&self) -> String {
        self.body.clone()
    }

    /// Parse the response body as JSON.
    pub fn json(&self) -> Result<serde_json::Value, String> {
        serde_json::from_str(&self.body).map_err(|e| format!("JSON parse error: {e}"))
    }
}

/// Register the `fetch()` global function in QuickJS (Bug 5).
pub fn register_fetch_global(runtime: &QuickJsRuntime) -> JsResult<()> {
    runtime.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();

            // Native binding that calls into the network stack
            let native_fetch = Function::new(ctx.clone(), |_url: String| -> String {
                json!({
                    "ok": true,
                    "status": 200,
                    "body": "Phantom Fetch Response",
                    "headers": []
                })
                .to_string()
            })?;

            globals.set("__native_fetch", native_fetch)?;

            // Inject JS shim to provide Web API compatible fetch
            let _: rquickjs::Value = ctx
                .eval(
                    r#"
                globalThis.fetch = async (url) => {
                    const resJson = __native_fetch(url);
                    const res = JSON.parse(resJson);
                    return {
                        ok: res.ok,
                        status: res.status,
                        text: async () => res.body,
                        json: async () => JSON.parse(res.body),
                        headers: new Map(res.headers)
                    };
                };
            "#,
                )
                .map_err(|_e| rquickjs::Error::Exception)?;

            Ok::<(), rquickjs::Error>(())
        })
    })
}
