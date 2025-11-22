use bitquan_network::*;

#[test]
fn test_ban_enforcement() {
    let config = NetworkConfig::default();
    let mut service = NetworkService::new(config);
    
    let peer = "test_peer".to_string();
    let ip = "127.0.0.1".parse().unwrap();
    
    // Ban peer
    service.security_mut()
        .ban_peer(peer.clone(), BanReason::ManualBan("test".to_string()))
        .unwrap();
    
    // Should reject
    assert!(service.connect(peer, ip).is_err());
}

#[test]
fn test_normal_connection() {
    let config = NetworkConfig::default();
    let mut service = NetworkService::new(config);
    
    let peer = "good_peer".to_string();
    let ip = "192.168.1.100".parse().unwrap();
    
    // Should allow connection
    assert!(service.connect(peer.clone(), ip).is_ok());
    
    // Verify peer is connected
    assert_eq!(service.peers().len(), 1);
    assert!(service.peers().contains(&peer));
}

#[test]
fn test_ip_ban() {
    let config = NetworkConfig::default();
    let mut service = NetworkService::new(config);
    
    let peer = "banned_peer".to_string();
    let ip = "10.0.0.1".parse().unwrap();
    
    // Ban IP
    service.security_mut()
        .ban_ip(ip, BanReason::ManualBan("test".to_string()))
        .unwrap();
    
    // Should reject connection from banned IP
    assert!(service.connect(peer, ip).is_err());
}

#[test]
fn test_security_manager_access() {
    let config = NetworkConfig::default();
    let service = NetworkService::new(config);
    
    // Should be able to access security manager
    let security = service.security();
    let stats = security.get_statistics();
    
    // Verify initial stats
    assert!(stats.security_score > 0.0);
    assert_eq!(stats.connections.current_connections, 0);
}