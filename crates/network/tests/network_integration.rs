//! Integration tests for P2P network functionality.

use bitquan_network::protocol::{InvType, Message};
use bitquan_network::{
    bootstrap_peers, create_envelope, BlockPropagator, ChainSync, PeerBook, SyncStatus,
};

#[test]
fn test_peer_book_management() {
    let mut book = PeerBook::new();

    // Add peers
    book.add_peer("peer1:18444".to_string());
    book.add_peer("peer2:18444".to_string());
    book.add_peer("peer3:18444".to_string());

    assert_eq!(book.peer_count(), 3);

    // Mark peer as successful
    book.mark_peer_success("peer1:18444");
    book.mark_peer_success("peer1:18444");

    // Get best peers
    let best = book.best_peers(2);
    assert!(!best.is_empty());
    assert_eq!(best[0], "peer1:18444");
}

#[test]
fn test_broadcast_block_to_peers() {
    let propagator = BlockPropagator::new();
    let block_hash = [42u8; 32];

    // Should propagate first time
    assert!(propagator.should_propagate_block(block_hash));

    // Mark as propagated
    propagator
        .mark_block_propagated(block_hash)
        .expect("Failed to mark block as propagated");

    // Should not propagate again
    assert!(!propagator.should_propagate_block(block_hash));

    // Check stats
    let stats = propagator.stats().expect("Failed to get propagator stats");
    assert_eq!(stats.blocks_broadcast, 1);
}

#[test]
fn test_duplicate_block_filtering() {
    let propagator = BlockPropagator::new();
    let hash1 = [1u8; 32];
    let hash2 = [2u8; 32];

    // Receive first block
    assert!(propagator
        .mark_block_received(hash1)
        .expect("Failed to mark first block as received"));

    // Receive second block
    assert!(propagator
        .mark_block_received(hash2)
        .expect("Failed to mark second block as received"));

    // Try to receive first block again
    assert!(!propagator
        .mark_block_received(hash1)
        .expect("Failed to mark duplicate block"));

    // Check stats
    let stats = propagator.stats().expect("Failed to get propagator stats");
    assert_eq!(stats.blocks_received, 2);
    assert_eq!(stats.blocks_rejected, 1);
}

#[test]
fn test_chain_sync_to_higher_height() {
    let sync = ChainSync::new(100);

    // Initially synced
    assert!(!sync.needs_sync());
    assert_eq!(sync.blocks_behind(), 0);

    // Peer announces higher height
    sync.set_best_height(150);

    // Now needs sync
    assert!(sync.needs_sync());
    assert_eq!(sync.blocks_behind(), 50);
    assert!((sync.progress() - 66.67).abs() < 0.01); // Float comparison with tolerance

    // Start sync
    assert!(sync.start_sync());
    assert!(sync.is_syncing());
    assert_eq!(sync.status(), SyncStatus::Discovering);

    // Update local height
    sync.set_local_height(125);
    assert!((sync.progress() - 83.33).abs() < 0.01); // Float comparison with tolerance

    // Catch up
    sync.set_local_height(150);

    // Auto-complete sync
    assert!(!sync.is_syncing());
    assert_eq!(sync.status(), SyncStatus::Synced);
}

#[test]
fn test_metrics_update_on_block_event() {
    let propagator = BlockPropagator::new();

    // Simulate receiving blocks
    for i in 0..10u8 {
        let hash = [i; 32];
        let _ = propagator.mark_block_received(hash);
    }

    // Check metrics
    let stats = propagator.stats().expect("Failed to get propagator stats");
    assert_eq!(stats.blocks_received, 10);
    assert_eq!(stats.blocks_rejected, 0);

    // Simulate broadcasting blocks
    for i in 0..5u8 {
        let hash = [i + 100; 32];
        let _ = propagator.mark_block_propagated(hash);
    }

    let stats = propagator
        .stats()
        .expect("Failed to get propagator stats after broadcast");
    assert_eq!(stats.blocks_broadcast, 5);
}

#[test]
fn test_create_block_inv_message() {
    let propagator = BlockPropagator::new();
    let hash = [123u8; 32];

    let inv = propagator.create_block_inv(hash);

    match inv {
        Message::Inv { inventory } => {
            assert_eq!(inventory.len(), 1);
            assert_eq!(inventory[0].hash, hash);
            assert_eq!(inventory[0].inv_type, InvType::Block);
        }
        _ => panic!("Expected Inv message"),
    }
}

#[test]
fn test_message_envelope_creation() {
    let msg = Message::Ping { nonce: 12345 };
    let envelope = create_envelope(msg.clone());

    assert_eq!(envelope.message, msg);
}

#[test]
fn test_bootstrap_peers_testnet() {
    let book = bootstrap_peers(true);

    // Should have seed nodes
    assert!(book.peer_count() > 0);

    // Should have localhost for testing
    assert!(book.get_peer("127.0.0.1:18444").is_some());
}

#[test]
fn test_bootstrap_peers_mainnet() {
    let book = bootstrap_peers(false);

    // Should have mainnet seeds
    assert!(book.peer_count() > 0);
}

#[test]
fn test_peer_scoring() {
    let mut book = PeerBook::new();

    book.add_peer("good_peer:18444".to_string());
    book.add_peer("bad_peer:18444".to_string());

    // Good peer: many successes
    for _ in 0..10 {
        book.mark_peer_success("good_peer:18444");
    }

    // Bad peer: many failures
    for _ in 0..10 {
        book.mark_peer_failure("bad_peer:18444");
    }
    book.mark_peer_success("bad_peer:18444"); // One success

    // Best peers should rank good peer first
    let best = book.best_peers(2);
    assert_eq!(best[0], "good_peer:18444");
}

#[test]
fn test_sync_status_transitions() {
    let sync = ChainSync::new(0);

    assert_eq!(sync.status(), SyncStatus::Idle);

    sync.set_status(SyncStatus::Discovering);
    assert_eq!(sync.status(), SyncStatus::Discovering);

    sync.set_status(SyncStatus::DownloadingHeaders);
    assert_eq!(sync.status(), SyncStatus::DownloadingHeaders);

    sync.set_status(SyncStatus::DownloadingBlocks);
    assert_eq!(sync.status(), SyncStatus::DownloadingBlocks);

    sync.set_status(SyncStatus::Synced);
    assert_eq!(sync.status(), SyncStatus::Synced);
}

#[test]
fn test_concurrent_sync_prevention() {
    let sync = ChainSync::new(0);
    sync.set_best_height(100);

    // First start should succeed
    assert!(sync.start_sync());
    assert!(sync.is_syncing());

    // Second start should fail (already syncing)
    assert!(!sync.start_sync());
    assert!(sync.is_syncing());

    // Complete sync
    sync.complete_sync();
    assert!(!sync.is_syncing());

    // Now can start again
    assert!(sync.start_sync());
}

#[test]
fn test_peer_book_persistence() {
    let mut book = PeerBook::new();
    book.add_peer("persistent:18444".to_string());
    book.mark_peer_success("persistent:18444");

    // Use a cross-platform temporary directory
    let temp_path = std::env::temp_dir().join("bitquan_network_test.json");

    // Save
    book.save_to_file(
        temp_path
            .to_str()
            .expect("Failed to convert temp path to string"),
    )
    .expect("Failed to save peer book");

    // Load
    let loaded = PeerBook::load_from_file(
        temp_path
            .to_str()
            .expect("Failed to convert temp path to string"),
    )
    .expect("Failed to load peer book");

    assert_eq!(loaded.peer_count(), 1);
    assert!(loaded.get_peer("persistent:18444").is_some());

    // Cleanup
    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_propagation_stats_reset() {
    let propagator = BlockPropagator::new();

    // Generate some stats
    for i in 0..5u8 {
        let _ = propagator.mark_block_received([i; 32]);
    }

    let stats = propagator.stats().expect("Failed to get propagator stats");
    assert_eq!(stats.blocks_received, 5);

    // Reset
    propagator
        .reset_stats()
        .expect("Failed to reset propagator stats");

    let stats = propagator
        .stats()
        .expect("Failed to get propagator stats after reset");
    assert_eq!(stats.blocks_received, 0);
}
