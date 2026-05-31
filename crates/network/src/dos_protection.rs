//! DoS Protection for Network Security
//!
//! This module provides comprehensive protection against Denial of Service
//! attacks including SYN floods, connection floods, and bandwidth attacks.
//!
//! ## Features
//!
//! - SYN flood protection
//! - Connection rate limiting
//! - Bandwidth throttling
//! - Attack detection and mitigation
//! - OS-level protection integration
//! - Automatic response to attacks

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::PeerId;

/// DoS protection errors
#[derive(Debug, Clone)]
pub enum DoSError {
    /// SYN flood detected
    SynFlood,
    /// Connection flood detected
    ConnectionFlood,
    /// Bandwidth attack detected
    BandwidthAttack,
    /// Resource exhaustion detected
    ResourceExhaustion,
    /// Suspicious activity pattern
    SuspiciousPattern,
}

/// DoS protection configuration
#[derive(Debug, Clone)]
pub struct DoSConfig {
    /// Enable SYN flood protection
    pub enable_syn_protection: bool,
    /// Max half-open connections per IP
    pub max_half_open_per_ip: usize,
    /// SYN cookie timeout
    pub syn_cookie_timeout: Duration,

    /// Connection flood detection
    pub max_connections_per_second: u32,
    /// Connection flood threshold
    pub connection_flood_threshold: u32,
    /// Connection flood window duration
    pub connection_flood_window: Duration,

    /// Bandwidth protection
    pub enable_bandwidth_protection: bool,
    /// Maximum bandwidth per peer (bytes/sec)
    pub max_bandwidth_per_peer: u64, // bytes per second
    /// Bandwidth burst size (bytes)
    pub bandwidth_burst_size: u64,
    /// Bandwidth window duration
    pub bandwidth_window_duration: Duration,

    /// Pattern detection
    pub enable_pattern_detection: bool,
    /// Suspicious pattern threshold
    pub suspicious_pattern_threshold: f64,
    /// Pattern analysis window duration
    pub pattern_analysis_window: Duration,

    /// Response actions
    pub auto_ban_on_detection: bool,
    /// Automatic ban duration
    pub auto_ban_duration: Duration,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
}

impl Default for DoSConfig {
    fn default() -> Self {
        Self {
            enable_syn_protection: true,
            max_half_open_per_ip: 100,
            syn_cookie_timeout: Duration::from_secs(5),
            max_connections_per_second: 50,
            connection_flood_threshold: 100,
            connection_flood_window: Duration::from_secs(10),
            enable_bandwidth_protection: true,
            max_bandwidth_per_peer: 1048576, // 1 MB/s
            bandwidth_burst_size: 2097152,   // 2 MB
            bandwidth_window_duration: Duration::from_secs(1),
            enable_pattern_detection: true,
            suspicious_pattern_threshold: 0.8,
            pattern_analysis_window: Duration::from_secs(60),
            auto_ban_on_detection: true,
            auto_ban_duration: Duration::from_secs(3600), // 1 hour
            enable_rate_limiting: true,
        }
    }
}

/// SYN flood protection state
#[derive(Debug)]
struct SynProtection {
    /// Half-open connections by IP
    half_open_connections: HashMap<IpAddr, Vec<SynCookie>>,
    /// SYN cookies
    cookie_counter: AtomicU64,
    /// Configuration
    config: DoSConfig,
}

/// SYN cookie information
#[derive(Debug, Clone)]
pub struct SynCookie {
    /// Cookie value
    value: u32,
    /// When issued
    issued_at: Instant,
    /// Peer ID
    _peer_id: Option<PeerId>,
}

impl SynCookie {
    /// Get the cookie value
    pub fn value(&self) -> u32 {
        self.value
    }
}

/// Connection flood detection
#[derive(Debug)]
struct ConnectionFloodDetector {
    /// Connection attempts by time window
    connection_attempts: Vec<Instant>,
    /// Configuration
    config: DoSConfig,
}

/// Bandwidth usage tracking
#[derive(Debug)]
struct BandwidthTracker {
    /// Bandwidth usage by peer
    peer_usage: HashMap<PeerId, BandwidthUsage>,
    /// Global bandwidth usage
    _global_usage: BandwidthUsage,
    /// Configuration
    config: DoSConfig,
}

/// Bandwidth usage for a peer
#[derive(Debug, Clone)]
struct BandwidthUsage {
    /// Bytes sent in current window
    bytes_sent: u64,
    /// Bytes received in current window
    bytes_received: u64,
    /// Window start time
    window_start: Instant,
    /// Peak usage rate
    peak_rate: u64,
}

/// Pattern detection state
#[derive(Debug)]
struct PatternDetector {
    /// Activity patterns by peer
    peer_patterns: HashMap<PeerId, ActivityPattern>,
    /// Configuration
    config: DoSConfig,
}

/// Activity pattern for a peer
#[derive(Debug, Clone)]
struct ActivityPattern {
    /// Messages per time window
    message_frequency: Vec<u32>,
    /// Connection attempts
    connection_attempts: u32,
    /// Failed operations
    failed_operations: u32,
    /// Last activity
    last_activity: Instant,
    /// Suspicion score (0.0 to 1.0)
    suspicion_score: f64,
}

/// Comprehensive DoS protection system
#[derive(Debug)]
pub struct DoSProtection {
    /// SYN flood protection
    syn_protection: SynProtection,
    /// Connection flood detection
    connection_detector: ConnectionFloodDetector,
    /// Bandwidth tracking
    bandwidth_tracker: BandwidthTracker,
    /// Pattern detection
    pattern_detector: PatternDetector,
    /// Statistics
    stats: DoSStats,
    /// Detected attacks
    detected_attacks: Vec<AttackInfo>,
}

/// Attack information
#[derive(Debug, Clone)]
pub struct AttackInfo {
    /// Attack type
    pub attack_type: DoSError,
    /// Source IP
    pub source_ip: Option<IpAddr>,
    /// Source peer ID (if known)
    pub source_peer: Option<PeerId>,
    /// When detected
    pub detected_at: Instant,
    /// Attack severity
    pub severity: AttackSeverity,
    /// Mitigation actions taken
    pub mitigations: Vec<String>,
}

/// Attack severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttackSeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// DoS protection statistics
#[derive(Debug, Clone, Default)]
pub struct DoSStats {
    /// Number of SYN floods detected
    pub syn_floods_detected: u64,
    /// Number of connection floods detected
    pub connection_floods_detected: u64,
    /// Number of bandwidth attacks detected
    pub bandwidth_attacks_detected: u64,
    /// Number of pattern attacks detected
    pub pattern_attacks_detected: u64,
    /// Total number of attacks detected
    pub total_attacks_detected: u64,
    /// Number of automatic bans issued
    pub auto_bans_issued: u64,
    /// Number of mitigations applied
    pub mitigations_applied: u64,
}

impl DoSProtection {
    /// Create new DoS protection system
    pub fn new(config: DoSConfig) -> Self {
        Self {
            syn_protection: SynProtection {
                half_open_connections: HashMap::new(),
                cookie_counter: AtomicU64::new(0),
                config: config.clone(),
            },
            connection_detector: ConnectionFloodDetector {
                connection_attempts: Vec::new(),
                config: config.clone(),
            },
            bandwidth_tracker: BandwidthTracker {
                peer_usage: HashMap::new(),
                _global_usage: BandwidthUsage {
                    bytes_sent: 0,
                    bytes_received: 0,
                    window_start: Instant::now(),
                    peak_rate: 0,
                },
                config: config.clone(),
            },
            pattern_detector: PatternDetector {
                peer_patterns: HashMap::new(),
                config: config.clone(),
            },
            stats: DoSStats::default(),
            detected_attacks: Vec::new(),
        }
    }

    /// Handle incoming SYN packet (connection attempt)
    pub fn handle_syn_packet(
        &mut self,
        source_ip: IpAddr,
        peer_id: Option<PeerId>,
    ) -> Result<Option<SynCookie>, DoSError> {
        if !self.syn_protection.config.enable_syn_protection {
            return Ok(None);
        }

        // Check half-open connections limit
        let half_open = self
            .syn_protection
            .half_open_connections
            .entry(source_ip)
            .or_default();

        if half_open.len() >= self.syn_protection.config.max_half_open_per_ip {
            let attack = AttackInfo {
                attack_type: DoSError::SynFlood,
                source_ip: Some(source_ip),
                source_peer: peer_id.clone(),
                detected_at: Instant::now(),
                severity: AttackSeverity::High,
                mitigations: vec![
                    "SYN flood detected".to_string(),
                    format!("Limiting connections from {}", source_ip),
                ],
            };

            self.detected_attacks.push(attack.clone());
            self.stats.syn_floods_detected += 1;

            if self.syn_protection.config.auto_ban_on_detection {
                return Err(DoSError::SynFlood);
            }
        }

        // Generate SYN cookie
        let cookie = self
            .syn_protection
            .cookie_counter
            .fetch_add(1, Ordering::Relaxed);
        let syn_cookie = SynCookie {
            value: cookie as u32,
            issued_at: Instant::now(),
            _peer_id: peer_id.clone(),
        };

        half_open.push(syn_cookie.clone());
        Ok(Some(syn_cookie))
    }

    /// Validate SYN cookie
    pub fn validate_syn_cookie(
        &mut self,
        source_ip: IpAddr,
        cookie_value: u32,
    ) -> Result<bool, DoSError> {
        let half_open = self
            .syn_protection
            .half_open_connections
            .get(&source_ip)
            .map(|cookies| {
                cookies.iter().find(|c| {
                    c.value == cookie_value
                        && c.issued_at.elapsed() < self.syn_protection.config.syn_cookie_timeout
                })
            });

        if let Some(_cookie) = half_open {
            // Remove used cookie
            if let Some(cookies) = self
                .syn_protection
                .half_open_connections
                .get_mut(&source_ip)
            {
                cookies.retain(|c| c.value != cookie_value);
                if cookies.is_empty() {
                    self.syn_protection.half_open_connections.remove(&source_ip);
                }
            }

            // Complete connection
            Ok(true)
        } else {
            // Invalid cookie - potential attack
            let attack = AttackInfo {
                attack_type: DoSError::SynFlood,
                source_ip: Some(source_ip),
                source_peer: None,
                detected_at: Instant::now(),
                severity: AttackSeverity::Medium,
                mitigations: vec![
                    "Invalid SYN cookie".to_string(),
                    format!("Rejecting connection from {}", source_ip),
                ],
            };

            self.detected_attacks.push(attack.clone());
            self.stats.syn_floods_detected += 1;
            Ok(false)
        }
    }

    /// Track new connection
    pub fn track_connection(&mut self, peer_id: PeerId, source_ip: IpAddr) -> Result<(), DoSError> {
        if !self.syn_protection.config.enable_syn_protection {
            return Ok(());
        }

        // Check connection flood
        let now = Instant::now();
        self.connection_detector.connection_attempts.push(now);

        // Clean old attempts
        let cutoff = now - self.connection_detector.config.connection_flood_window;
        self.connection_detector
            .connection_attempts
            .retain(|&timestamp| timestamp > cutoff);

        // Check for flood
        if self.connection_detector.connection_attempts.len() as u32
            >= self.connection_detector.config.connection_flood_threshold
        {
            let attack = AttackInfo {
                attack_type: DoSError::ConnectionFlood,
                source_ip: Some(source_ip),
                source_peer: Some(peer_id.clone()),
                detected_at: now,
                severity: AttackSeverity::High,
                mitigations: vec![
                    "Connection flood detected".to_string(),
                    format!("Rate limiting connections from {}", source_ip),
                ],
            };

            self.detected_attacks.push(attack.clone());
            self.stats.connection_floods_detected += 1;

            if self.connection_detector.config.auto_ban_on_detection {
                return Err(DoSError::ConnectionFlood);
            }
        }

        Ok(())
    }

    /// Track bandwidth usage
    pub fn track_bandwidth(
        &mut self,
        peer_id: PeerId,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Result<(), DoSError> {
        if !self.bandwidth_tracker.config.enable_bandwidth_protection {
            return Ok(());
        }

        let now = Instant::now();
        let usage = self
            .bandwidth_tracker
            .peer_usage
            .entry(peer_id.clone())
            .or_insert_with(|| BandwidthUsage {
                bytes_sent: 0,
                bytes_received: 0,
                window_start: now,
                peak_rate: 0,
            });

        // Reset window if needed
        if now.duration_since(usage.window_start)
            > self.bandwidth_tracker.config.bandwidth_window_duration
        {
            usage.bytes_sent = 0;
            usage.bytes_received = 0;
            usage.window_start = now;
        }

        // Check bandwidth limits
        let total_bytes = usage.bytes_sent + bytes_sent;
        if total_bytes > self.bandwidth_tracker.config.max_bandwidth_per_peer {
            let attack = AttackInfo {
                attack_type: DoSError::BandwidthAttack,
                source_ip: None, // Unknown at this level
                source_peer: Some(peer_id.clone()),
                detected_at: now,
                severity: AttackSeverity::Medium,
                mitigations: vec![
                    "Bandwidth limit exceeded".to_string(),
                    "Throttling peer bandwidth".to_string(),
                ],
            };

            self.detected_attacks.push(attack.clone());
            self.stats.bandwidth_attacks_detected += 1;

            if self.bandwidth_tracker.config.auto_ban_on_detection {
                return Err(DoSError::BandwidthAttack);
            }
        }

        usage.bytes_sent += bytes_sent;
        usage.bytes_received += bytes_received;

        // Update peak rate
        let elapsed = now.duration_since(usage.window_start);
        if !elapsed.is_zero() {
            let secs = elapsed.as_secs().max(1);
            let current_rate = total_bytes / secs;
            usage.peak_rate = usage.peak_rate.max(current_rate);
        }

        Ok(())
    }

    /// Analyze peer behavior patterns
    pub fn analyze_pattern(
        &mut self,
        peer_id: PeerId,
        message_count: u32,
        failed_operations: u32,
    ) -> Result<(), DoSError> {
        if !self.pattern_detector.config.enable_pattern_detection {
            return Ok(());
        }

        let now = Instant::now();
        let pattern = self
            .pattern_detector
            .peer_patterns
            .entry(peer_id.clone())
            .or_insert_with(|| ActivityPattern {
                message_frequency: Vec::new(),
                connection_attempts: 0,
                failed_operations: 0,
                last_activity: now,
                suspicion_score: 0.0,
            });

        // Update pattern
        pattern.message_frequency.push(message_count);
        pattern.failed_operations += failed_operations;
        pattern.last_activity = now;

        // Calculate suspicion score
        let avg_frequency = pattern.message_frequency.iter().sum::<u32>() as f64
            / pattern.message_frequency.len() as f64;
        let failure_rate =
            pattern.failed_operations as f64 / (pattern.connection_attempts + 1) as f64;

        // High frequency + high failure rate = suspicious
        pattern.suspicion_score = (avg_frequency / 100.0).min(1.0) + (failure_rate * 0.5).min(0.5);

        if pattern.suspicion_score >= self.pattern_detector.config.suspicious_pattern_threshold {
            let attack = AttackInfo {
                attack_type: DoSError::SuspiciousPattern,
                source_ip: None,
                source_peer: Some(peer_id.clone()),
                detected_at: now,
                severity: AttackSeverity::Medium,
                mitigations: vec![
                    "Suspicious activity pattern".to_string(),
                    format!(
                        "Peer {} suspicion score: {:.2}",
                        peer_id, pattern.suspicion_score
                    ),
                ],
            };

            self.detected_attacks.push(attack.clone());
            self.stats.pattern_attacks_detected += 1;

            if self.pattern_detector.config.auto_ban_on_detection {
                return Err(DoSError::SuspiciousPattern);
            }
        }

        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &DoSStats {
        &self.stats
    }

    /// Get detected attacks
    pub fn get_detected_attacks(&self) -> &[AttackInfo] {
        &self.detected_attacks
    }

    /// Clear old attacks
    pub fn cleanup(&mut self) {
        let cutoff = Instant::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or_else(Instant::now); // Keep 1 hour, safe on low uptime
        self.detected_attacks
            .retain(|attack| attack.detected_at > cutoff);
    }

    /// Check if IP is currently under attack
    pub fn is_ip_under_attack(&self, ip: &IpAddr) -> bool {
        let now = Instant::now();
        let recent_cutoff = now - Duration::from_secs(60); // Last minute

        self.detected_attacks
            .iter()
            .any(|attack| attack.source_ip == Some(*ip) && attack.detected_at > recent_cutoff)
    }

    /// Get attack severity distribution
    pub fn get_attack_severity_distribution(&self) -> HashMap<AttackSeverity, usize> {
        let mut distribution = HashMap::new();

        for attack in &self.detected_attacks {
            *distribution.entry(attack.severity).or_insert(0) += 1;
        }

        distribution
    }

    /// Apply OS-level TCP protections
    #[cfg(target_os = "linux")]
    pub fn apply_tcp_protections(
        &self,
        socket: &std::net::TcpListener,
    ) -> Result<(), std::io::Error> {
        use std::os::unix::io::AsRawFd;

        let fd = socket.as_raw_fd();

        // SECURITY: Validate file descriptor before using in unsafe code
        if fd < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid file descriptor",
            ));
        }

        // Enable TCP_DEFER_ACCEPT (reduces SYN flood impact)
        let defer_accept: libc::c_int = 1;
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_DEFER_ACCEPT,
                &defer_accept as *const _ as *const libc::c_void,
                std::mem::size_of_val(&defer_accept) as libc::socklen_t,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Enable TCP_SYNCOOKIES (SYN flood protection)
        // Commented out due to compilation issues on macOS
        /*
        let syn_cookies: libc::c_int = 1;
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_SYNCOOKIES,
                &syn_cookies as *const _ as *const libc::c_void,
                std::mem::size_of_val(&syn_cookies) as libc::socklen_t,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
        */

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syn_flood_detection() {
        let config = DoSConfig {
            max_half_open_per_ip: 3,
            ..Default::default()
        };
        let mut protection = DoSProtection::new(config);
        let ip = "192.168.1.100".parse().expect("Invalid IP address in test");

        // Should allow normal connections
        for _ in 0..3 {
            assert!(protection.handle_syn_packet(ip, None).is_ok());
        }

        // Should detect SYN flood
        let result = protection.handle_syn_packet(ip, None);
        assert!(matches!(result, Err(DoSError::SynFlood)));
    }

    #[test]
    fn test_syn_cookie_validation() {
        let config = DoSConfig::default();
        let mut protection = DoSProtection::new(config);
        let ip = "192.168.1.100".parse().expect("Invalid IP address in test");

        // Generate valid cookie
        let cookie = protection
            .handle_syn_packet(ip, None)
            .expect("Handle SYN packet should succeed in test")
            .expect("SYN cookie should be generated in test");
        assert!(protection
            .validate_syn_cookie(ip, cookie.value)
            .expect("Valid cookie should validate successfully"));

        // Invalid cookie should fail
        assert!(!protection
            .validate_syn_cookie(ip, 99999)
            .expect("Invalid cookie should return false"));
    }

    #[test]
    fn test_connection_flood_detection() {
        let config = DoSConfig {
            connection_flood_threshold: 5,
            ..Default::default()
        };
        let mut protection = DoSProtection::new(config);
        let ip = "192.168.1.100".parse().expect("Invalid IP address in test");
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Should allow normal connections
        for _ in 0..4 {
            assert!(protection.track_connection(peer.clone(), ip).is_ok());
        }

        // Should detect flood
        let result = protection.track_connection(peer, ip);
        assert!(matches!(result, Err(DoSError::ConnectionFlood)));
    }

    #[test]
    fn test_bandwidth_tracking() {
        let config = DoSConfig {
            max_bandwidth_per_peer: 1000,
            ..Default::default()
        };
        let mut protection = DoSProtection::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Should allow normal bandwidth
        assert!(protection.track_bandwidth(peer.clone(), 500, 500).is_ok());

        // Should detect bandwidth attack
        let result = protection.track_bandwidth(peer, 600, 600);
        assert!(matches!(result, Err(DoSError::BandwidthAttack)));
    }

    #[test]
    fn test_pattern_analysis() {
        let config = DoSConfig {
            suspicious_pattern_threshold: 0.7,
            ..Default::default()
        };
        let mut protection = DoSProtection::new(config);
        let peer = format!("test_peer_{}", rand::random::<u64>());

        // Normal activity
        assert!(protection.analyze_pattern(peer.clone(), 10, 0).is_ok());

        // Suspicious activity
        let result = protection.analyze_pattern(peer, 100, 50);
        assert!(matches!(result, Err(DoSError::SuspiciousPattern)));
    }
}
