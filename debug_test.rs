use bitquan_network::*;

#[test]
fn debug_reputation_issue() {
    let config = ReputationConfig::default();
    let mut manager = ReputationManager::new(config);
    let peer = format!("test_peer_{}", rand::random::<u64>());

    println!("Created peer: {}", peer);

    // Check if peer exists
    let score_before = manager.get_score(&peer);
    println!("Score before: {:?}", score_before);

    // The test expects this to work
    assert_eq!(score_before, Some(50));
}
