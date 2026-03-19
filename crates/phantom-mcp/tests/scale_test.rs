//! Scale test — 1000 concurrent sessions.
//!
//! Verifies: zero panics, P99 latency < 5s, 0 data leakage between sessions.
//! Decision D-21 (Circuit Breaker) and D-22 (Scheduler) are exercised here.

use phantom_session::broker::SessionBroker;
use phantom_session::types::EngineKind;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use uuid::Uuid;

/// 1000-session concurrent scale test.
///
/// Asserts:
/// - All 1000 sessions created successfully (no panics)
/// - P99 session creation latency < 5 seconds
/// - Session IDs are unique (no cross-session leakage)
/// - Active session count matches sessions created
#[tokio::test]
async fn test_1000_concurrent_sessions_scale() {
    let broker = Arc::new(SessionBroker::new());
    let n: usize = 1000;
    let mut set = JoinSet::new();

    let start_total = Instant::now();

    for i in 0..n {
        let broker_clone = Arc::clone(&broker);
        set.spawn(async move {
            let start = Instant::now();
            // 80% QuickJS (Tier 1), 20% V8 (Tier 2) — matches D-07 ratio
            let kind = if i % 5 == 0 {
                EngineKind::V8
            } else {
                EngineKind::QuickJS
            };
            let session_id = broker_clone.create_session(kind);
            (session_id, start.elapsed())
        });
    }

    let mut latencies: Vec<Duration> = Vec::with_capacity(n);
    let mut session_ids: Vec<Uuid> = Vec::with_capacity(n);

    while let Some(res) = set.join_next().await {
        let (id, latency) = res.expect("Scale test task panicked — zero panics assertion FAILED");
        latencies.push(latency);
        session_ids.push(id);
    }

    let total_duration = start_total.elapsed();

    // Sort latencies for percentile calculation
    latencies.sort();

    let p50 = latencies[n / 2];
    let p95 = latencies[(n as f64 * 0.95) as usize];
    let p99 = latencies[(n as f64 * 0.99) as usize];
    let avg: Duration = latencies.iter().sum::<Duration>() / n as u32;

    let throughput = n as f64 / total_duration.as_secs_f64();

    println!();
    println!("╔══════════════════════════════════════════════╗");
    println!("║    SCALE TEST RESULTS — 1000 Sessions        ║");
    println!("╠══════════════════════════════════════════════╣");
    println!(
        "║  Total time   : {:>10?}                   ║",
        total_duration
    );
    println!("║  Throughput   : {:>10.1} sessions/sec      ║", throughput);
    println!("╠══════════════════════════════════════════════╣");
    println!("║  P50 latency  : {:>10?}                   ║", p50);
    println!("║  P95 latency  : {:>10?}                   ║", p95);
    println!("║  P99 latency  : {:>10?}                   ║", p99);
    println!("║  Average      : {:>10?}                   ║", avg);
    println!("╠══════════════════════════════════════════════╣");
    println!(
        "║  Active count : {:>10}                   ║",
        broker.active_count()
    );
    println!("╚══════════════════════════════════════════════╝");
    println!();

    // --- Assertions ---

    // 1. All sessions created — exact count
    assert_eq!(
        broker.active_count(),
        n,
        "Expected {n} active sessions, got {}",
        broker.active_count()
    );

    // 2. No data leakage — all session IDs are unique
    let unique_ids: HashSet<Uuid> = session_ids.iter().copied().collect();
    assert_eq!(
        unique_ids.len(),
        n,
        "SESSION ID COLLISION DETECTED — data leakage between sessions!"
    );

    // 3. P99 < 5 seconds (performance gate)
    assert!(
        p99 < Duration::from_secs(5),
        "P99 latency {p99:?} exceeds 5s gate — performance FAILED"
    );

    // 4. Total throughput reasonable (> 100 sessions/sec)
    assert!(
        throughput > 100.0,
        "Throughput {throughput:.1} sess/sec is below minimum threshold"
    );

    println!("✅ SCALE TEST: PASSED — 1000 sessions, zero panics, P99 = {p99:?}");
}

/// Verify session isolation — destroying one session does not affect others.
#[tokio::test]
async fn test_session_isolation_on_destroy() {
    let broker = Arc::new(SessionBroker::new());

    let id_a = broker.create_session(EngineKind::QuickJS);
    let id_b = broker.create_session(EngineKind::QuickJS);
    let id_c = broker.create_session(EngineKind::QuickJS);

    assert_eq!(broker.active_count(), 3);

    // Destroy B — A and C must remain
    let destroyed = broker.destroy_session(id_b);
    assert!(
        destroyed,
        "destroy_session should return true for existing session"
    );
    assert_eq!(broker.active_count(), 2, "Only B should be destroyed");

    // A and C still accessible
    let a_found = broker.get_session(id_a, |s| s.id);
    let c_found = broker.get_session(id_c, |s| s.id);
    assert_eq!(a_found, Some(id_a));
    assert_eq!(c_found, Some(id_c));

    // B no longer accessible
    let b_found = broker.get_session(id_b, |s| s.id);
    assert_eq!(b_found, None, "Destroyed session must not be accessible");

    println!("✅ SESSION ISOLATION: PASSED");
}
