//! Security Configuration for BitQuan Network
//!
//! This module provides comprehensive configuration options for all
//! network security features including rate limiting, connection management,
//! reputation systems, and DoS protection.
//!
//! ## Configuration Structure
//!
//! - Rate limiting: Per-message-type and global limits
//! - Connection management: Global, per-IP, and directional limits
//! - Reputation: Scoring, penalties, and ban thresholds
//! - DoS protection: SYN flood, bandwidth, and pattern detection
//! - Monitoring: Statistics and alerting

use std::time::Duration;

use super::{
    rate_limiter::{RateLimitConfig, MessageTypeLimits},
    connection_manager::{ConnectionConfig},
    reputation::{ReputationConfig},
    ban_manager::{BanConfig},
    dos_protection::{DoSConfig},
};

/// Comprehensive security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,

    /// Connection management configuration
    pub connections: ConnectionConfig,

    /// Reputation management configuration
    pub reputation: ReputationConfig,

    /// Ban management configuration
    pub bans: BanConfig,

    /// DoS protection configuration
    pub dos_protection: DoSConfig,

    /// Global security settings
    pub global: GlobalSecurityConfig,
}

/// Global security configuration
#[derive(Debug, Clone)]
pub struct GlobalSecurityConfig {
    /// Enable all security features
    pub enable_security: bool,

    /// Security level preset
    pub security_level: SecurityLevel,

    /// Alert configuration
    pub alerts: AlertConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// Security level presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Minimal security (basic protection)
    Minimal,
    /// Standard security (recommended for most nodes)
    Standard,
    /// High security (for critical infrastructure)
    High,
    /// Maximum security (for high-value targets)
    Maximum,
}

/// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// Enable security alerts
    pub enable_alerts: bool,

    /// Alert channels
    pub alert_channels: Vec<AlertChannel>,

    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Alert channels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertChannel {
    Log,
    Email,
    Webhook,
    Slack,
    Telegram,
}

/// Alert thresholds
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// Ban rate threshold (bans per minute)
    pub ban_rate_threshold: u32,

    /// Connection flood threshold
    pub connection_flood_threshold: u32,

    /// Reputation score threshold
    pub reputation_threshold: i32,

    /// DoS attack threshold
    pub dos_attack_threshold: u32,
}

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Enable detailed monitoring
    pub enable_monitoring: bool,

    /// Statistics collection interval
    pub stats_interval: Duration,

    /// Export statistics
    pub export_stats: bool,

    /// Metrics retention period
    pub metrics_retention: Duration,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limiting: RateLimitConfig::default(),
            connections: ConnectionConfig::default(),
            reputation: ReputationConfig::default(),
            bans: BanConfig::default(),
            dos_protection: DoSConfig::default(),
            global: GlobalSecurityConfig::default(),
        }
    }
}

impl Default for GlobalSecurityConfig {
    fn default() -> Self {
        Self {
            enable_security: true,
            security_level: SecurityLevel::Standard,
            alerts: AlertConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enable_alerts: true,
            alert_channels: vec![AlertChannel::Log],
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            ban_rate_threshold: 10, // 10 bans per minute
            connection_flood_threshold: 100, // 100 connections per second
            reputation_threshold: -30, // Average score below -30
            dos_attack_threshold: 5, // 5 DoS attacks per minute
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            stats_interval: Duration::from_secs(60), // Every minute
            export_stats: true,
            metrics_retention: Duration::from_secs(86400), // 24 hours
        }
    }
}

impl SecurityConfig {
    /// Create security config for specific security level
    pub fn for_security_level(level: SecurityLevel) -> Self {
        let mut config = Self::default();

        match level {
            SecurityLevel::Minimal => {
                // Minimal protection settings
                config.rate_limiting.max_messages_per_window = 50;
                config.rate_limiting.violation_threshold = 5;
                config.connections.max_total_connections = 50;
                config.connections.max_inbound_connections = 40;
                config.connections.max_outbound_connections = 10;
                config.connections.max_connections_per_ip = 2;
                config.reputation.initial_score = 30;
                config.reputation.temp_ban_threshold = -20;
                config.reputation.perm_ban_threshold = -30;
                config.bans.default_temp_ban_duration = Duration::from_secs(1800); // 30 minutes
                config.bans.max_temp_ban_duration = Duration::from_secs(7200); // 2 hours
                config.dos_protection.max_connections_per_second = 25;
                config.dos_protection.connection_flood_threshold = 50;
                config.global.security_level = level;
            }

            SecurityLevel::Standard => {
                // Standard protection settings (default)
                // Uses default values
                config.global.security_level = level;
            }

            SecurityLevel::High => {
                // High protection settings
                config.rate_limiting.max_messages_per_window = 200;
                config.rate_limiting.violation_threshold = 2;
                config.connections.max_total_connections = 200;
                config.connections.max_inbound_connections = 160;
                config.connections.max_outbound_connections = 40;
                config.connections.max_connections_per_ip = 5;
                config.reputation.initial_score = 60;
                config.reputation.temp_ban_threshold = -40;
                config.reputation.perm_ban_threshold = -50;
                config.bans.default_temp_ban_duration = Duration::from_secs(3600); // 1 hour
                config.bans.max_temp_ban_duration = Duration::from_secs(14400); // 4 hours
                config.dos_protection.max_connections_per_second = 100;
                config.dos_protection.connection_flood_threshold = 200;
                config.global.security_level = level;
            }

            SecurityLevel::Maximum => {
                // Maximum protection settings
                config.rate_limiting.max_messages_per_window = 500;
                config.rate_limiting.violation_threshold = 1;
                config.connections.max_total_connections = 500;
                config.connections.max_inbound_connections = 400;
                config.connections.max_outbound_connections = 100;
                config.connections.max_connections_per_ip = 10;
                config.reputation.initial_score = 80;
                config.reputation.temp_ban_threshold = -50;
                config.reputation.perm_ban_threshold = -60;
                config.bans.default_temp_ban_duration = Duration::from_secs(7200); // 2 hours
                config.bans.max_temp_ban_duration = Duration::from_secs(28800); // 8 hours
                config.dos_protection.max_connections_per_second = 200;
                config.dos_protection.connection_flood_threshold = 500;
                config.global.security_level = level;
            }
        }

        config
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate rate limiting
        if self.rate_limiting.max_messages_per_window == 0 {
            return Err("Rate limiting must be enabled".to_string());
        }

        // Validate connections
        if self.connections.max_total_connections == 0 {
            return Err("Maximum connections must be greater than 0".to_string());
        }

        // Validate reputation
        if self.reputation.initial_score < 0 || self.reputation.initial_score > 100 {
            return Err("Initial reputation score must be between 0 and 100".to_string());
        }

        // Validate DoS protection
        if self.dos_protection.max_connections_per_second == 0 {
            return Err("DoS protection must have connection limits".to_string());
        }

        Ok(())
    }

    /// Get configuration summary
    pub fn get_summary(&self) -> String {
        format!(
            "Security Configuration (Level: {:?}):\n\
             Rate Limiting: {}/{} messages per window\n\
             Connections: {}/{} total ({} inbound, {} outbound)\n\
             Reputation: Score {}/{} to {}/{}\n\
             DoS Protection: {}/{} connections/sec\n\
             Security: {:?}",
            self.global.security_level,
            self.rate_limiting.max_messages_per_window,
            self.rate_limiting.violation_threshold,
            self.connections.max_total_connections,
            self.connections.max_inbound_connections,
            self.connections.max_outbound_connections,
            self.reputation.initial_score,
            self.reputation.perm_ban_threshold,
            self.dos_protection.max_connections_per_second,
            self.global.enable_security
        )
    }

    /// Export configuration to TOML format
    pub fn export_toml(&self) -> Result<String, toml::SerializationError> {
        toml::to_string_pretty(&self)
    }

    /// Import configuration from TOML format
    pub fn import_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
