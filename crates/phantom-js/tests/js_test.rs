use parking_lot::RwLock;
use phantom_core::dom::DomTree;
use phantom_js::quickjs::bindings::element::{EngineContext, ENGINE_CONTEXT};
use phantom_js::quickjs::bindings::navigator::Persona;
use phantom_js::quickjs::runtime::QuickJsRuntime;
use phantom_js::shims::generate_shims;
use std::collections::HashMap;
use std::sync::Arc;

/// CRITICAL: navigator.webdriver MUST return "undefined" (as typeof string).
/// Not "boolean", not "object". The typeof must be literally "undefined".
#[test]
fn test_webdriver_is_undefined() {
    let runtime = QuickJsRuntime::new().expect("runtime creation must succeed");

    let persona = Persona::default();
    let shims = generate_shims(&persona);
    runtime
        .inject(&shims, "shims")
        .expect("shim injection must succeed");

    let result = runtime
        .execute("typeof navigator.webdriver", "webdriver_test")
        .expect("execution must succeed");

    assert_eq!(
        result.as_str(),
        Some("undefined"),
        "CRITICAL: navigator.webdriver typeof must be 'undefined', not 'false' or 'boolean'"
    );

    runtime.dispose();
}

/// Hardware concurrency must be a realistic value: 4, 8, or 16.
/// Never 1 (suspicious). Never 128 (unrealistic).
#[test]
fn test_hardware_concurrency_realistic() {
    let runtime = QuickJsRuntime::new().expect("runtime creation must succeed");

    let persona = Persona {
        hardware_concurrency: 8,
        ..Persona::default()
    };
    let shims = generate_shims(&persona);
    runtime
        .inject(&shims, "shims")
        .expect("shim injection must succeed");

    let result = runtime
        .execute("navigator.hardwareConcurrency", "hw_test")
        .expect("execution must succeed");

    let concurrency = result.as_i64().unwrap_or(0);
    assert!(
        concurrency == 4 || concurrency == 8 || concurrency == 16,
        "hardwareConcurrency must be 4, 8, or 16, got {concurrency}"
    );

    // Validate the Persona struct itself
    assert!(
        persona.validate(),
        "Persona with hardware_concurrency=8 must be valid"
    );

    let bad_persona = Persona {
        hardware_concurrency: 1,
        ..Persona::default()
    };
    assert!(
        !bad_persona.validate(),
        "Persona with hardware_concurrency=1 must be invalid"
    );

    runtime.dispose();
}

/// Timeout protection: an infinite loop must be interrupted after 10 seconds.
/// The runtime must NOT hang forever.
#[test]
fn test_timeout_protection() {
    let runtime = QuickJsRuntime::new().expect("runtime creation must succeed");

    let start = std::time::Instant::now();
    let result = runtime.execute("while(true) {}", "infinite_loop");
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Infinite loop must trigger an error");

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    // Should be either a timeout or an interrupt error
    assert!(
        err_str.contains("timed out") || err_str.contains("interrupted"),
        "Error must indicate timeout/interrupt, got: {err_str}"
    );

    // Must complete within a reasonable time (10s timeout + some overhead)
    assert!(
        elapsed.as_secs() <= 12,
        "Must not hang forever, elapsed: {elapsed:?}"
    );

    runtime.dispose();
}

/// Burn it down: create runtime, execute, dispose, repeat.
/// Verify no panics or leaks via repeated creation/disposal.
#[test]
fn test_burn_it_down() {
    for i in 0..10 {
        let runtime = QuickJsRuntime::new().expect("runtime creation must succeed");

        let result = runtime
            .execute(&format!("1 + {i}"), "arithmetic")
            .expect("simple eval must succeed");

        let expected = 1 + i;
        assert_eq!(
            result.as_i64(),
            Some(expected as i64),
            "Expected {expected}"
        );

        // Decision D-08: burn it down
        runtime.dispose();
    }
}

/// Basic JS execution: verify the runtime can evaluate expressions.
#[test]
fn test_basic_execution() {
    let runtime = QuickJsRuntime::new().expect("runtime creation must succeed");

    let result = runtime
        .execute("2 + 3", "basic_math")
        .expect("must succeed");
    assert_eq!(result.as_i64(), Some(5));

    let result = runtime
        .execute("'hello' + ' ' + 'world'", "string_concat")
        .expect("must succeed");
    assert_eq!(result.as_str(), Some("hello world"));

    runtime.dispose();
}

/// Verify the engine context and HTMLElement arena_id bridge works.
#[tokio::test]
async fn test_engine_context_dom_bridge() {
    use phantom_js::quickjs::bindings::element::HTMLElement;

    let mut tree = DomTree::new();
    let root = tree.create_element("div", HashMap::new());
    tree.document_root = Some(root);

    let mut attrs = HashMap::new();
    attrs.insert("id".to_string(), "test-btn".to_string());
    let btn = tree.create_element("button", attrs);
    tree.append_child(root, btn);

    let text = tree.create_text("Click Me");
    tree.append_child(btn, text);

    let bounds = HashMap::new();
    let ctx = EngineContext::new(tree, bounds, "https://example.com");

    // Verify that arena IDs were assigned
    let root_id = ctx.node_to_id.get(&root).copied();
    let btn_id = ctx.node_to_id.get(&btn).copied();
    assert!(root_id.is_some(), "Root must have an arena ID");
    assert!(btn_id.is_some(), "Button must have an arena ID");

    // Set the context and test HTMLElement methods
    let ctx_shared = Arc::new(RwLock::new(ctx));
    ENGINE_CONTEXT
        .scope(Some(ctx_shared), async move {
            let elem = HTMLElement {
                arena_id: btn_id.unwrap(),
            };
            assert_eq!(elem.tag_name(), "BUTTON");
            assert_eq!(elem.text_content(), "Click Me");
            assert!(elem.has_attribute("id"));
            assert_eq!(elem.get_attribute("id"), Some("test-btn".to_string()));
        })
        .await;
}

/// Verify document binding methods.
#[tokio::test]
async fn test_document_binding() {
    use phantom_js::quickjs::bindings::document::DocumentBinding;
    use phantom_js::quickjs::bindings::element::{EngineContext, ENGINE_CONTEXT};

    let mut tree = DomTree::new();
    let root = tree.create_element("html", HashMap::new());
    tree.document_root = Some(root);

    let mut body_attrs = HashMap::new();
    body_attrs.insert("id".to_string(), "main-body".to_string());
    let body = tree.create_element("body", body_attrs);
    tree.append_child(root, body);

    let bounds = HashMap::new();
    let ctx = EngineContext::new(tree, bounds, "https://example.com");
    let ctx_shared = Arc::new(RwLock::new(ctx));
    ENGINE_CONTEXT
        .scope(Some(ctx_shared), async move {
            // Test getElementById
            let found = DocumentBinding::get_element_by_id("main-body");
            assert!(found.is_some(), "Must find element by id");
            assert_eq!(found.unwrap().tag_name(), "BODY");

            // Test createElement
            let new_elem = DocumentBinding::create_element("div");
            assert!(new_elem.is_some(), "Must create element");
            assert_eq!(new_elem.unwrap().tag_name(), "DIV");

            // Test title
            DocumentBinding::set_title("Test Page");
            assert_eq!(DocumentBinding::title(), "Test Page");

            // Test readyState
            assert_eq!(DocumentBinding::ready_state(), "complete");
        })
        .await;
}
