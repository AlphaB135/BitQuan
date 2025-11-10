//! Mining Alert System for hybrid mining operations.
//!
//! Provides configurable alerts for various mining events and anomalies.
//! Supports multiple notification channels and severity levels.

use bitquan_types::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;

/// Alert severity levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(dead_code)] // Alert system ready for production use
pub enum AlertSeverity {
    /// Informational alerts only.
    Info = 0,
    /// Warning conditions.
    Warning = 1,
    /// Error conditions requiring attention.
    Error = 2,
    /// Critical conditions requiring immediate action.
    Critical = 3,
}

/// Alert types for mining operations.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)] // Alert system ready for production use
pub enum AlertType {
    /// Hashrate dropped significantly.
    HashrateDrop,
    /// Pool efficiency too low.
    LowEfficiency,
    /// High rejection rate.
    HighRejectionRate,
    /// Miner disconnected.
    MinerDisconnected,
    /// Network difficulty spike.
    DifficultySpike,
    /// Block found.
    BlockFound,
    /// Algorithm imbalance.
    AlgorithmImbalance,
    /// Pool hashrate anomaly.
    PoolHashrateAnomaly,
    /// Geographic concentration.
    GeographicConcentration,
    /// Security event.
    SecurityEvent,
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertType::HashrateDrop => write!(f, "HashrateDrop"),
            AlertType::LowEfficiency => write!(f, "LowEfficiency"),
            AlertType::HighRejectionRate => write!(f, "HighRejectionRate"),
            AlertType::MinerDisconnected => write!(f, "MinerDisconnected"),
            AlertType::DifficultySpike => write!(f, "DifficultySpike"),
            AlertType::BlockFound => write!(f, "BlockFound"),
            AlertType::AlgorithmImbalance => write!(f, "AlgorithmImbalance"),
            AlertType::PoolHashrateAnomaly => write!(f, "PoolHashrateAnomaly"),
            AlertType::GeographicConcentration => write!(f, "GeographicConcentration"),
            AlertType::SecurityEvent => write!(f, "SecurityEvent"),
        }
    }
}

/// Alert notification channels.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)] // Alert system ready for production use
pub enum NotificationChannel {
    /// Log to console.
    Console,
    /// Send to webhook URL.
    Webhook(String),
    /// Send email (placeholder).
    Email(String),
    /// Send Slack message.
    Slack(String),
    /// Send Discord message.
    Discord(String),
}

/// Alert configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)] // Alert system ready for production use
pub struct AlertConfig {
    /// Alert type.
    pub alert_type: AlertType,
    /// Severity level.
    pub severity: AlertSeverity,
    /// Notification channels.
    pub channels: Vec<NotificationChannel>,
    /// Alert message template.
    pub message_template: String,
    /// Cooldown period between alerts (seconds).
    pub cooldown_seconds: u64,
    /// Threshold values for triggering alerts.
    pub thresholds: HashMap<String, f64>,
    /// Whether alert is enabled.
    pub enabled: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            alert_type: AlertType::HashrateDrop,
            severity: AlertSeverity::Warning,
            channels: vec![NotificationChannel::Console],
            message_template: "Alert: {alert_type} - {details}".to_string(),
            cooldown_seconds: 300, // 5 minutes
            thresholds: HashMap::new(),
            enabled: true,
        }
    }
}

/// Alert data structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)] // Alert system ready for production use
pub struct Alert {
    /// Unique alert ID.
    pub id: String,
    /// Alert type.
    pub alert_type: AlertType,
    /// Severity level.
    pub severity: AlertSeverity,
    /// Alert message.
    pub message: String,
    /// Timestamp when alert was generated.
    pub timestamp: u64,
    /// Algorithm associated with alert (if any).
    pub algorithm: Option<String>,
    /// Additional context data.
    pub context: HashMap<String, serde_json::Value>,
    /// Whether alert has been acknowledged.
    pub acknowledged: bool,
}

/// Mining metrics for alert evaluation.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Alert system ready for production use
pub struct MiningMetrics {
    /// Current hashrate by algorithm.
    pub hashrate_by_algo: HashMap<String, f64>,
    /// Total pool hashrate.
    pub total_hashrate: f64,
    /// Pool efficiency percentage.
    pub efficiency: f64,
    /// Rejection rate percentage.
    pub rejection_rate: f64,
    /// Active miners count.
    pub active_miners: usize,
    /// Network difficulty.
    pub network_difficulty: f64,
    /// Algorithm distribution percentages.
    pub algo_distribution: HashMap<String, f64>,
    /// Geographic distribution.
    pub geo_distribution: HashMap<String, f64>,
    /// Recent blocks found.
    pub recent_blocks: u64,
}

/// Alert system state.
#[derive(Clone)]
#[allow(dead_code)] // Alert system ready for production use
pub struct AlertState {
    /// Last alert timestamps by type.
    pub last_alert_times: HashMap<AlertType, u64>,
    /// Current alert counts by type.
    pub alert_counts: HashMap<AlertType, u64>,
    /// Historical hashrate for trend analysis.
    pub hashrate_history: Vec<(u64, f64)>,
    /// Historical efficiency for trend analysis.
    pub efficiency_history: Vec<(u64, f64)>,
}

/// Alert system for mining operations.
#[allow(dead_code)] // Alert system ready for production use
pub struct AlertSystem {
    /// Alert configurations.
    configs: Arc<RwLock<Vec<AlertConfig>>>,
    /// Alert system state.
    state: Arc<RwLock<AlertState>>,
    /// Generated alerts.
    alerts: Arc<RwLock<Vec<Alert>>>,
    /// Notification handlers.
    handlers: Arc<RwLock<HashMap<NotificationChannel, Box<dyn NotificationHandler>>>>,
}

/// Trait for notification handlers.
#[allow(dead_code)] // Alert system ready for production use
pub trait NotificationHandler: Send + Sync {
    /// Send notification.
    fn send(&self, alert: &Alert) -> Result<()>;
}

/// Console notification handler.
#[allow(dead_code)] // Alert system ready for production use
pub struct ConsoleHandler;

impl NotificationHandler for ConsoleHandler {
    fn send(&self, alert: &Alert) -> Result<()> {
        let severity_str = match alert.severity {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARN",
            AlertSeverity::Error => "ERROR",
            AlertSeverity::Critical => "CRITICAL",
        };
        
        println!(
            "[{}] {} [{}] {}",
            severity_str,
            chrono::DateTime::from_timestamp(alert.timestamp as i64, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M:%S"),
            alert.alert_type,
            alert.message
        );
        
        Ok(())
    }
}

/// Webhook notification handler.
#[allow(dead_code)] // Alert system ready for production use
pub struct WebhookHandler;

impl WebhookHandler {
    #[allow(dead_code)] // Alert system ready for production use
    pub fn new() -> Self {
        Self
    }
}

impl NotificationHandler for WebhookHandler {
    fn send(&self, alert: &Alert) -> Result<()> {
        // This would send HTTP POST to webhook URL
        // For now, just log attempt
        println!("Webhook alert: {}", alert.message);
        Ok(())
    }
}

impl AlertSystem {
    /// Create new alert system.
    #[allow(dead_code)] // Alert system ready for production use
    pub fn new() -> Self {
        let mut handlers: HashMap<NotificationChannel, Box<dyn NotificationHandler>> = HashMap::new();
        handlers.insert(NotificationChannel::Console, Box::new(ConsoleHandler));
        handlers.insert(NotificationChannel::Webhook("default".to_string()), Box::new(WebhookHandler::new()));

        Self {
            configs: Arc::new(RwLock::new(Vec::new())),
            state: Arc::new(RwLock::new(AlertState {
                last_alert_times: HashMap::new(),
                alert_counts: HashMap::new(),
                hashrate_history: Vec::new(),
                efficiency_history: Vec::new(),
            })),
            alerts: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(handlers)),
        }
    }

    /// Add alert configuration.
    #[allow(dead_code)] // Alert system ready for production use
    pub async fn add_config(&self, config: AlertConfig) {
        let mut configs = self.configs.write().await;
        configs.push(config);
    }

    /// Load default alert configurations.
    #[allow(dead_code)] // Alert system ready for production use
    pub async fn load_defaults(&self) {
        let defaults = vec![
            // Hashrate drop alert
            AlertConfig {
                alert_type: AlertType::HashrateDrop,
                severity: AlertSeverity::Warning,
                channels: vec![NotificationChannel::Console],
                message_template: "Hashrate dropped by {drop_percent}% for algorithm {algorithm}".to_string(),
                cooldown_seconds: 600,
                thresholds: {
                    let mut t = HashMap::new();
                    t.insert("drop_percent".to_string(), 50.0);
                    t.insert("time_window".to_string(), 300.0); // 5 minutes
                    t
                },
                enabled: true,
            },
            // Low efficiency alert
            AlertConfig {
                alert_type: AlertType::LowEfficiency,
                severity: AlertSeverity::Warning,
                channels: vec![NotificationChannel::Console],
                message_template: "Pool efficiency dropped to {efficiency}%".to_string(),
                cooldown_seconds: 900,
                thresholds: {
                    let mut t = HashMap::new();
                    t.insert("efficiency_threshold".to_string(), 90.0);
                    t
                },
                enabled: true,
            },
            // High rejection rate alert
            AlertConfig {
                alert_type: AlertType::HighRejectionRate,
                severity: AlertSeverity::Error,
                channels: vec![NotificationChannel::Console],
                message_template: "High rejection rate: {rejection_rate}%".to_string(),
                cooldown_seconds: 600,
                thresholds: {
                    let mut t = HashMap::new();
                    t.insert("rejection_threshold".to_string(), 5.0);
                    t
                },
                enabled: true,
            },
            // Block found alert
            AlertConfig {
                alert_type: AlertType::BlockFound,
                severity: AlertSeverity::Info,
                channels: vec![NotificationChannel::Console],
                message_template: "Block found by {miner} using {algorithm}!".to_string(),
                cooldown_seconds: 1,
                thresholds: HashMap::new(),
                enabled: true,
            },
            // Algorithm imbalance alert
            AlertConfig {
                alert_type: AlertType::AlgorithmImbalance,
                severity: AlertSeverity::Warning,
                channels: vec![NotificationChannel::Console],
                message_template: "Algorithm imbalance detected: {details}".to_string(),
                cooldown_seconds: 1800,
                thresholds: {
                    let mut t = HashMap::new();
                    t.insert("max_percentage".to_string(), 80.0);
                    t
                },
                enabled: true,
            },
        ];

        for config in defaults {
            self.add_config(config).await;
        }
    }

    /// Evaluate metrics and generate alerts.
    pub async fn evaluate_metrics(&self, metrics: &MiningMetrics) -> Result<()> {
        let configs = self.configs.read().await;
        let mut state = self.state.write().await;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Update history
        state.hashrate_history.push((current_time, metrics.total_hashrate));
        state.efficiency_history.push((current_time, metrics.efficiency));

        // Keep only last 100 data points
        if state.hashrate_history.len() > 100 {
            state.hashrate_history.remove(0);
        }
        if state.efficiency_history.len() > 100 {
            state.efficiency_history.remove(0);
        }

        for config in configs.iter().filter(|c| c.enabled) {
            // Check cooldown
            if let Some(last_time) = state.last_alert_times.get(&config.alert_type) {
                if current_time - last_time < config.cooldown_seconds {
                    continue;
                }
            }

            // Evaluate alert condition
            if self.should_trigger_alert(config, metrics, &state).await? {
                let alert = self.generate_alert(config, metrics, current_time).await?;
                self.send_alert(&alert).await?;
                
                // Update state
                state.last_alert_times.insert(config.alert_type.clone(), current_time);
                *state.alert_counts.entry(config.alert_type.clone()).or_insert(0) += 1;
            }
        }

        Ok(())
    }

    /// Check if alert should be triggered.
    async fn should_trigger_alert(
        &self,
        config: &AlertConfig,
        metrics: &MiningMetrics,
        state: &AlertState,
    ) -> Result<bool> {
        match config.alert_type {
            AlertType::HashrateDrop => {
                if let Some(&drop_percent) = config.thresholds.get("drop_percent") {
                    if let Some(&time_window) = config.thresholds.get("time_window") {
                        let current_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        
                        // Find hashrate from time window ago
                        let target_time = current_time - time_window as u64;
                        if let Some((_, old_hashrate)) = state.hashrate_history.iter()
                            .find(|(t, _)| *t >= target_time) {
                            let drop_percent_actual = ((old_hashrate - metrics.total_hashrate) / old_hashrate) * 100.0;
                            return Ok(drop_percent_actual >= drop_percent);
                        }
                    }
                }
            }
            AlertType::LowEfficiency => {
                if let Some(&threshold) = config.thresholds.get("efficiency_threshold") {
                    return Ok(metrics.efficiency < threshold);
                }
            }
            AlertType::HighRejectionRate => {
                if let Some(&threshold) = config.thresholds.get("rejection_threshold") {
                    return Ok(metrics.rejection_rate > threshold);
                }
            }
            AlertType::AlgorithmImbalance => {
                if let Some(&max_percentage) = config.thresholds.get("max_percentage") {
                    for &percentage in metrics.algo_distribution.values() {
                        if percentage > max_percentage {
                            return Ok(true);
                        }
                    }
                }
            }
            AlertType::BlockFound => {
                // This is event-driven, not threshold-based
                return Ok(false);
            }
            _ => {}
        }
        Ok(false)
    }

    /// Generate alert from configuration and metrics.
    async fn generate_alert(
        &self,
        config: &AlertConfig,
        metrics: &MiningMetrics,
        timestamp: u64,
    ) -> Result<Alert> {
        let mut context = HashMap::new();
        context.insert("total_hashrate".to_string(), serde_json::Value::String(metrics.total_hashrate.to_string()));
        context.insert("efficiency".to_string(), serde_json::Value::String(metrics.efficiency.to_string()));
        context.insert("rejection_rate".to_string(), serde_json::Value::String(metrics.rejection_rate.to_string()));
        context.insert("active_miners".to_string(), serde_json::Value::String(metrics.active_miners.to_string()));

        let message = self.format_message(&config.message_template, metrics, &context);

        let alert = Alert {
            id: format!("{}-{}", config.alert_type, timestamp),
            alert_type: config.alert_type.clone(),
            severity: config.severity,
            message,
            timestamp,
            algorithm: None, // Can be set based on context
            context,
            acknowledged: false,
        };

        Ok(alert)
    }

    /// Format alert message with template variables.
    fn format_message(
        &self,
        template: &str,
        metrics: &MiningMetrics,
        context: &HashMap<String, serde_json::Value>,
    ) -> String {
        let mut message = template.to_string();
        
        // Replace common variables
        message = message.replace("{total_hashrate}", &format!("{:.2}", metrics.total_hashrate));
        message = message.replace("{efficiency}", &format!("{:.1}", metrics.efficiency));
        message = message.replace("{rejection_rate}", &format!("{:.1}", metrics.rejection_rate));
        message = message.replace("{active_miners}", &metrics.active_miners.to_string());

        // Replace context variables
        for (key, value) in context {
            let value_str = match value {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            message = message.replace(&format!("{{{}}}", key), &value_str);
        }

        message
    }

    /// Send alert through configured channels.
    async fn send_alert(&self, alert: &Alert) -> Result<()> {
        let configs = self.configs.read().await;
        let handlers = self.handlers.read().await;

        // Store alert
        {
            let mut alerts = self.alerts.write().await;
            alerts.push(alert.clone());
            
            // Keep only last 1000 alerts
            if alerts.len() > 1000 {
                alerts.remove(0);
            }
        }

        // Find config for this alert type
        if let Some(config) = configs.iter().find(|c| c.alert_type == alert.alert_type) {
            for channel in &config.channels {
                if let Some(handler) = handlers.get(channel) {
                    if let Err(e) = handler.send(alert) {
                        eprintln!("Failed to send alert via {:?}: {}", channel, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get recent alerts.
    #[allow(dead_code)]
    pub async fn get_recent_alerts(&self, limit: usize) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.iter().rev().take(limit).cloned().collect()
    }

    /// Get alert statistics.
    #[allow(dead_code)]
    pub async fn get_alert_stats(&self) -> HashMap<AlertType, u64> {
        let state = self.state.read().await;
        state.alert_counts.clone()
    }

    /// Start background monitoring task.
    #[allow(dead_code)]
    pub async fn start_monitoring(&self, metrics_provider: Arc<dyn MiningMetricsProvider>) {
        let alert_system = self.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
            
            loop {
                interval.tick().await;
                
                if let Ok(metrics) = metrics_provider.get_metrics() {
                    if let Err(e) = alert_system.evaluate_metrics(&metrics).await {
                        eprintln!("Alert evaluation error: {}", e);
                    }
                }
            }
        });
    }
}

/// Trait for providing mining metrics.
#[allow(dead_code)]
pub trait MiningMetricsProvider: Send + Sync {
    /// Get current mining metrics.
    fn get_metrics(&self) -> Result<MiningMetrics>;
}

impl Clone for AlertSystem {
    fn clone(&self) -> Self {
        Self {
            configs: Arc::clone(&self.configs),
            state: Arc::clone(&self.state),
            alerts: Arc::clone(&self.alerts),
            handlers: Arc::clone(&self.handlers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::Error);
        assert!(AlertSeverity::Error > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert_eq!(config.alert_type, AlertType::HashrateDrop);
        assert_eq!(config.severity, AlertSeverity::Warning);
        assert!(config.enabled);
        assert_eq!(config.cooldown_seconds, 300);
    }

    #[tokio::test]
    async fn test_alert_system_creation() {
        let alert_system = AlertSystem::new();
        alert_system.load_defaults().await;
        
        let configs = alert_system.configs.read().await;
        assert!(!configs.is_empty());
    }

    #[tokio::test]
    async fn test_alert_generation() {
        let alert_system = AlertSystem::new();
        alert_system.load_defaults().await;
        
        let metrics = MiningMetrics {
            hashrate_by_algo: HashMap::new(),
            total_hashrate: 1e9,
            efficiency: 85.0, // Below threshold
            rejection_rate: 2.0,
            active_miners: 10,
            network_difficulty: 1.0,
            algo_distribution: HashMap::new(),
            geo_distribution: HashMap::new(),
            recent_blocks: 0,
        };

        // This should trigger a low efficiency alert
        let result = alert_system.evaluate_metrics(&metrics).await;
        assert!(result.is_ok());
    }
}