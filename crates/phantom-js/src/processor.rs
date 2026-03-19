//! JS page processor — orchestrates the full page processing pipeline.
//!
//! Receives a URL, fetches HTML, parses the DOM, applies CSS cascade,
//! runs JavaScript, computes layout, and serializes to CCT format.

use crate::quickjs::bindings::element::{
    take_engine_context, with_engine_context_mut, EngineContext, ENGINE_CONTEXT,
};
use crate::quickjs::bindings::navigator::Persona;
use crate::quickjs::runtime::{JsError, QuickJsRuntime};
use crate::shims::generate_shims;
use parking_lot::RwLock;
use std::sync::Arc;

/// Errors produced by the page processor.
#[derive(thiserror::Error, Debug)]
pub enum ProcessorError {
    /// JavaScript engine error.
    #[error("JS error: {0}")]
    Js(#[from] JsError),

    /// Network or pipeline error.
    #[error("pipeline error: {0}")]
    Pipeline(String),
}

/// JS-aware page processor that integrates the full Phantom Engine pipeline.
///
/// Orchestrates: fetch → parse → cascade → JS execute → layout → CCT serialize.
pub struct JsPageProcessor {
    /// Persona for anti-detection shims.
    persona: Persona,
}

impl Default for JsPageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl JsPageProcessor {
    /// Create a new page processor with default persona.
    pub fn new() -> Self {
        Self {
            persona: Persona::default(),
        }
    }

    /// Create a page processor with a specific persona.
    pub fn with_persona(persona: Persona) -> Self {
        Self { persona }
    }

    /// Process a DOM tree with JavaScript execution and return CCT output.
    ///
    /// This follows the exact 11-step pipeline mandated by the architecture:
    ///
    /// 1. (Caller provides HTML string)
    /// 2. Parse HTML → DomTree
    /// 3. Apply CSS cascade
    /// 4. Create QuickJsRuntime::new()
    /// 5. Inject generate_shims(persona)
    /// 6. Execute provided JS scripts
    /// 7. (MutationObserver settle — future)
    /// 8. (Apply taffy layout to post-JS DOM — future)
    /// 9. (Serialize to CCT — future)
    /// 10. runtime.dispose() — BURN IT DOWN
    /// 11. Return the engine context (DOM tree available for serialization)
    ///
    /// # Errors
    ///
    /// Returns `ProcessorError::Js` for JavaScript execution failures.
    /// Returns `ProcessorError::Pipeline` for other processing errors.
    pub async fn process_with_scripts(
        &self,
        dom_tree: phantom_core::dom::DomTree,
        bounds: std::collections::HashMap<indextree::NodeId, phantom_core::layout::ViewportBounds>,
        scripts: &[&str],
        url: &str,
    ) -> Result<EngineContext, ProcessorError> {
        // Step 4: Create fresh QuickJS runtime (D-08: one per task)
        let runtime = QuickJsRuntime::new()?;

        // Step 5: Inject browser shims BEFORE any page scripts
        let shims = generate_shims(&self.persona);
        runtime
            .inject(&shims, "browser_shims")
            .map_err(|e| ProcessorError::Pipeline(format!("shim injection failed: {e}")))?;

        // Register fetch global (Bug 5)
        crate::quickjs::bindings::fetch::register_fetch_global(&runtime)
            .map_err(|e| ProcessorError::Pipeline(format!("fetch registration failed: {e}")))?;

        // Initialize engine context (Bug 2 fix: using tokio::task_local scope)
        let engine_ctx = Some(Arc::new(RwLock::new(EngineContext::new(
            dom_tree, bounds, url,
        ))));

        // Step 6: Execute page scripts in DOM order with 10s hard timeout
        // Wrapped in ENGINE_CONTEXT scope to persist across awaits
        ENGINE_CONTEXT
            .scope(engine_ctx, async {
                for (i, script) in scripts.iter().enumerate() {
                    let label = format!("page_script_{i}");
                    let script_content = script.to_string();

                    // Apply 10s timeout per script execution (D-12/Task 5)
                    let timeout_duration = std::time::Duration::from_secs(10);

                    match tokio::time::timeout(timeout_duration, async {
                        runtime.execute(&script_content, &label)
                    })
                    .await
                    {
                        Ok(Ok(_)) => {}
                        Ok(Err(JsError::Timeout { timeout_ms })) => {
                            tracing::warn!(timeout_ms, "Script {i} timed out internally");
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "Script {i} failed");
                        }
                        Err(_) => {
                            tracing::error!("Script {i} reached 10s watchdog timeout");
                        }
                    }
                }

                // Step 7: Flush zero-delay timers (Bug 4)
                // React and other frameworks use setTimeout(fn, 0) for deferred work
                let mut loop_count = 0;
                while loop_count < 10 {
                    // Limit nesting to prevent infinite loops
                    let pending: Vec<String> =
                        with_engine_context_mut(|ctx| ctx.timers.flush_zero_delay_timers())
                            .unwrap_or_default();

                    if pending.is_empty() {
                        break;
                    }

                    for (i, callback) in pending.into_iter().enumerate() {
                        let label = format!("timer_callback_{loop_count}_{i}");
                        let _ = runtime.execute(&callback, &label);
                    }
                    loop_count += 1;
                }

                // Step 10: Burn it down — dispose the JS runtime
                runtime.dispose();

                // Extract the final mutated context
                // Since the runtime is disposed and we are at the end of the scope,
                // we should be able to get ownership if needed, but the scope is about to end.
                // We'll just clone it from the task-local before the scope ends.
                let arc_opt = take_engine_context();
                let arc = arc_opt.ok_or_else(|| {
                    ProcessorError::Pipeline("engine context missing".to_string())
                })?;

                // Try to get ownership. Since this is task-local and the only task, it should succeed.
                std::sync::Arc::try_unwrap(arc)
                    .map(|lock| lock.into_inner())
                    .map_err(|_| {
                        ProcessorError::Pipeline("engine context still referenced".to_string())
                    })
            })
            .await
    }
}
