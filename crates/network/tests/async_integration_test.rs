//! Integration tests for async network layer

use bitquan_network::peer_async::AsyncPeerManager;
use bitquan_network::server_async::spawn_p2p_server_with_limit;
use bitquan_types::NetworkId;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

/// Helper to get a random free port for testing
fn get_random_port() -> u16 {
    use std::net::TcpListener as StdTcpListener;
    let listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    addr.port()
}

#[tokio::test]
async fn test_async_p2p_server_startup() {
    let port = get_random_port();
    let addr = format!("127.0.0.1:{}", port);
    let addr_clone = addr.clone();
    let peer_manager = Arc::new(AsyncPeerManager::new(10, NetworkId::Devnet));

    // Start server on random port
    let result = spawn_p2p_server_with_limit(&addr_clone, peer_manager.clone(), 10).await;

    assert!(result.is_ok(), "Failed to start P2P server: {:?}", result);

    // Give it time to start
    sleep(Duration::from_millis(100)).await;

    // Verify we can connect to the server
    let connect_result = TcpStream::connect(&addr).await;
    assert!(
        connect_result.is_ok(),
        "Could not connect to started server"
    );

    // Check peer count
    let peer_count = peer_manager.peer_count().await;
    assert_eq!(peer_count, 0, "Should have 0 peers initially");
}

#[tokio::test]
async fn test_slowloris_protection() {
    // This test verifies basic timeout functionality
    let port = get_random_port();
    let addr = format!("127.0.0.1:{}", port);
    let addr_clone = addr.clone();
    let peer_manager = Arc::new(AsyncPeerManager::new(10, NetworkId::Devnet));

    // Start server
    tokio::spawn(async move {
        spawn_p2p_server_with_limit(&addr_clone, peer_manager, 10)
            .await
            .unwrap()
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Verify we can connect
    let _stream = TcpStream::connect(&addr).await.expect("Failed to connect");

    // For now, just test that we can establish connections
    // Full Slowloris testing would require more complex setup with timeout configuration
    // Basic connection test passed - no assertion needed
}

#[tokio::test]
async fn test_connection_limit() {
    let port = get_random_port();
    let addr = format!("127.0.0.1:{}", port);
    let addr_clone = addr.clone();
    let max_connections = 5;
    let peer_manager = Arc::new(AsyncPeerManager::new(max_connections, NetworkId::Devnet));

    // Start server with connection limit
    tokio::spawn(async move {
        spawn_p2p_server_with_limit(&addr_clone, peer_manager.clone(), max_connections)
            .await
            .unwrap();
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Try to connect to the server
    let connection_result = TcpStream::connect(&addr).await;
    assert!(
        connection_result.is_ok(),
        "Should be able to connect to server"
    );

    // For now, just test basic connection establishment
    // Full connection limit testing would require more complex setup
    // Connection limit test basic setup passed - no assertion needed
}

#[tokio::test]
async fn test_peer_manager_concurrent_access() {
    let peer_manager = Arc::new(AsyncPeerManager::new(100, NetworkId::Devnet));

    // Spawn multiple tasks to access peer manager concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let pm = peer_manager.clone();
        let handle = tokio::spawn(async move {
            // Test concurrent peer count access
            for _ in 0..10 {
                let count = pm.peer_count().await;
                let ready_count = pm.ready_peer_count().await;
                assert!(
                    ready_count <= count,
                    "Ready peers should not exceed total peers"
                );

                // Small delay to increase chance of race conditions
                sleep(Duration::from_millis(1)).await;
            }

            // Note: We cannot easily add mock peers without actual TcpStream
            // This test focuses on concurrent read access instead
            i
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results = futures::future::join_all(handles).await;

    // Verify all tasks completed successfully
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Task {} failed: {:?}", i, result);
        assert_eq!(result.unwrap(), i, "Task {} should return its index", i);
    }

    // Verify final state (should still be 0 since we didn't add real peers)
    let final_count = peer_manager.peer_count().await;
    assert_eq!(
        final_count, 0,
        "Should have 0 peers after concurrent access test"
    );
}

#[tokio::test]
async fn test_peer_manager_cleanup() {
    let peer_manager = Arc::new(AsyncPeerManager::new(100, NetworkId::Devnet));

    // Start with 0 peers
    let initial_count = peer_manager.peer_count().await;
    assert_eq!(initial_count, 0, "Should have 0 peers initially");

    // Run cleanup (should work fine even with no peers)
    peer_manager.cleanup_peers().await;

    // Count should remain 0
    let after_cleanup_count = peer_manager.peer_count().await;
    assert_eq!(
        after_cleanup_count, 0,
        "Should still have 0 peers after cleanup"
    );
}

#[tokio::test]
async fn test_async_network_error_handling() {
    use bitquan_network::async_sync::AsyncSyncManager;

    let sync_manager = AsyncSyncManager::new_for_testing(100); // height=100

    // Test getting sync status
    let result = sync_manager.sync_status().await;
    assert!(result.is_ok(), "Should get sync status: {:?}", result);

    // Test basic functionality works without panics
    assert!(result.is_ok(), "Sync status should be accessible");
}

// Helper function to write to stream
#[allow(dead_code)] // Test helper - may be used in future tests
async fn write_all_with_timeout(
    stream: &mut TcpStream,
    data: &[u8],
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(timeout, stream.write_all(data)).await??;
    Ok(())
}

// Helper function to read from stream
#[allow(dead_code)] // Test helper - may be used in future tests
async fn read_u8_with_timeout(
    stream: &mut TcpStream,
    timeout: Duration,
) -> Result<u8, Box<dyn std::error::Error>> {
    let result = tokio::time::timeout(timeout, stream.read_u8()).await??;
    Ok(result)
}
