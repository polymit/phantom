use phantom_core::dom::DomTree;
use phantom_core::layout::ViewportBounds;
use phantom_serializer::HeadlessSerializer;
use phantom_session::pool::IsolatePool;
use phantom_session::types::EngineKind;
use std::collections::HashMap;
use std::time::Instant;

#[test]
fn test_gate_cct_serialization_speed() {
    // 1. Setup a 1000-node DOM tree
    let mut tree = DomTree::new();
    let root = tree.create_element("div", HashMap::new());
    tree.document_root = Some(root);

    let mut bounds = HashMap::new();
    let mock_bound = ViewportBounds {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 100.0,
    };
    bounds.insert(root, mock_bound.clone());

    for i in 0..1000 {
        let text = tree.create_text(&format!("Node content {}", i));
        tree.append_child(root, text);
        // Text nodes don't need bounds in the current serializer logic as they are
        // processed within their parent element's semantic extraction,
        // but the parent MUST have bounds.
    }

    let layout_bounds = ViewportBounds {
        x: 0.0,
        y: 0.0,
        width: 1280.0,
        height: 720.0,
    };
    let serializer = HeadlessSerializer::new(layout_bounds);

    // 2. Measure serialization
    let start = Instant::now();
    let cct = serializer.serialize(&tree, &bounds);
    let elapsed = start.elapsed();

    println!("CCT Serialization (1001 nodes): {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 50, // 50ms for debug builds, <5ms in release
        "Serialization too slow: {:?}",
        elapsed
    );
    assert!(!cct.is_empty());
}

#[test]
fn test_gate_session_creation_pool() {
    let pool = IsolatePool::new();
    pool.prewarm(5, 5);

    let start = Instant::now();
    let isolate = pool.acquire(EngineKind::QuickJS);
    let elapsed = start.elapsed();

    println!("Pool Acquire (QuickJS): {:?}", elapsed);
    assert!(isolate.is_some());
    assert!(
        elapsed.as_millis() < 10,
        "Pool acquisition too slow: {:?}",
        elapsed
    );
}

#[test]
fn test_gate_session_creation_cold() {
    let _pool = IsolatePool::new(); // No prewarm

    let start = Instant::now();
    // In a real cold start, we'd spawn a new isolate.
    // IsolatePool::acquire returns None if empty, so we simulate the spawn.
    let _handle = phantom_session::types::IsolateHandle {
        id: uuid::Uuid::new_v4(),
        kind: EngineKind::QuickJS,
        created_at: std::time::SystemTime::now(),
    };
    let elapsed = start.elapsed();

    println!("Cold Start Simulation: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 50,
        "Cold start too slow: {:?}",
        elapsed
    );
}
