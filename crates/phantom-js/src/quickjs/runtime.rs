//! QuickJS runtime wrapper for Phantom Engine.
//!
//! Implements Decision D-08: one isolate per agent task.
//! When the task completes, the entire runtime is dropped ("burn it down").

use rquickjs::{CatchResultExt, Context, Runtime};
use std::time::Duration;

/// Errors produced by the JavaScript engine.
///
/// Matches the `JsError` hierarchy from `references/error-types.md`.
#[derive(thiserror::Error, Debug)]
pub enum JsError {
    /// An uncaught JavaScript exception with message and stack trace.
    #[error("uncaught exception: {message}\nstack: {stack}")]
    UncaughtException { message: String, stack: String },

    /// Script execution exceeded the hard timeout.
    #[error("script execution timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// JavaScript heap out of memory.
    #[error("JavaScript heap out of memory")]
    OutOfMemory,

    /// Error during script evaluation.
    #[error("evaluation error: {0}")]
    Evaluation(String),

    /// Internal runtime error.
    #[error("runtime error: {0}")]
    Runtime(String),
}

/// Isolated QuickJS runtime — one per agent task.
///
/// Decision D-08: never reuse across tasks. Create fresh, execute scripts,
/// then call `dispose()` to burn it down.
pub struct QuickJsRuntime {
    runtime: Runtime,
    context: Context,
}

impl QuickJsRuntime {
    /// Create a new isolated QuickJS runtime.
    ///
    /// Each call creates a fresh isolate with no shared state.
    /// Decision D-08: never reuse runtimes across agent tasks.
    ///
    /// # Errors
    ///
    /// Returns `JsError::Runtime` if the QuickJS engine fails to initialize.
    #[tracing::instrument]
    pub fn new() -> Result<Self, JsError> {
        let runtime = Runtime::new()
            .map_err(|e| JsError::Runtime(format!("failed to create runtime: {e}")))?;

        // Set memory limit to 64MB to prevent runaway scripts
        runtime.set_memory_limit(64 * 1024 * 1024);
        // Set max stack size to 1MB
        runtime.set_max_stack_size(1024 * 1024);

        let context = Context::full(&runtime)
            .map_err(|e| JsError::Runtime(format!("failed to create context: {e}")))?;

        Ok(Self { runtime, context })
    }

    /// Execute a JavaScript script and return the result as a JSON value.
    ///
    /// Applies a hard 10-second timeout. If the script does not complete
    /// within that window, returns `JsError::Timeout`.
    ///
    /// # Arguments
    ///
    /// * `script` - JavaScript source code to evaluate.
    /// * `label` - Human-readable label for tracing/debugging.
    ///
    /// # Errors
    ///
    /// Returns `JsError::Timeout` if execution exceeds 10 seconds.
    /// Returns `JsError::UncaughtException` for uncaught JS exceptions.
    /// Returns `JsError::Evaluation` for eval-time errors.
    #[tracing::instrument(skip(self, script))]
    pub fn execute(&self, script: &str, label: &str) -> Result<serde_json::Value, JsError> {
        let label_owned = label.to_string();
        let script_owned = script.to_string();

        // Set an interrupt handler that fires after 10 seconds
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(10);
        self.runtime
            .set_interrupt_handler(Some(Box::new(move || start.elapsed() > timeout)));

        let result = self.context.with(|ctx| {
            let val: Result<rquickjs::Value, _> = ctx.eval(script_owned.as_bytes());

            match val.catch(&ctx) {
                Ok(v) => convert_js_value_to_json(&ctx, &v),
                Err(err) => {
                    let (message, stack) = extract_exception_info(&ctx, &err);
                    // Check if this was an interrupt (timeout)
                    if message.contains("interrupted") {
                        Err(JsError::Timeout { timeout_ms: 10_000 })
                    } else {
                        Err(JsError::UncaughtException { message, stack })
                    }
                }
            }
        });

        // Clear the interrupt handler after execution
        self.runtime.set_interrupt_handler(None);

        tracing::debug!(label = label_owned, "JS execution completed");
        result
    }

    /// Inject a JavaScript string into the global scope without returning a value.
    ///
    /// Used for injecting browser shims before page scripts run.
    ///
    /// # Arguments
    ///
    /// * `script` - JavaScript source code to inject.
    /// * `label` - Human-readable label for tracing/debugging.
    ///
    /// # Errors
    ///
    /// Returns `JsError::Evaluation` if the script has syntax errors.
    #[tracing::instrument(skip(self, script))]
    pub fn inject(&self, script: &str, label: &str) -> Result<(), JsError> {
        let script_owned = script.to_string();
        self.context.with(|ctx| {
            let result: Result<rquickjs::Value, _> = ctx.eval(script_owned.as_bytes());
            match result.catch(&ctx) {
                Ok(_) => {
                    tracing::debug!(label, "JS injection completed");
                    Ok(())
                }
                Err(err) => {
                    let (message, _stack) = extract_exception_info(&ctx, &err);
                    Err(JsError::Evaluation(message))
                }
            }
        })
    }

    /// Provide access to the context for registering classes and globals.
    ///
    /// This is used internally to set up DOM bindings before script execution.
    pub fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Context) -> R,
    {
        f(&self.context)
    }

    /// Explicitly dispose the runtime after task completion.
    ///
    /// Decision D-08: burn it down. Consumes self, dropping the entire
    /// QuickJS isolate and freeing all associated memory.
    #[tracing::instrument(skip(self))]
    pub fn dispose(self) {
        tracing::debug!("QuickJS runtime disposed (burn it down)");
        drop(self);
    }
}

/// Convert a QuickJS value to a serde_json::Value.
fn convert_js_value_to_json<'js>(
    ctx: &rquickjs::Ctx<'js>,
    val: &rquickjs::Value<'js>,
) -> Result<serde_json::Value, JsError> {
    if val.is_undefined() || val.is_null() {
        return Ok(serde_json::Value::Null);
    }
    if val.is_bool() {
        let b: bool = val
            .get()
            .map_err(|e| JsError::Evaluation(format!("bool conversion: {e}")))?;
        return Ok(serde_json::Value::Bool(b));
    }
    if val.is_int() {
        let i: i32 = val
            .get()
            .map_err(|e| JsError::Evaluation(format!("int conversion: {e}")))?;
        return Ok(serde_json::json!(i));
    }
    if val.is_float() {
        let f: f64 = val
            .get()
            .map_err(|e| JsError::Evaluation(format!("float conversion: {e}")))?;
        return Ok(serde_json::json!(f));
    }
    if val.is_string() {
        let s: String = val
            .get()
            .map_err(|e| JsError::Evaluation(format!("string conversion: {e}")))?;
        return Ok(serde_json::Value::String(s));
    }
    // For objects/arrays, stringify via JSON.stringify
    let json_global: rquickjs::Object = ctx
        .globals()
        .get("JSON")
        .map_err(|e| JsError::Evaluation(format!("JSON global: {e}")))?;
    let stringify: rquickjs::Function = json_global
        .get("stringify")
        .map_err(|e| JsError::Evaluation(format!("JSON.stringify: {e}")))?;
    let json_str: String = stringify
        .call((val.clone(),))
        .map_err(|e| JsError::Evaluation(format!("stringify call: {e}")))?;
    serde_json::from_str(&json_str).map_err(|e| JsError::Evaluation(format!("JSON parse: {e}")))
}

/// Extract exception message and stack from a CaughtError.
fn extract_exception_info(
    _ctx: &rquickjs::Ctx<'_>,
    err: &rquickjs::CaughtError<'_>,
) -> (String, String) {
    let message = format!("{err}");
    let stack = String::new();
    (message, stack)
}
