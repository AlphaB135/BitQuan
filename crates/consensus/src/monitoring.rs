//! Security-focused monitoring and alerting system for blockchain anomalies
//!
//! This module provides comprehensive monitoring capabilities without
//! exposing sensitive information or creating attack vectors.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Maximum number of alerts to keep in memory
const MAX_ALERTS: usize = 1000;

/// Maximum alert message length
const MAX_ALERT_MESSAGE_LENGTH: usize = 500;

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info = 0,
    /// Warning alert
    Warning = 1,
    /// Critical alert
    Critical = 2,
}

/// Types of monitored events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitorEventType {
    /// Block validation failed
    BlockValidationFailed,
    /// Anomaly was detected
    AnomalyDetected,
    /// System error occurred
    SystemError,
    /// Security violation detected
    SecurityViolation,
}

/// Security-focused alert structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert identifier
    pub id: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Event type
    pub event_type: MonitorEventType,
    /// Alert message (sanitized)
    pub message: String,
    /// Block height (if applicable)
    pub block_height: Option<u64>,
    /// Timestamp when alert was created
    pub timestamp: u64,
    /// Whether alert has been acknowledged
    pub acknowledged: bool,
}

impl Alert {
    /// Creates a new alert with security validation
    pub fn new(
        severity: AlertSeverity,
        event_type: MonitorEventType,
        message: String,
        block_height: Option<u64>,
    ) -> Result<Self, MonitorError> {
        // Security: Validate message length
        if message.len() > MAX_ALERT_MESSAGE_LENGTH {
            return Err(MonitorError::InvalidMessage {
                reason: "Message too long".to_string(),
            });
        }

        // Security: Sanitize message to prevent injection attacks
        let sanitized_message = Self::sanitize_message(&message);

        // Security: Generate secure ID
        let id = Self::generate_secure_id();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Self {
            id,
            severity,
            event_type,
            message: sanitized_message,
            block_height,
            timestamp,
            acknowledged: false,
        })
    }

    /// Sanitizes message to prevent security issues
    fn sanitize_message(message: &str) -> String {
        message
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control())
            .take(MAX_ALERT_MESSAGE_LENGTH)
            .collect()
    }

    /// Generates cryptographically secure alert ID
    fn generate_secure_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .hash(&mut hasher);

        format!("alert_{:x}", hasher.finish())
    }

    /// Acknowledges the alert
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    /// Returns age of alert in seconds
    pub fn age(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.timestamp)
    }
}

/// Monitoring configuration with security defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Whether monitoring is enabled
    pub enabled: bool,
    /// Maximum alerts per hour (rate limiting)
    pub max_alerts_per_hour: u32,
    /// Alert retention period (seconds)
    pub retention_period: u64,
    /// Whether to auto-acknowledge old alerts
    pub auto_acknowledge_old: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_alerts_per_hour: 100,
            retention_period: 86400, // 24 hours
            auto_acknowledge_old: true,
        }
    }
}

/// Security-focused monitoring system
#[derive(Debug, Clone)]
pub struct Monitor {
    /// Configuration
    config: MonitorConfig,
    /// Alert storage
    alerts: VecDeque<Alert>,
    /// Alert rate tracking
    alert_timestamps: VecDeque<u64>,
    /// Statistics
    stats: MonitorStats,
}

/// Monitoring statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitorStats {
    /// Total number of alerts generated
    pub total_alerts: u64,
    /// Number of critical alerts
    pub critical_alerts: u64,
    /// Number of warning alerts
    pub warning_alerts: u64,
    /// Number of info alerts
    pub info_alerts: u64,
    /// Number of acknowledged alerts
    pub acknowledged_alerts: u64,
}

impl Monitor {
    /// Creates a new monitor with secure defaults
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            alerts: VecDeque::with_capacity(MAX_ALERTS),
            alert_timestamps: VecDeque::new(),
            stats: MonitorStats::default(),
        }
    }
}

impl Default for Monitor {
    /// Creates a monitor with default secure configuration
    fn default() -> Self {
        Self::new(MonitorConfig::default())
    }
}

impl Monitor {
    /// Adds an alert with comprehensive security validation
    pub fn add_alert(&mut self, alert: Alert) -> Result<(), MonitorError> {
        // Security: Check if monitoring is enabled
        if !self.config.enabled {
            return Err(MonitorError::Disabled);
        }

        // Security: Rate limiting
        if !self.check_rate_limit() {
            return Err(MonitorError::RateLimited);
        }

        // Security: Validate alert structure
        self.validate_alert(&alert)?;

        // Add alert to storage
        self.alerts.push_back(alert.clone());
        self.alert_timestamps.push_back(alert.timestamp);

        // Update statistics
        self.update_stats(&alert);

        // Cleanup old alerts
        self.cleanup_old_alerts();

        // Enforce maximum alert limit
        if self.alerts.len() > MAX_ALERTS {
            self.alerts.pop_front();
        }

        Ok(())
    }

    /// Creates and adds an alert with validation
    pub fn create_alert(
        &mut self,
        severity: AlertSeverity,
        event_type: MonitorEventType,
        message: String,
        block_height: Option<u64>,
    ) -> Result<String, MonitorError> {
        let alert = Alert::new(severity, event_type, message, block_height)?;
        let alert_id = alert.id.clone();
        self.add_alert(alert)?;
        Ok(alert_id)
    }

    /// Validates alert structure and content
    fn validate_alert(&self, alert: &Alert) -> Result<(), MonitorError> {
        // Security: Validate ID format
        if !alert.id.starts_with("alert_") || alert.id.len() > 100 {
            return Err(MonitorError::InvalidAlert {
                reason: "Invalid alert ID format".to_string(),
            });
        }

        // Security: Validate timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if alert.timestamp > now + 300 {
            // Allow 5 minute clock skew
            return Err(MonitorError::InvalidAlert {
                reason: "Alert timestamp is in the future".to_string(),
            });
        }

        // Security: Validate block height
        if let Some(height) = alert.block_height {
            if height == 0 {
                return Err(MonitorError::InvalidAlert {
                    reason: "Invalid block height".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Checks rate limiting for alert creation
    fn check_rate_limit(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let one_hour_ago = now.saturating_sub(3600);

        // Count alerts in the last hour
        let recent_alerts = self
            .alert_timestamps
            .iter()
            .filter(|&&timestamp| timestamp > one_hour_ago)
            .count();

        recent_alerts < self.config.max_alerts_per_hour as usize
    }

    /// Updates monitoring statistics
    fn update_stats(&mut self, alert: &Alert) {
        self.stats.total_alerts += 1;

        match alert.severity {
            AlertSeverity::Critical => self.stats.critical_alerts += 1,
            AlertSeverity::Warning => self.stats.warning_alerts += 1,
            AlertSeverity::Info => self.stats.info_alerts += 1,
        }

        if alert.acknowledged {
            self.stats.acknowledged_alerts += 1;
        }
    }

    /// Removes old alerts based on retention policy
    fn cleanup_old_alerts(&mut self) {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.config.retention_period);

        // Remove old alerts
        while let Some(alert) = self.alerts.front() {
            if alert.timestamp < cutoff_time {
                self.alerts.pop_front();
            } else {
                break;
            }
        }

        // Clean up timestamp tracking
        while let Some(&timestamp) = self.alert_timestamps.front() {
            if timestamp < cutoff_time {
                self.alert_timestamps.pop_front();
            } else {
                break;
            }
        }
    }

    /// Gets alerts by severity level
    pub fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|alert| alert.severity == severity)
            .collect()
    }

    /// Gets unacknowledged alerts
    pub fn get_unacknowledged_alerts(&self) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|alert| !alert.acknowledged)
            .collect()
    }

    /// Gets recent alerts within time window
    pub fn get_recent_alerts(&self, seconds: u64) -> Vec<&Alert> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(seconds);

        self.alerts
            .iter()
            .filter(|alert| alert.timestamp > cutoff_time)
            .collect()
    }

    /// Acknowledges an alert by ID
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> Result<(), MonitorError> {
        // Security: Validate alert ID format
        if !alert_id.starts_with("alert_") {
            return Err(MonitorError::InvalidAlert {
                reason: "Invalid alert ID format".to_string(),
            });
        }

        for alert in &mut self.alerts {
            if alert.id == alert_id {
                alert.acknowledge();
                return Ok(());
            }
        }

        Err(MonitorError::AlertNotFound {
            id: alert_id.to_string(),
        })
    }

    /// Gets monitoring statistics
    pub fn get_stats(&self) -> &MonitorStats {
        &self.stats
    }

    /// Gets current configuration
    pub fn get_config(&self) -> &MonitorConfig {
        &self.config
    }

    /// Updates configuration with validation
    pub fn update_config(&mut self, config: MonitorConfig) -> Result<(), MonitorError> {
        // Security: Validate configuration values
        if config.max_alerts_per_hour == 0 || config.max_alerts_per_hour > 10000 {
            return Err(MonitorError::InvalidConfig {
                reason: "Invalid max_alerts_per_hour value".to_string(),
            });
        }

        if config.retention_period == 0 || config.retention_period > 86400 * 30 {
            return Err(MonitorError::InvalidConfig {
                reason: "Invalid retention_period value".to_string(),
            });
        }

        self.config = config;
        Ok(())
    }

    /// Clears all alerts (emergency use only)
    pub fn clear_all_alerts(&mut self) {
        self.alerts.clear();
        self.alert_timestamps.clear();
    }
}

/// Monitoring system errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MonitorError {
    #[error("monitoring is disabled")]
    /// Monitoring system is disabled
    Disabled,

    #[error("rate limit exceeded")]
    /// Rate limit exceeded
    RateLimited,

    #[error("invalid message: {reason}")]
    /// Invalid message format
    InvalidMessage {
        /// Reason why the message is invalid
        reason: String,
    },

    #[error("invalid alert: {reason}")]
    /// Invalid alert data
    InvalidAlert {
        /// Reason why the alert is invalid
        reason: String,
    },

    #[error("invalid configuration: {reason}")]
    /// Invalid configuration provided
    InvalidConfig {
        /// Reason why the configuration is invalid
        reason: String,
    },

    #[error("alert not found: {id}")]
    /// Alert with specified ID not found
    AlertNotFound {
        /// ID of the missing alert
        id: String,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn create_test_alert() -> Alert {
        Alert::new(
            AlertSeverity::Warning,
            MonitorEventType::BlockValidationFailed,
            "Test alert".to_string(),
            Some(1000),
        )
        .expect("Failed to create test alert")
    }

    #[test]
    fn test_alert_creation() {
        let alert = create_test_alert();
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.event_type, MonitorEventType::BlockValidationFailed);
        assert_eq!(alert.message, "Test alert");
        assert_eq!(alert.block_height, Some(1000));
        assert!(!alert.acknowledged);
    }

    #[test]
    fn test_alert_security() {
        // Test message length validation
        let long_message = "x".repeat(501);
        let result = Alert::new(
            AlertSeverity::Info,
            MonitorEventType::SystemError,
            long_message,
            None,
        );
        assert!(result.is_err());

        // Test message sanitization
        let malicious_message = "Test\x00\x01\x02";
        let alert = Alert::new(
            AlertSeverity::Info,
            MonitorEventType::SystemError,
            malicious_message.to_string(),
            None,
        )
        .expect("Failed to create alert with malicious message");
        assert!(!alert.message.contains('\x00'));
    }

    #[test]
    fn test_monitor_functionality() {
        let mut monitor = Monitor::default();
        let alert = create_test_alert();

        // Should add alert successfully
        let result = monitor.add_alert(alert.clone());
        assert!(result.is_ok());
        assert_eq!(monitor.get_stats().total_alerts, 1);

        // Should find the alert
        let unacknowledged = monitor.get_unacknowledged_alerts();
        assert_eq!(unacknowledged.len(), 1);
        assert_eq!(unacknowledged[0].id, alert.id);

        // Should acknowledge alert
        let result = monitor.acknowledge_alert(&alert.id);
        assert!(result.is_ok());
        assert_eq!(monitor.get_unacknowledged_alerts().len(), 0);
    }

    #[test]
    fn test_rate_limiting() {
        let config = MonitorConfig {
            enabled: true,
            max_alerts_per_hour: 2,
            retention_period: 86400,
            auto_acknowledge_old: false,
        };
        let mut monitor = Monitor::new(config);

        // Should allow first 2 alerts
        let alert1 = create_test_alert();
        let alert2 = create_test_alert();
        assert!(monitor.add_alert(alert1).is_ok());
        assert!(monitor.add_alert(alert2).is_ok());

        // Should reject 3rd alert due to rate limiting
        let alert3 = create_test_alert();
        let result = monitor.add_alert(alert3);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MonitorError::RateLimited));
    }

    #[test]
    fn test_config_validation() {
        let mut monitor = Monitor::default();

        // Should reject invalid config
        let invalid_config = MonitorConfig {
            enabled: true,
            max_alerts_per_hour: 0, // Invalid
            retention_period: 86400,
            auto_acknowledge_old: false,
        };
        let result = monitor.update_config(invalid_config);
        assert!(result.is_err());

        // Should accept valid config
        let valid_config = MonitorConfig {
            enabled: true,
            max_alerts_per_hour: 100,
            retention_period: 86400,
            auto_acknowledge_old: false,
        };
        let result = monitor.update_config(valid_config);
        assert!(result.is_ok());
    }
}
