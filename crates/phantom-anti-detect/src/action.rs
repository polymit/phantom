use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

/// Trait for dispatching raw DOM events into the engine's JS context.
#[async_trait::async_trait]
pub trait EventDispatcher: Send + Sync {
    async fn dispatch_event(
        &self,
        event_type: &str,
        target_id: u64,
        detail: Value,
    ) -> Result<(), String>;
}

pub struct ActionEngine;

impl ActionEngine {
    /// Executes a realistic click sequence:
    /// mousemove -> mouseenter -> mouseover -> mousedown -> mouseup -> click -> focus
    pub async fn click<D: EventDispatcher>(
        dispatcher: &D,
        target_id: u64,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        let mouse_detail = serde_json::json!({ "x": x, "y": y });

        // 1. mousemove
        dispatcher
            .dispatch_event("mousemove", target_id, mouse_detail.clone())
            .await?;
        sleep(Duration::from_millis(20)).await;

        // 2. mouseenter
        dispatcher
            .dispatch_event("mouseenter", target_id, mouse_detail.clone())
            .await?;

        // 3. mouseover
        dispatcher
            .dispatch_event("mouseover", target_id, mouse_detail.clone())
            .await?;
        sleep(Duration::from_millis(15)).await;

        // 4. mousedown
        dispatcher
            .dispatch_event("mousedown", target_id, mouse_detail.clone())
            .await?;
        sleep(Duration::from_millis(30)).await;

        // 5. mouseup
        dispatcher
            .dispatch_event("mouseup", target_id, mouse_detail.clone())
            .await?;

        // 6. click
        dispatcher
            .dispatch_event("click", target_id, mouse_detail)
            .await?;

        // 7. focus
        dispatcher
            .dispatch_event("focus", target_id, serde_json::json!({}))
            .await?;

        Ok(())
    }

    /// Executes a realistic typing sequence per character:
    /// keydown -> keypress -> input -> keyup
    pub async fn type_text<D: EventDispatcher>(
        dispatcher: &D,
        target_id: u64,
        text: &str,
    ) -> Result<(), String> {
        for c in text.chars() {
            let key = c.to_string();
            let detail = serde_json::json!({ "key": key });

            dispatcher
                .dispatch_event("keydown", target_id, detail.clone())
                .await?;
            sleep(Duration::from_millis(10)).await;

            dispatcher
                .dispatch_event("keypress", target_id, detail.clone())
                .await?;
            dispatcher
                .dispatch_event("input", target_id, detail.clone())
                .await?;
            sleep(Duration::from_millis(20)).await;

            dispatcher
                .dispatch_event("keyup", target_id, detail)
                .await?;

            // Human-like inter-character delay
            sleep(Duration::from_millis(50)).await;
        }
        Ok(())
    }

    /// Executes a scroll/wheel event
    pub async fn scroll<D: EventDispatcher>(
        dispatcher: &D,
        target_id: u64,
        delta_y: i32,
    ) -> Result<(), String> {
        let detail = serde_json::json!({ "deltaY": delta_y });
        dispatcher
            .dispatch_event("wheel", target_id, detail)
            .await?;
        Ok(())
    }

    /// Dispatches a specialized key press (e.g. Enter, Backspace)
    pub async fn press_key<D: EventDispatcher>(
        dispatcher: &D,
        target_id: u64,
        key: &str,
    ) -> Result<(), String> {
        let detail = serde_json::json!({ "key": key });
        dispatcher
            .dispatch_event("keydown", target_id, detail.clone())
            .await?;
        sleep(Duration::from_millis(15)).await;
        dispatcher
            .dispatch_event("keyup", target_id, detail)
            .await?;
        Ok(())
    }
}
