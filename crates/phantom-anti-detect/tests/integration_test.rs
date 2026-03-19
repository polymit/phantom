use async_trait::async_trait;
use parking_lot::Mutex;
use phantom_anti_detect::*;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

// Mock Dispatcher to record event sequences
struct MockDispatcher {
    events: Arc<Mutex<Vec<(String, u64)>>>,
}

#[async_trait]
impl EventDispatcher for MockDispatcher {
    async fn dispatch_event(
        &self,
        event_type: &str,
        target_id: u64,
        _detail: Value,
    ) -> Result<(), String> {
        self.events.lock().push((event_type.to_string(), target_id));
        Ok(())
    }
}

#[tokio::test]
async fn test_navigator_webdriver_undefined() {
    let pool = PersonaPool::new();
    let persona = pool.get_or_create(Uuid::new_v4());
    let shims = generate_js_shims(&persona);

    // Simple check that the script contains the webdriver undefined patch
    assert!(shims.contains("navigator, 'webdriver'"));
    assert!(shims.contains("() => undefined"));
}

#[tokio::test]
async fn test_hardware_concurrency_realistic() {
    let pool = PersonaPool::new();
    let persona = pool.get_or_create(Uuid::new_v4());

    // Must be 4, 8, or 16
    let val = persona.hardware_concurrency;
    assert!(val == 4 || val == 8 || val == 16);
}

#[tokio::test]
async fn test_persona_consistency() {
    let pool = PersonaPool::new();
    let id = Uuid::new_v4();

    let p1 = pool.get_or_create(id);
    let p2 = pool.get_or_create(id);

    assert_eq!(p1.user_agent, p2.user_agent);
    assert_eq!(p1.hardware_concurrency, p2.hardware_concurrency);
    assert_eq!(p1.screen_width, p2.screen_width);
}

#[tokio::test]
async fn test_click_event_sequence() {
    let dispatcher = MockDispatcher {
        events: Arc::new(Mutex::new(Vec::new())),
    };

    ActionEngine::click(&dispatcher, 42, 100, 200)
        .await
        .unwrap();

    let events = dispatcher.events.lock();
    let expected = vec![
        "mousemove",
        "mouseenter",
        "mouseover",
        "mousedown",
        "mouseup",
        "click",
        "focus",
    ];

    let event_names: Vec<String> = events.iter().map(|(n, _)| n.clone()).collect();
    assert_eq!(event_names, expected);
}

#[tokio::test]
async fn test_canvas_noise_deterministic() {
    let id = Uuid::new_v4();
    let shim1 = generate_noise_shim(id);
    let shim2 = generate_noise_shim(id);

    assert_eq!(shim1, shim2);
    assert!(shim1.contains("getNoiseBit"));
}
