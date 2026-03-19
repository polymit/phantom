use phantom_mcp::McpServer;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_mcp_navigate_and_get_scene_graph() {
    // Start MCP server on port 18080
    let server = Arc::new(McpServer::new(vec!["test-key-123".to_string()]));
    let addr = "127.0.0.1:18081"; // Using 18081 to avoid conflicts if another test runs on 18080

    let server_clone = server.clone();
    tokio::spawn(async move { server_clone.start(18081).await });
    tokio::time::sleep(Duration::from_millis(500)).await; // give it time to bind

    // Create session
    let client = reqwest::Client::new();

    // Call browser_navigate
    let nav_response = client
        .post(format!("http://{}/mcp", addr))
        .header("X-API-Key", "test-key-123")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "browser_navigate",
                "arguments": { "url": "http://127.0.0.1:18081/mock" }
            },
            "id": 1
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        nav_response.status(),
        200,
        "Navigation request should succeed"
    );
    let nav_result: serde_json::Value = nav_response.json().await.unwrap();

    // Ensure success was true
    assert_eq!(
        nav_result["result"]["success"].as_bool(),
        Some(true),
        "Navigation tool should return success: true"
    );
    let session_id = nav_result["result"]["session_id"]
        .as_str()
        .expect("Session ID must be returned");

    // Call browser_get_scene_graph
    let sg_response = client
        .post(format!("http://{}/mcp", addr))
        .header("X-API-Key", "test-key-123")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "browser_get_scene_graph",
                "arguments": {},
                "session_id": session_id
            },
            "id": 2
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        sg_response.status(),
        200,
        "Scene graph request should succeed"
    );
    let sg_result: serde_json::Value = sg_response.json().await.unwrap();

    let scene_graph = sg_result["result"]["scene_graph"]
        .as_str()
        .expect("CCT Scene graph must be a string");
    let node_count = sg_result["result"]["node_count"]
        .as_u64()
        .expect("Node count must be an integer");

    assert!(!scene_graph.is_empty(), "Scene graph must not be empty");
    assert!(node_count > 0, "Node count should be greater than 0");

    // Test auth failure
    let auth_fail_response = client
        .post(format!("http://{}/mcp", addr))
        .header("X-API-Key", "wrong-key")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "browser_navigate",
                "arguments": { "url": "https://example.com" }
            },
            "id": 3
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        auth_fail_response.status(),
        401,
        "Should reject invalid API keys"
    );
}
