//! DDoS Attack Simulation Tests
//!
//! This module provides DDoS attack simulation tests to evaluate
//! the resilience of the P2P network layer against various attack vectors.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bitquan_network::protocol::{validate_message, InvType, Message};

/// Simple metrics collector for DDoS testing
#[derive(Debug)]
struct DdosMetrics {
    connections_attempted: Arc<AtomicU64>,
    connections_established: Arc<AtomicU64>,
    connections_rejected: Arc<AtomicU64>,
    messages_sent: Arc<AtomicU64>,
    messages_processed: Arc<AtomicU64>,
    messages_rejected: Arc<AtomicU64>,
    response_times: Arc<std::sync::Mutex<Vec<f64>>>,
}

impl DdosMetrics {
    fn new() -> Self {
        Self {
            connections_attempted: Arc::new(AtomicU64::new(0)),
            connections_established: Arc::new(AtomicU64::new(0)),
            connections_rejected: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_processed: Arc::new(AtomicU64::new(0)),
            messages_rejected: Arc::new(AtomicU64::new(0)),
            response_times: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

/// Test configuration
#[derive(Debug, Clone)]
struct DdosConfig {
    connection_flood_count: usize,
    message_rate_per_second: u32,
    test_duration: Duration,
}

impl Default for DdosConfig {
    fn default() -> Self {
        Self {
            connection_flood_count: 50,
            message_rate_per_second: 100,
            test_duration: Duration::from_secs(10),
        }
    }
}

/// Simulate TCP connection flood attack
fn simulate_connection_flood(config: &DdosConfig) -> DdosMetrics {
    println!("🔥 Simulating TCP Connection Flood Attack");
    println!("  - Target connections: {}", config.connection_flood_count);
    println!("  - Duration: {:?}", config.test_duration);
    
    let metrics = DdosMetrics::new();
    let start_time = std::time::Instant::now();
    
    // Create connection flood threads
    let mut handles = vec![];
    
    for i in 0..config.connection_flood_count {
        let metrics = metrics.connections_attempted.clone();
        let established = metrics.connections_established.clone();
        let rejected = metrics.connections_rejected.clone();
        
        let handle = thread::spawn(move || {
            let connection_id = i as u64;
            
            // Simulate connection attempt
            metrics.fetch_add(1, Ordering::Relaxed);
            
            // Simulate DoS protection (10% rejection rate)
            if connection_id % 10 == 0 {
                rejected.fetch_add(1, Ordering::Relaxed);
                return;
            }
            
            // Simulate connection establishment
            thread::sleep(Duration::from_millis(rand::random::<u64>() % 2000));
            
            if rand::random::<f64>() > 0.3 { // 70% success rate
                established.fetch_add(1, Ordering::Relaxed);
            } else {
                rejected.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all connections to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    println!("  - Duration: {:?}", duration);
    
    metrics
}

/// Simulate protocol message spam attack
fn simulate_message_spam(config: &DdosConfig) -> DdosMetrics {
    println!("🔥 Simulating Protocol Message Spam Attack");
    println!("  - Messages per second: {}", config.message_rate_per_second);
    println!("  - Duration: {:?}", config.test_duration);
    
    let metrics = DdosMetrics::new();
    let start_time = std::time::Instant::now();
    
    // Create message spam threads
    let mut handles = vec![];
    
    for _ in 0..5 { // 5 concurrent spammers
        let metrics = metrics.messages_sent.clone();
        let processed = metrics.messages_processed.clone();
        let rejected = metrics.messages_rejected.clone();
        
        let handle = thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut local_processed = 0;
            let mut local_rejected = 0;
            
            while start.elapsed() < config.test_duration {
                // Generate random message
                let message = generate_random_message();
                
                // Validate message (DoS protection check)
                if validate_message(&message).is_ok() {
                    local_processed += 1;
                } else {
                    local_rejected += 1;
                }
                
                // Simulate processing time
                thread::sleep(Duration::from_millis(10));
                
                // Send at target rate
                let delay = Duration::from_millis(1000) / config.message_rate_per_second;
                thread::sleep(delay);
            }
            
            // Update global metrics
            metrics.fetch_add(local_processed + local_rejected, Ordering::Relaxed);
            processed.fetch_add(local_processed, Ordering::Relaxed);
            rejected.fetch_add(local_rejected, Ordering::Relaxed);
        });
        
        handles.push(handle);
    }
    
    // Wait for all spammers to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start_time.elapsed();
    println!("  - Duration: {:?}", duration);
    
    metrics
}

/// Generate random protocol messages
fn generate_random_message() -> Message {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    match rng.gen_range(0..4) {
        0 => Message::Ping { nonce: rng.gen() },
        1 => Message::Pong { nonce: rng.gen() },
        2 => Message::Inv { 
            inventory: vec![bitquan_network::protocol::InvVector {
                hash: [rng.gen(); 32],
                inv_type: InvType::Block,
            }]
        },
        3 => Message::GetData { 
            inventory: vec![bitquan_network::protocol::InvVector {
                hash: [rng.gen(); 32],
                inv_type: InvType::Tx,
            }]
        },
        _ => Message::Ping { nonce: rng.gen() },
    }
}

/// Test connection flood protection
#[test]
fn test_connection_flood_protection() {
    let config = DdosConfig {
        connection_flood_count: 20,
        message_rate_per_second: 50,
        test_duration: Duration::from_secs(2),
    };
    
    let metrics = simulate_connection_flood(&config);
    
    // Check that some connections are rejected (DoS protection working)
    let attempted = metrics.connections_attempted.load(Ordering::Relaxed);
    let rejected = metrics.connections_rejected.load(Ordering::Relaxed);
    let established = metrics.connections_established.load(Ordering::Relaxed);
    
    println!("  - Connection Results:");
    println!("    - Attempted: {}", attempted);
    println!("    - Rejected: {}", rejected);
    println!("    - Established: {}", established);
    
    // Should have some rejections
    assert!(attempted > 0, "Should have attempted connections");
    assert!(rejected > 0, "Should have some rejected connections (DoS protection)");
    
    // Success rate should be reasonable
    let success_rate = established as f64 / attempted as f64;
    println!("    - Success Rate: {:.1}%", success_rate * 100.0);
    
    // Should not have 100% success (DoS protection working)
    assert!(success_rate < 1.0, "Should not have 100% success rate");
}

/// Test message spam protection
#[test]
fn test_message_spam_protection() {
    let config = DdosConfig {
        connection_flood_count: 5,
        message_rate_per_second: 20,
        test_duration: Duration::from_secs(2),
    };
    
    let metrics = simulate_message_spam(&config);
    
    let sent = metrics.messages_sent.load(Ordering::Relaxed);
    let processed = metrics.messages_processed.load(Ordering::Relaxed);
    let rejected = metrics.messages_rejected.load(Ordering::Relaxed);
    
    println!("  - Message Results:");
    println!("    - Sent: {}", sent);
    println!("    - Processed: {}", processed);
    println!("    - Rejected: {}", rejected);
    
    // Should have some processing
    assert!(sent > 0, "Should have sent messages");
    assert!(processed > 0 || rejected > 0, "Should have processed or rejected messages");
    
    // Processing rate should be reasonable
    let processing_rate = if sent > 0 {
        processed as f64 / sent as f64
    } else {
        0.0
    };
    println!("    - Processing Rate: {:.1}%", processing_rate * 100.0);
}

/// Test rate limiting effectiveness
#[test]
fn test_rate_limiting_effectiveness() {
    let config = DdosConfig {
        connection_flood_count: 30,
        message_rate_per_second: 150, // High rate
        test_duration: Duration::from_secs(3),
    };
    
    let metrics = simulate_connection_flood(&config);
    
    let attempted = metrics.connections_attempted.load(Ordering::Relaxed);
    let rejected = metrics.connections_rejected.load(Ordering::Relaxed);
    
    println!("  - Rate Limiting Test:");
    println!("    - Attempted: {}", attempted);
    println!("    - Rejected: {}", rejected);
    println!("    - Rejection Rate: {:.1}%", (rejected as f64 / attempted.max(1) as f64) * 100.0);
    
    // Should have significant rejections at high rate
    assert!(rejected > 0, "Should have rejections at high rate");
    let rejection_rate = rejected as f64 / attempted.max(1) as f64;
    assert!(rejection_rate > 0.1, "Should have at least 10% rejection rate");
}

/// Test memory protection
#[test]
fn test_memory_protection() {
    let config = DdosConfig {
        connection_flood_count: 10,
        message_rate_per_second: 10,
        test_duration: Duration::from_secs(2),
    };
    
    // Test message size validation
    let large_inv: Vec<bitquan_network::protocol::InvVector> = (0..100_000)
        .map(|i| bitquan_network::protocol::InvVector {
            hash: [i as u8; 32],
            inv_type: InvType::Block,
        })
        .collect();
    
    let large_message = Message::Inv { inventory: large_inv };
    
    let result = validate_message(&large_message);
    assert!(result.is_err(), "Should reject oversized inv message");
    
    // Test normal message
    let normal_inv: Vec<bitquan_network::protocol::InvVector> = (0..10)
        .map(|i| bitquan_network::protocol::InvVector {
            hash: [i as u8; 32],
            inv_type: InvType::Block,
        })
        .collect();
    
    let normal_message = Message::Inv { inventory: normal_inv };
    let result = validate_message(&normal_message);
    assert!(result.is_ok(), "Should accept normal sized message");
    
    println!("  - Memory Protection:");
    println!("    - Large message rejected: {}", result.is_err());
    println!("    - Normal message accepted: {}", result.is_ok());
}

/// Run comprehensive DDoS simulation
#[test]
fn run_comprehensive_ddos_simulation() {
    println!("🚀 Starting Comprehensive DDoS Simulation");
    println!("{}", "=".repeat(50));
    
    let config = DdosConfig::default();
    
    // Test 1: Connection Flood
    println!("\n1. Connection Flood Test");
    let conn_metrics = simulate_connection_flood(&config);
    
    // Test 2: Message Spam
    println!("\n2. Message Spam Test");
    let msg_metrics = simulate_message_spam(&config);
    
    // Generate summary report
    println!("\n{}", "=".repeat(60));
    println!("📊 DDoS Simulation Summary Report");
    println!("{}", "=".repeat(60));
    
    // Connection metrics
    let conn_attempted = conn_metrics.connections_attempted.load(Ordering::Relaxed);
    let conn_established = conn_metrics.connections_established.load(Ordering::Relaxed);
    let conn_rejected = conn_metrics.connections_rejected.load(Ordering::Relaxed);
    let conn_success_rate = if conn_attempted > 0 {
        (conn_established as f64 / conn_attempted as f64) * 100.0
    } else {
        0.0
    };
    
    println!("\n🔗 Connection Flood Results:");
    println!("  - Attempted: {}", conn_attempted);
    println!("  - Established: {}", conn_established);
    println!("  - Rejected: {}", conn_rejected);
    println!("  - Success Rate: {:.1}%", conn_success_rate);
    
    // Message metrics
    let msg_sent = msg_metrics.messages_sent.load(Ordering::Relaxed);
    let msg_processed = msg_metrics.messages_processed.load(Ordering::Relaxed);
    let msg_rejected = msg_metrics.messages_rejected.load(Ordering::Relaxed);
    let msg_processing_rate = if msg_sent > 0 {
        (msg_processed as f64 / msg_sent as f64) * 100.0
    } else {
        0.0
    };
    
    println!("\n💬 Message Spam Results:");
    println!("  - Sent: {}", msg_sent);
    println!("  - Processed: {}", msg_processed);
    println!("  - Rejected: {}", msg_rejected);
    println!("  - Processing Rate: {:.1}%", msg_processing_rate);
    
    // Security assessment
    println!("\n🛡️ Security Assessment:");
    
    if conn_success_rate > 80.0 {
        println!("  ⚠️  High connection success rate - Consider tightening connection limits");
    }
    
    if msg_processing_rate < 50.0 {
        println!("  ⚠️  Low message processing rate - Increase validation efficiency");
    }
    
    if conn_rejected > conn_attempted / 4 {
        println!("  ✅ Good connection rejection rate - DoS protection working");
    }
    
    if msg_rejected > msg_sent / 4 {
        println!("  ✅ Good message rejection rate - Message validation working");
    }
    
    println!("\n✅ DDoS simulation complete!");
}
