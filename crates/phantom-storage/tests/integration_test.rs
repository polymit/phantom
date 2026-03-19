use phantom_storage::SessionStorageManager;
use phantom_storage::errors::StorageError;
use phantom_storage::snapshot::SnapshotManager;
use std::fs;
use uuid::Uuid;

fn generate_test_id() -> String {
    Uuid::new_v4().to_string()
}

fn setup_test_dir(test_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("phantom_storage_tests")
        .join(test_name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn test_full_storage_lifecycle() {
    let base_dir = setup_test_dir("lifecycle");
    let session_id = generate_test_id();

    // 1. Initialize manager
    let mgr = SessionStorageManager::new(&base_dir, &session_id, 10_000_000).expect("Setup failed");

    // 2. Test LocalStorage
    mgr.local
        .set("https://example.com", "theme", "dark")
        .unwrap();
    assert_eq!(
        mgr.local
            .get("https://example.com", "theme")
            .unwrap()
            .unwrap(),
        "dark"
    );

    // 3. Test CacheStorage
    let payload = b"Hello, World!";
    let hash = mgr.cache.put_blob(payload).unwrap();
    let retrieved = mgr.cache.get_blob(&hash).unwrap().unwrap();
    assert_eq!(payload.as_slice(), retrieved.as_slice());

    // 4. Test IndexedDB
    let conn = mgr.idb.get_connection("hash_example_com").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY)",
        (),
    )
    .unwrap();

    // 5. Cleanup
    fs::remove_dir_all(&base_dir).unwrap();
}

#[tokio::test]
async fn test_snapshot_and_restore() {
    let base_dir = setup_test_dir("snapshot_restore");
    let session_id = generate_test_id();

    // Create data
    {
        let mgr = SessionStorageManager::new(&base_dir, &session_id, 10_000_000).unwrap();
        mgr.local.set("origin", "key", "persisted_value").unwrap();
    } // Drops mgr and sled instances safely

    let snapshot_mgr = SnapshotManager::new(&base_dir);

    // Create snapshot
    let snapshot_path = snapshot_mgr
        .create_snapshot(&session_id)
        .expect("Snapshot failed");
    assert!(snapshot_path.exists());

    // Blow away isolated data
    let session_dir = base_dir.join(&session_id);
    fs::remove_dir_all(&session_dir).unwrap();
    assert!(!session_dir.exists());

    // Restore
    snapshot_mgr
        .restore_snapshot(&session_id)
        .expect("Restore failed");

    // Verify restored data
    let mgr_restored = SessionStorageManager::new(&base_dir, &session_id, 10_000_000).unwrap();
    assert_eq!(
        mgr_restored.local.get("origin", "key").unwrap().unwrap(),
        "persisted_value"
    );

    fs::remove_dir_all(&base_dir).unwrap();
}

#[tokio::test]
async fn test_path_traversal_blocked() {
    let base_dir = setup_test_dir("path_traversal");

    // Attempt directory traversal
    let bad_session_id = "../../../etc/passwd";
    let result = SessionStorageManager::new(&base_dir, bad_session_id, 1000);

    assert!(matches!(result, Err(StorageError::InvalidSessionId(_))));

    // Try UUID format but with traversal embedded
    let bad_uuid = format!("{}/../../foo", generate_test_id());
    let result_2 = SessionStorageManager::new(&base_dir, &bad_uuid, 1000);
    assert!(matches!(result_2, Err(StorageError::InvalidSessionId(_))));

    fs::remove_dir_all(&base_dir).unwrap();
}

#[tokio::test]
async fn test_quota_enforcement() {
    let base_dir = setup_test_dir("quota_enforcement");
    let session_id = generate_test_id();

    // Create with tiny quota 1KB
    let mgr = SessionStorageManager::new(&base_dir, &session_id, 1024).unwrap();

    // Allocate exactly 1KB
    mgr.quota.add_usage(1024, "cache").unwrap();
    assert_eq!(mgr.quota.get_usage(), 1024);

    // Attempt to exceed
    let err = mgr.quota.add_usage(1, "cache").unwrap_err();
    assert!(matches!(err, StorageError::QuotaExceeded { .. }));

    fs::remove_dir_all(&base_dir).unwrap();
}

#[tokio::test]
async fn test_cross_session_isolation() {
    let base_dir = setup_test_dir("isolation");
    let session_a = generate_test_id();
    let session_b = generate_test_id();

    // Write to A
    let mgr_a = SessionStorageManager::new(&base_dir, &session_a, 10_000_000).unwrap();
    mgr_a
        .local
        .set("origin", "secret", "session_a_secret")
        .unwrap();

    // Attempt to read from B (should not have the data)
    let mgr_b = SessionStorageManager::new(&base_dir, &session_b, 10_000_000).unwrap();
    let val_in_b = mgr_b.local.get("origin", "secret").unwrap();

    assert_eq!(val_in_b, None, "Session B leaked Data from Session A");

    fs::remove_dir_all(&base_dir).unwrap();
}
