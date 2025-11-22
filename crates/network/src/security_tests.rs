//! Comprehensive Security Module Tests
//!
//! This module provides extensive testing for all network security
//! features including rate limiting, connection management, reputation,
//! ban management, and DoS protection.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread;

    // Test rate limiting functionality
    mod rate_limiter_tests {
        use super::super::rate_limiter::*;

        #[test]
        fn test_rate_limiter_normal_traffic() {
            let config = RateLimitConfig::default();
            let mut limiter = RateLimiter::new(config);
            let peer = PeerId::random();

            // Should allow normal traffic
            for _ in 0..50 {
                assert!(limiter.check_message(&peer, MessageType::Ping).is_ok());
            }
        }

        #[test]
        fn test_rate_limiter_message_type_limits() {
            let config = RateLimitConfig::default();
            let mut limiter = RateLimiter::new(config);
            let peer = PeerId::random();

            // Should allow up to transaction limit
            for _ in 0..50 {
                assert!(limiter.check_message(&peer, MessageType::Transaction).is_ok());
            }

            // Next transaction should be rate limited
            assert!(matches!(
                limiter.check_message(&peer, MessageType::Transaction),
                Err(RateLimitError::RateLimited)
            ));
        }

        #[test]
        fn test_rate_limiter_global_limits() {
            let mut config = RateLimitConfig::default();
            config.max_global_messages_per_second = 100;
            let mut limiter = RateLimiter::new(config);

            // Should trigger global limit
            for _ in 0..101 {
                let _ = limiter.check_message(&PeerId::random(), MessageType::Ping);
            }

            // Next message should be rejected
            assert!(matches!(
                limiter.check_message(&PeerId::random(), MessageType::Ping),
                Err(RateLimitError::GlobalLimitReached)
            ));
        }

        #[test]
        fn test_rate_limiter_peer_violations() {
            let mut config = RateLimitConfig::default();
            config.violation_threshold = 2;
            let mut limiter = RateLimiter::new(config);
            let peer = PeerId::random();

            // First violation
            assert!(matches!(
                limiter.check_message(&peer, MessageType::Ping),
                Err(RateLimitError::RateLimited)
            ));

            // Second violation should ban
            assert!(matches!(
                limiter.check_message(&peer, MessageType::Ping),
                Err(RateLimitError::BanPeer)
            ));
        }

        #[test]
        fn test_rate_limiter_cleanup() {
            let config = RateLimitConfig::default();
            let mut limiter = RateLimiter::new(config);
            let peer = PeerId::random();

            // Add peer
            assert!(limiter.check_message(&peer, MessageType::Ping).is_ok());

            // Cleanup should not remove active peer
            limiter.cleanup();
            assert_eq!(limiter.get_peer_violations(&peer), 1);
        }
    }

    // Test connection management functionality
    mod connection_manager_tests {
        use super::super::connection_manager::*;

        #[test]
        fn test_connection_manager_basic_operations() {
            let config = ConnectionConfig::default();
            let mut manager = ConnectionManager::new(config);
            let peer = PeerId::random();
            let ip = "127.0.0.1".parse().unwrap();

            // Should accept inbound connection
            assert!(manager.accept_inbound_connection(peer, ip, None).is_ok());
            assert_eq!(manager.get_connection_counts(), (1, 0, 1));

            // Should reject duplicate from same IP
            let peer2 = PeerId::random();
            assert!(matches!(
                manager.accept_inbound_connection(peer2, ip, None),
                Err(ConnectionError::IpLimitReached)
            ));
        }

        #[test]
        fn test_connection_limits() {
            let mut config = ConnectionConfig::default();
            config.max_total_connections = 2;
            let mut manager = ConnectionManager::new(config);

            let peer1 = PeerId::random();
            let peer2 = PeerId::random();
            let ip1 = "127.0.0.1".parse().unwrap();
            let ip2 = "127.0.0.2".parse().unwrap();

            // Should accept first two connections
            assert!(manager.accept_inbound_connection(peer1, ip1, None).is_ok());
            assert!(manager.accept_inbound_connection(peer2, ip2, None).is_ok());

            // Should reject third connection
            let peer3 = PeerId::random();
            let ip3 = "127.0.0.3".parse().unwrap();
            assert!(matches!(
                manager.accept_inbound_connection(peer3, ip3, None),
                Err(ConnectionError::GlobalLimitReached)
            ));
        }

        #[test]
        fn test_outbound_connections() {
            let config = ConnectionConfig::default();
            let mut manager = ConnectionManager::new(config);
            let peer = PeerId::random();
            let ip = "127.0.0.1".parse().unwrap();

            // Should accept outbound connection
            assert!(manager.initiate_outbound_connection(peer, ip, None).is_ok());
            assert_eq!(manager.get_connection_counts(), (0, 1, 1));
        }

        #[test]
        fn test_connection_cleanup() {
            let mut config = ConnectionConfig::default();
            config.idle_timeout = Duration::from_millis(100);
            let mut manager = ConnectionManager::new(config);

            let peer = PeerId::random();
            let ip = "127.0.0.1".parse().unwrap();

            assert!(manager.accept_inbound_connection(peer, ip, None).is_ok());

            // Wait for timeout
            thread::sleep(Duration::from_millis(150));

            // Should clean up idle connection
            let disconnected = manager.cleanup_connections();
            assert_eq!(disconnected.len(), 1);
            assert_eq!(disconnected[0], peer);
        }

        #[test]
        fn test_connection_statistics() {
            let config = ConnectionConfig::default();
            let mut manager = ConnectionManager::new(config);

            let peer1 = PeerId::random();
            let peer2 = PeerId::random();
            let ip1 = "127.0.0.1".parse().unwrap();
            let ip2 = "127.0.0.2".parse().unwrap();

            // Add connections
            assert!(manager.accept_inbound_connection(peer1, ip1, None).is_ok());
            assert!(manager.accept_inbound_connection(peer2, ip2, None).is_ok());

            let stats = manager.get_stats();
            assert_eq!(stats.total_connections, 2);
            assert_eq!(stats.current_connections, 2);
            assert_eq!(stats.unique_ips, 2);
        }
    }

    // Test reputation management functionality
    mod reputation_tests {
        use super::super::reputation::*;

        #[test]
        fn test_reputation_initial_score() {
            let config = ReputationConfig::default();
            let mut manager = ReputationManager::new(config);
            let peer = PeerId::random();

            let score = manager.get_score(&peer);
            assert_eq!(score, Some(50));
        }

        #[test]
        fn test_reputation_violation_penalties() {
            let config = ReputationConfig::default();
            let mut manager = ReputationManager::new(config);
            let peer = PeerId::random();

            // Report rate limit violation
            let action = manager.report_violation(&peer, Violation::RateLimitExceeded);
            assert_eq!(action, ReputationAction::None); // Score still above threshold

            let score = manager.get_score(&peer);
            assert_eq!(score, Some(40)); // 50 - 10
        }

        #[test]
        fn test_reputation_banning() {
            let config = ReputationConfig::default();
            let mut manager = ReputationManager::new(config);
            let peer = PeerId::random();

            // Report multiple violations to trigger ban
            for _ in 0..5 {
                manager.report_violation(&peer, Violation::ProtocolViolation);
            }

            let action = manager.report_violation(&peer, Violation::ProtocolViolation);
            assert!(matches!(action, ReputationAction::PermanentBan));

            assert!(manager.is_banned(&peer));
        }

        #[test]
        fn test_reputation_good_behavior() {
            let config = ReputationConfig::default();
            let mut manager = ReputationManager::new(config);
            let peer = PeerId::random();

            // Report violation
            manager.report_violation(&peer, Violation::RateLimitExceeded);
            let score = manager.get_score(&peer);
            assert_eq!(score, Some(40));

            // Report good behavior
            manager.report_good_behavior(&peer);
            let score = manager.get_score(&peer);
            assert_eq!(score, Some(41)); // 40 + 1
        }

        #[test]
        fn test_reputation_decay() {
            let config = ReputationConfig::default();
            let mut manager = ReputationManager::new(config);
            let peer = PeerId::random();

            // Report violation
            manager.report_violation(&peer, Violation::ProtocolViolation);
            let score = manager.get_score(&peer);
            assert_eq!(score, Some(0)); // 50 - 50

            // Apply decay (simulate time passing)
            manager.apply_decay();
            let score = manager.get_score(&peer);
            assert_eq!(score, Some(1)); // Should increase towards initial
        }

        #[test]
        fn test_reputation_statistics() {
            let config = ReputationConfig::default();
            let mut manager = ReputationManager::new(config);

            let peer1 = PeerId::random();
            let peer2 = PeerId::random();
            let peer3 = PeerId::random();

            // Add peers with different scores
            manager.report_good_behavior(&peer1);
            manager.report_violation(&peer2, Violation::RateLimitExceeded);
            for _ in 0..4 {
                manager.report_violation(&peer3, Violation::ProtocolViolation);
            }

            let stats = manager.get_stats();
            assert_eq!(stats.total_peers, 3);
            assert!(stats.average_score > 0.0 && stats.average_score < 50.0);
        }
    }

    // Test ban management functionality
    mod ban_manager_tests {
        use super::super::ban_manager::*;

        #[test]
        fn test_ban_manager_basic_operations() {
            let config = BanConfig::default();
            let mut manager = BanManager::new(config);
            let peer = PeerId::random();
            let reason = BanReason::RateLimitViolation;

            // Should ban peer
            assert!(manager.ban_peer_temporarily(peer, reason.clone()).is_ok());
            assert!(manager.is_peer_banned(&peer));

            // Should get ban info
            let ban_info = manager.get_peer_ban_info(&peer);
            assert!(ban_info.is_some());
            assert_eq!(ban_info.unwrap().reason, reason);
        }

        #[test]
        fn test_ban_expiration() {
            let mut config = BanConfig::default();
            config.default_temp_ban_duration = Duration::from_millis(100);
            let mut manager = BanManager::new(config);
            let peer = PeerId::random();

            // Trigger temporary ban
            assert!(manager.ban_peer_temporarily(peer, BanReason::ProtocolViolation).is_ok());
            assert!(manager.is_peer_banned(&peer));

            // Wait for expiration
            thread::sleep(Duration::from_millis(150));

            // Clear expired bans
            let cleared = manager.clear_expired_bans();
            assert_eq!(cleared, 1);
            assert!(!manager.is_peer_banned(&peer));
        }

        #[test]
        fn test_permanent_ban() {
            let config = BanConfig::default();
            let mut manager = BanManager::new(config);
            let peer = PeerId::random();

            // Permanent ban
            assert!(manager.ban_peer_permanently(peer, BanReason::AttackBehavior, None, None).is_ok());
            assert!(manager.is_peer_banned(&peer));

            // Should not expire
            thread::sleep(Duration::from_millis(100));
            let cleared = manager.clear_expired_bans();
            assert_eq!(cleared, 0);
            assert!(manager.is_peer_banned(&peer));
        }

        #[test]
        fn test_ip_banning() {
            let config = BanConfig::default();
            let mut manager = BanManager::new(config);
            let ip = "192.168.1.100".parse().unwrap();

            // Ban IP
            assert!(manager.ban_ip(ip, BanReason::SpamBehavior, None, None).is_ok());
            assert!(manager.is_ip_banned(&ip));

            // Should get ban info
            let ban_info = manager.get_ip_ban_info(&ip);
            assert!(ban_info.is_some());
            assert_eq!(ban_info.unwrap().reason, BanReason::SpamBehavior);
        }

        #[test]
        fn test_ban_statistics() {
            let config = BanConfig::default();
            let mut manager = BanManager::new(config);

            let peer1 = PeerId::random();
            let peer2 = PeerId::random();
            let ip1 = "127.0.0.1".parse().unwrap();
            let ip2 = "127.0.0.2".parse().unwrap();

            // Create different types of bans
            assert!(manager.ban_peer_temporarily(peer1, BanReason::RateLimitViolation).is_ok());
            assert!(manager.ban_peer_permanently(peer2, BanReason::AttackBehavior, None, None).is_ok());

            let stats = manager.get_stats();
            assert_eq!(stats.total_bans, 2);
            assert_eq!(stats.active_bans, 2);
            assert_eq!(stats.temporary_bans, 1);
            assert_eq!(stats.permanent_bans, 1);
        }

        #[test]
        fn test_ban_export() {
            let config = BanConfig::default();
            let mut manager = BanManager::new(config);
            let peer = PeerId::random();
            let ip = "192.168.1.100".parse().unwrap();

            // Create bans
            assert!(manager.ban_peer_temporarily(peer, BanReason::RateLimitViolation).is_ok());
            assert!(manager.ban_ip(ip, BanReason::SpamBehavior, None, None).is_ok());

            // Export bans
            let export = manager.export_bans();
            assert!(export.contains("Peer:"));
            assert!(export.contains("IP:"));
            assert!(export.contains("RateLimitViolation"));
            assert!(export.contains("SpamBehavior"));
        }
    }

    // Test DoS protection functionality
    mod dos_protection_tests {
        use super::super::dos_protection::*;

        #[test]
        fn test_syn_flood_detection() {
            let config = DoSConfig::default();
            let mut protection = DoSProtection::new(config);
            let ip = "192.168.1.100".parse().unwrap();

            // Should allow normal connections
            for _ in 0..3 {
                assert!(protection.handle_syn_packet(ip, None).is_ok());
            }

            // Should detect SYN flood
            let result = protection.handle_syn_packet(ip, None);
            assert!(matches!(result, Err(DoSError::SynFlood)));
        }

        #[test]
        fn test_connection_flood_detection() {
            let config = DoSConfig::default();
            config.connection_flood_threshold = 5;
            let mut protection = DoSProtection::new(config);
            let ip = "192.168.1.100".parse().unwrap();
            let peer = PeerId::random();

            // Should allow normal connections
            for _ in 0..4 {
                assert!(protection.track_connection(peer, ip).is_ok());
            }

            // Should detect flood
            let result = protection.track_connection(peer, ip);
            assert!(matches!(result, Err(DoSError::ConnectionFlood)));
        }

        #[test]
        fn test_bandwidth_tracking() {
            let config = DoSConfig::default();
            config.max_bandwidth_per_peer = 1000;
            let mut protection = DoSProtection::new(config);
            let peer = PeerId::random();

            // Should allow normal bandwidth
            assert!(protection.track_bandwidth(peer, 500, 500).is_ok());

            // Should detect bandwidth attack
            let result = protection.track_bandwidth(peer, 600, 600);
            assert!(matches!(result, Err(DoSError::BandwidthAttack)));
        }

        #[test]
        fn test_pattern_analysis() {
            let config = DoSConfig::default();
            config.suspicious_pattern_threshold = 0.7;
            let mut protection = DoSProtection::new(config);
            let peer = PeerId::random();

            // Normal activity
            assert!(protection.analyze_pattern(peer, 10, 0).is_ok());

            // Suspicious activity
            let result = protection.analyze_pattern(peer, 100, 50);
            assert!(matches!(result, Err(DoSError::SuspiciousPattern)));
        }

        #[test]
        fn test_attack_statistics() {
            let config = DoSConfig::default();
            let mut protection = DoSProtection::new(config);
            let ip = "192.168.1.100".parse().unwrap();

            // Trigger different attack types
            assert!(protection.handle_syn_packet(ip, None).is_ok());
            assert!(protection.track_connection(&PeerId::random(), ip).is_ok());
            assert!(protection.track_bandwidth(&PeerId::random(), 100, 100).is_ok());
            assert!(protection.analyze_pattern(&PeerId::random(), 50, 25).is_ok());

            let stats = protection.get_stats();
            assert_eq!(stats.total_attacks_detected, 4);
            assert_eq!(stats.syn_floods_detected, 1);
            assert_eq!(stats.connection_floods_detected, 1);
            assert_eq!(stats.bandwidth_attacks_detected, 1);
            assert_eq!(stats.pattern_attacks_detected, 1);
        }
    }

    // Test security configuration
    mod security_config_tests {
        use super::super::security_config::*;

        #[test]
        fn test_security_config_validation() {
            let config = SecurityConfig::default();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn test_security_config_invalid() {
            let mut config = SecurityConfig::default();
            config.rate_limiting.max_messages_per_window = 0;
            assert!(config.validate().is_err());
        }

        #[test]
        fn test_security_levels() {
            // Test minimal level
            let minimal = SecurityConfig::for_security_level(SecurityLevel::Minimal);
            assert_eq!(minimal.connections.max_total_connections, 50);
            assert_eq!(minimal.reputation.initial_score, 30);

            // Test standard level
            let standard = SecurityConfig::for_security_level(SecurityLevel::Standard);
            assert_eq!(standard.connections.max_total_connections, 125);
            assert_eq!(standard.reputation.initial_score, 50);

            // Test high level
            let high = SecurityConfig::for_security_level(SecurityLevel::High);
            assert_eq!(high.connections.max_total_connections, 200);
            assert_eq!(high.reputation.initial_score, 60);

            // Test maximum level
            let maximum = SecurityConfig::for_security_level(SecurityLevel::Maximum);
            assert_eq!(maximum.connections.max_total_connections, 500);
            assert_eq!(maximum.reputation.initial_score, 80);
        }

        #[test]
        fn test_config_export_import() {
            let config = SecurityConfig::default();

            // Export to TOML
            let toml_str = config.export_toml().unwrap();

            // Import from TOML
            let imported = SecurityConfig::import_toml(&toml_str).unwrap();

            // Should be equal
            assert_eq!(imported.rate_limiting.max_messages_per_window, config.rate_limiting.max_messages_per_window);
            assert_eq!(imported.connections.max_total_connections, config.connections.max_total_connections);
        }
    }

    // Integration tests for all security modules
    mod integration_tests {
        use super::super::*;
        use std::sync::Arc;
        use tokio::time::{sleep, Duration};

        #[tokio::test]
        async fn test_integrated_security_system() {
            let security_config = SecurityConfig::for_security_level(SecurityLevel::Standard);
            let mut rate_limiter = RateLimiter::new(security_config.rate_limiting.clone());
            let mut connection_manager = ConnectionManager::new(security_config.connections.clone());
            let mut reputation_manager = ReputationManager::new(security_config.reputation.clone());
            let mut ban_manager = BanManager::new(security_config.bans.clone());
            let mut dos_protection = DoSProtection::new(security_config.dos_protection.clone());

            let malicious_peer = PeerId::random();
            let malicious_ip = "192.168.1.100".parse().unwrap();

            // Simulate attack
            for i in 0..1000 {
                let _ = rate_limiter.check_message(&malicious_peer, MessageType::Ping);

                if i % 100 == 0 {
                    let _ = connection_manager.accept_inbound_connection(malicious_peer, malicious_ip, None);
                    let _ = reputation_manager.report_violation(&malicious_peer, Violation::RateLimitExceeded);
                }
            }

            // Should detect and mitigate attack
            let stats = dos_protection.get_stats();
            assert!(stats.total_attacks_detected > 0);

            // Check peer is banned
            assert!(reputation_manager.is_banned(&malicious_peer) || ban_manager.is_peer_banned(&malicious_peer));
        }

        #[tokio::test]
        async fn test_security_performance_under_load() {
            let security_config = SecurityConfig::for_security_level(SecurityLevel::High);
            let mut rate_limiter = RateLimiter::new(security_config.rate_limiting.clone());
            let mut connection_manager = ConnectionManager::new(security_config.connections.clone());

            let start = std::time::Instant::now();

            // Simulate high load
            let mut handles = Vec::new();
            for i in 0..100 {
                let peer = PeerId::random();
                let ip = format!("127.0.0.{}", i % 255 + 1).parse().unwrap();

                let handle = tokio::spawn(async move {
                    let _ = connection_manager.accept_inbound_connection(peer, ip, None);
                    sleep(Duration::from_millis(1)).await;
                });

                handles.push(handle);
            }

            // Wait for all connections
            for handle in handles {
                handle.await.unwrap();
            }

            let duration = start.elapsed();

            // Should handle high load efficiently
            assert!(duration.as_secs() < 5); // Should complete within 5 seconds

            let stats = connection_manager.get_stats();
            assert!(stats.total_connections >= 50); // Should accept many connections
        }
    }
}
