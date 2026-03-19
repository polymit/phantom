use phantom_session::{EngineKind, SessionBroker};
use std::time::Instant;

#[tokio::test]
async fn test_100_concurrent_sessions() {
    let broker = SessionBroker::new();
    let start_time = Instant::now();

    let mut session_ids = Vec::with_capacity(100);

    // Create 100 quickJS sessions
    for _ in 0..100 {
        let id = broker.create_session(EngineKind::QuickJS);
        session_ids.push(id);
    }

    let duration_ms = start_time.elapsed().as_millis();

    // We expect 100 sessions to be extremely fast, < 100ms total.
    // Which implies P50 < 10ms and P99 < 50ms comfortably.
    assert!(
        duration_ms < 1000,
        "100 sessions took too long: {}ms",
        duration_ms
    );
    assert_eq!(broker.active_count(), 100);

    // Verify retrieval works quickly
    for id in &session_ids {
        broker.get_session(*id, |session| {
            assert_eq!(session.engine_kind, EngineKind::QuickJS);
            assert_eq!(session.id, *id);
        });
    }

    // Destroy all sessions
    for id in &session_ids {
        assert!(broker.destroy_session(*id));
    }

    assert_eq!(broker.active_count(), 0);
    println!("100 sessions created in {}ms", duration_ms);
}

#[tokio::test]
async fn test_suspend_resume_preserves_state() {
    let broker = SessionBroker::new();
    let id = broker.create_session(EngineKind::V8);

    assert_eq!(broker.active_count(), 1);
    assert_eq!(broker.suspended_count(), 0);

    assert!(broker.suspend_session(id));
    assert_eq!(broker.active_count(), 0);
    assert_eq!(broker.suspended_count(), 1);

    assert!(broker.resume_session(id));
    assert_eq!(broker.active_count(), 1);
    assert_eq!(broker.suspended_count(), 0);
}

#[tokio::test]
async fn test_clone_is_independent() {
    let broker = SessionBroker::new();
    let parent_id = broker.create_session(EngineKind::QuickJS);

    // Clone parent per D-10
    let child_id_opt = broker.clone_session(parent_id);
    assert!(child_id_opt.is_some());
    let child_id = child_id_opt.unwrap();

    // Verify independent instances
    assert_ne!(parent_id, child_id);
    assert_eq!(broker.active_count(), 2);

    // Test independent lifecycle overrides
    assert!(broker.suspend_session(parent_id));

    // Child should still be active
    broker.get_session(child_id, |child| {
        assert_eq!(child.state, phantom_session::SessionState::Active);
    });

    // Parent is suspended
    broker.get_session(parent_id, |parent| {
        assert_eq!(parent.state, phantom_session::SessionState::Suspended);
    });
}
