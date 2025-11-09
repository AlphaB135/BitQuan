//! Circuit breaker system for automatic protection against anomalies.
//!
//! This module provides a comprehensive circuit breaker system that automatically
//! detects and responds to various types of anomalies including:
//! - High transaction failure rates
//! - Unusual network activity patterns
//! - Resource exhaustion attacks
//! - Consensus failures
//! - Memory/CPU usage spikes

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Default threshold for failure rate (percentage)
pub const DEFAULT_FAILURE_RATE_THRESHOLD: f64 = 10.0;

/// Default threshold for anomaly detection (standard deviations)
pub const DEFAULT_ANOMALY_THRESHOLD: f64 = 3.0;

/// Default window size for metrics collection (seconds)
pub const DEFAULT_METRICS_WINDOW: u64 = 60;

/// Default cooldown period after circuit breaker trips (seconds)
pub const DEFAULT_COOLDOWN_PERIOD: u64 = 300;

/// Default maximum pause duration (seconds)
pub const DEFAULT_MAX_PAUSE_DURATION: u64 = 3600;

/// Minimum number of samples required for anomaly detection
pub const MIN_SAMPLES_FOR_ANOMALY_DETECTION: usize = 10;

/// Errors that can occur during circuit breaker operations
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    /// Circuit breaker is already tripped
    #[error("circuit breaker is already tripped: {reason}")]
    AlreadyTripped { reason: String },

    /// Circuit breaker is not in a valid state
    #[error("circuit breaker state invalid: {reason}")]
    InvalidState { reason: String },

    /// Invalid configuration
    #[error("invalid circuit breaker configuration: {reason}")]
    InvalidConfiguration { reason: String },

    /// Operation not allowed in current state
    #[error("operation not allowed in state: {state}")]
    OperationNotAllowed { state: String },

    /// Metrics collection failed
    #[error("metrics collection failed: {reason}")]
    MetricsFailed { reason: String },
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed and operating normally
    Closed,
    /// Circuit is open and blocking operations
    Open,
    /// Circuit is half-open and testing recovery
    HalfOpen,
}

/// Types of anomalies that can trigger circuit breakers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// High transaction failure rate
    HighFailureRate,
    /// Unusual network activity
    NetworkAnomaly,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Consensus failure
    ConsensusFailure,
    /// Memory usage spike
    MemorySpike,
    /// CPU usage spike
    CpuSpike,
    /// Transaction volume anomaly
    VolumeAnomaly,
    /// Latency spike
    LatencySpike,
}

/// A single circuit breaker instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Unique identifier for this circuit breaker
    pub id: String,
    /// Current state
    pub state: CircuitState,
    /// Type of anomalies this breaker monitors
    pub anomaly_types: Vec<AnomalyType>,
    /// Configuration parameters
    pub config: CircuitBreakerConfig,
    /// Current metrics
    pub metrics: CircuitMetrics,
    /// State change history
    pub state_history: VecDeque<StateChange>,
    /// Last state change timestamp
    pub last_state_change: u64,
    /// Number of times this breaker has tripped
    pub trip_count: u64,
}

/// Configuration for a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure rate threshold (percentage)
    pub failure_rate_threshold: f64,
    /// Anomaly detection threshold (standard deviations)
    pub anomaly_threshold: f64,
    /// Metrics collection window (seconds)
    pub metrics_window: u64,
    /// Cooldown period after tripping (seconds)
    pub cooldown_period: u64,
    /// Maximum pause duration (seconds)
    pub max_pause_duration: u64,
    /// Whether automatic recovery is enabled
    pub auto_recovery: bool,
    /// Minimum samples required for anomaly detection
    pub min_samples: usize,
}

/// Metrics collected by the circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitMetrics {
    /// Total operations
    pub total_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Recent operation timestamps
    pub recent_operations: VecDeque<u64>,
    /// Recent failure timestamps
    pub recent_failures: VecDeque<u64>,
    /// Resource usage samples
    pub resource_samples: VecDeque<ResourceSample>,
    /// Network activity samples
    pub network_samples: VecDeque<NetworkSample>,
    /// Transaction volume samples
    pub volume_samples: VecDeque<VolumeSample>,
    /// Latency samples
    pub latency_samples: VecDeque<LatencySample>,
}

/// Resource usage sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSample {
    pub timestamp: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Network activity sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSample {
    pub timestamp: u64,
    pub connections_count: u32,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub packets_dropped: u32,
}

/// Transaction volume sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSample {
    pub timestamp: u64,
    pub transaction_count: u64,
    pub total_value: f64,
}

/// Latency sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySample {
    pub timestamp: u64,
    pub latency_ms: f64,
    pub operation_type: String,
}

/// Record of a state change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub from_state: CircuitState,
    pub to_state: CircuitState,
    pub timestamp: u64,
    pub reason: String,
    pub anomaly_type: Option<AnomalyType>,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with default configuration
    pub fn new(id: String, anomaly_types: Vec<AnomalyType>) -> Self {
        Self::with_config(id, anomaly_types, CircuitBreakerConfig::default())
    }

    /// Creates a new circuit breaker with custom configuration
    pub fn with_config(id: String, anomaly_types: Vec<AnomalyType>, config: CircuitBreakerConfig) -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            state: CircuitState::Closed,
            anomaly_types,
            config,
            metrics: CircuitMetrics::new(),
            state_history: VecDeque::new(),
            last_state_change: current_time,
            trip_count: 0,
        }
    }

    /// Records a successful operation
    pub fn record_success(&mut self) -> Result<(), CircuitBreakerError> {
        if self.state == CircuitState::Open {
            return Err(CircuitBreakerError::OperationNotAllowed {
                state: "Open".to_string(),
            });
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.record_success(current_time);
        self.cleanup_old_metrics(current_time);

        // Check if we should attempt recovery
        if self.state == CircuitState::HalfOpen && self.config.auto_recovery {
            self.attempt_recovery()?;
        }

        Ok(())
    }

    /// Records a failed operation
    pub fn record_failure(&mut self) -> Result<(), CircuitBreakerError> {
        if self.state == CircuitState::Open {
            return Err(CircuitBreakerError::OperationNotAllowed {
                state: "Open".to_string(),
            });
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.record_failure(current_time);
        self.cleanup_old_metrics(current_time);

        // Check if we should trip the circuit
        if self.should_trip() {
            self.trip("High failure rate detected".to_string(), Some(AnomalyType::HighFailureRate))?;
        }

        Ok(())
    }

    /// Records a resource usage sample
    pub fn record_resource_usage(&mut self, memory_mb: f64, cpu_percent: f64) -> Result<(), CircuitBreakerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.record_resource_usage(current_time, memory_mb, cpu_percent);
        self.cleanup_old_metrics(current_time);

        // Check for resource anomalies
        if self.anomaly_types.contains(&AnomalyType::MemorySpike) {
            if self.detect_memory_anomaly() {
                self.trip("Memory usage spike detected".to_string(), Some(AnomalyType::MemorySpike))?;
            }
        }

        if self.anomaly_types.contains(&AnomalyType::CpuSpike) {
            if self.detect_cpu_anomaly() {
                self.trip("CPU usage spike detected".to_string(), Some(AnomalyType::CpuSpike))?;
            }
        }

        Ok(())
    }

    /// Records a network activity sample
    pub fn record_network_activity(&mut self, connections: u32, bytes_in: u64, bytes_out: u64, packets_dropped: u32) -> Result<(), CircuitBreakerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.record_network_activity(current_time, connections, bytes_in, bytes_out, packets_dropped);
        self.cleanup_old_metrics(current_time);

        // Check for network anomalies
        if self.anomaly_types.contains(&AnomalyType::NetworkAnomaly) {
            if self.detect_network_anomaly() {
                self.trip("Network anomaly detected".to_string(), Some(AnomalyType::NetworkAnomaly))?;
            }
        }

        Ok(())
    }

    /// Records a transaction volume sample
    pub fn record_transaction_volume(&mut self, count: u64, total_value: f64) -> Result<(), CircuitBreakerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.record_transaction_volume(current_time, count, total_value);
        self.cleanup_old_metrics(current_time);

        // Check for volume anomalies
        if self.anomaly_types.contains(&AnomalyType::VolumeAnomaly) {
            if self.detect_volume_anomaly() {
                self.trip("Transaction volume anomaly detected".to_string(), Some(AnomalyType::VolumeAnomaly))?;
            }
        }

        Ok(())
    }

    /// Records a latency sample
    pub fn record_latency(&mut self, latency_ms: f64, operation_type: String) -> Result<(), CircuitBreakerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.record_latency(current_time, latency_ms, operation_type);
        self.cleanup_old_metrics(current_time);

        // Check for latency anomalies
        if self.anomaly_types.contains(&AnomalyType::LatencySpike) {
            if self.detect_latency_anomaly() {
                self.trip("Latency spike detected".to_string(), Some(AnomalyType::LatencySpike))?;
            }
        }

        Ok(())
    }

    /// Trips the circuit breaker
    pub fn trip(&mut self, reason: String, anomaly_type: Option<AnomalyType>) -> Result<(), CircuitBreakerError> {
        if self.state == CircuitState::Open {
            return Err(CircuitBreakerError::AlreadyTripped { reason });
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let old_state = self.state.clone();
        self.state = CircuitState::Open;
        self.last_state_change = current_time;
        self.trip_count += 1;

        // Record state change
        let state_change = StateChange {
            from_state: old_state,
            to_state: CircuitState::Open,
            timestamp: current_time,
            reason,
            anomaly_type,
        };

        self.state_history.push_back(state_change);

        // Keep only recent history
        if self.state_history.len() > 100 {
            self.state_history.pop_front();
        }

        Ok(())
    }

    /// Attempts to reset the circuit breaker
    pub fn reset(&mut self) -> Result<(), CircuitBreakerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let old_state = self.state.clone();
        self.state = CircuitState::Closed;
        self.last_state_change = current_time;

        // Clear metrics
        self.metrics = CircuitMetrics::new();

        // Record state change
        let state_change = StateChange {
            from_state: old_state,
            to_state: CircuitState::Closed,
            timestamp: current_time,
            reason: "Manual reset".to_string(),
            anomaly_type: None,
        };

        self.state_history.push_back(state_change);

        Ok(())
    }

    /// Checks if the circuit should trip based on current metrics
    fn should_trip(&self) -> bool {
        if self.metrics.total_operations < self.config.min_samples as u64 {
            return false;
        }

        let failure_rate = (self.metrics.failed_operations as f64 / self.metrics.total_operations as f64) * 100.0;
        failure_rate > self.config.failure_rate_threshold
    }

    /// Detects memory usage anomalies
    fn detect_memory_anomaly(&self) -> bool {
        if self.metrics.resource_samples.len() < self.config.min_samples {
            return false;
        }

        let memory_values: Vec<f64> = self.metrics.resource_samples
            .iter()
            .map(|s| s.memory_usage_mb)
            .collect();

        detect_statistical_anomaly(&memory_values, self.config.anomaly_threshold)
    }

    /// Detects CPU usage anomalies
    fn detect_cpu_anomaly(&self) -> bool {
        if self.metrics.resource_samples.len() < self.config.min_samples {
            return false;
        }

        let cpu_values: Vec<f64> = self.metrics.resource_samples
            .iter()
            .map(|s| s.cpu_usage_percent)
            .collect();

        detect_statistical_anomaly(&cpu_values, self.config.anomaly_threshold)
    }

    /// Detects network activity anomalies
    fn detect_network_anomaly(&self) -> bool {
        if self.metrics.network_samples.len() < self.config.min_samples {
            return false;
        }

        // Check for unusual packet drops
        let drop_rates: Vec<f64> = self.metrics.network_samples
            .iter()
            .map(|s| {
                if s.bytes_in + s.bytes_out > 0 {
                    (s.packets_dropped as f64 / (s.bytes_in + s.bytes_out) as f64) * 100.0
                } else {
                    0.0
                }
            })
            .collect();

        detect_statistical_anomaly(&drop_rates, self.config.anomaly_threshold)
    }

    /// Detects transaction volume anomalies
    fn detect_volume_anomaly(&self) -> bool {
        if self.metrics.volume_samples.len() < self.config.min_samples {
            return false;
        }

        let volumes: Vec<f64> = self.metrics.volume_samples
            .iter()
            .map(|s| s.transaction_count as f64)
            .collect();

        detect_statistical_anomaly(&volumes, self.config.anomaly_threshold)
    }

    /// Detects latency anomalies
    fn detect_latency_anomaly(&self) -> bool {
        if self.metrics.latency_samples.len() < self.config.min_samples {
            return false;
        }

        let latencies: Vec<f64> = self.metrics.latency_samples
            .iter()
            .map(|s| s.latency_ms)
            .collect();

        detect_statistical_anomaly(&latencies, self.config.anomaly_threshold)
    }

    /// Attempts automatic recovery
    fn attempt_recovery(&mut self) -> Result<(), CircuitBreakerError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if current_time - self.last_state_change >= self.config.cooldown_period {
            let old_state = self.state.clone();
            self.state = CircuitState::HalfOpen;
            self.last_state_change = current_time;

            // Record state change
            let state_change = StateChange {
                from_state: old_state,
                to_state: CircuitState::HalfOpen,
                timestamp: current_time,
                reason: "Attempting automatic recovery".to_string(),
                anomaly_type: None,
            };

            self.state_history.push_back(state_change);
        }

        Ok(())
    }

    /// Cleans up old metrics outside the window
    fn cleanup_old_metrics(&mut self, current_time: u64) {
        let cutoff_time = current_time - self.config.metrics_window;

        // Clean up recent operations
        while let Some(&timestamp) = self.metrics.recent_operations.front() {
            if timestamp < cutoff_time {
                self.metrics.recent_operations.pop_front();
            } else {
                break;
            }
        }

        // Clean up recent failures
        while let Some(&timestamp) = self.metrics.recent_failures.front() {
            if timestamp < cutoff_time {
                self.metrics.recent_failures.pop_front();
            } else {
                break;
            }
        }

        // Clean up resource samples
        while let Some(sample) = self.metrics.resource_samples.front() {
            if sample.timestamp < cutoff_time {
                self.metrics.resource_samples.pop_front();
            } else {
                break;
            }
        }

        // Clean up network samples
        while let Some(sample) = self.metrics.network_samples.front() {
            if sample.timestamp < cutoff_time {
                self.metrics.network_samples.pop_front();
            } else {
                break;
            }
        }

        // Clean up volume samples
        while let Some(sample) = self.metrics.volume_samples.front() {
            if sample.timestamp < cutoff_time {
                self.metrics.volume_samples.pop_front();
            } else {
                break;
            }
        }

        // Clean up latency samples
        while let Some(sample) = self.metrics.latency_samples.front() {
            if sample.timestamp < cutoff_time {
                self.metrics.latency_samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Gets the current failure rate
    pub fn get_failure_rate(&self) -> f64 {
        if self.metrics.total_operations == 0 {
            0.0
        } else {
            (self.metrics.failed_operations as f64 / self.metrics.total_operations as f64) * 100.0
        }
    }

    /// Checks if the circuit breaker allows operations
    pub fn allows_operations(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true, // Limited operations allowed for testing
        }
    }

    /// Gets time until next recovery attempt
    pub fn get_recovery_time(&self) -> Option<u64> {
        if self.state != CircuitState::Open {
            return None;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let next_attempt = self.last_state_change + self.config.cooldown_period;
        
        if current_time >= next_attempt {
            Some(0)
        } else {
            Some(next_attempt - current_time)
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_rate_threshold: DEFAULT_FAILURE_RATE_THRESHOLD,
            anomaly_threshold: DEFAULT_ANOMALY_THRESHOLD,
            metrics_window: DEFAULT_METRICS_WINDOW,
            cooldown_period: DEFAULT_COOLDOWN_PERIOD,
            max_pause_duration: DEFAULT_MAX_PAUSE_DURATION,
            auto_recovery: true,
            min_samples: MIN_SAMPLES_FOR_ANOMALY_DETECTION,
        }
    }
}

impl CircuitMetrics {
    /// Creates new metrics
    pub fn new() -> Self {
        Self {
            total_operations: 0,
            failed_operations: 0,
            recent_operations: VecDeque::new(),
            recent_failures: VecDeque::new(),
            resource_samples: VecDeque::new(),
            network_samples: VecDeque::new(),
            volume_samples: VecDeque::new(),
            latency_samples: VecDeque::new(),
        }
    }

    /// Records a successful operation
    pub fn record_success(&mut self, timestamp: u64) {
        self.total_operations += 1;
        self.recent_operations.push_back(timestamp);
    }

    /// Records a failed operation
    pub fn record_failure(&mut self, timestamp: u64) {
        self.total_operations += 1;
        self.failed_operations += 1;
        self.recent_operations.push_back(timestamp);
        self.recent_failures.push_back(timestamp);
    }

    /// Records resource usage
    pub fn record_resource_usage(&mut self, timestamp: u64, memory_mb: f64, cpu_percent: f64) {
        self.resource_samples.push_back(ResourceSample {
            timestamp,
            memory_usage_mb: memory_mb,
            cpu_usage_percent: cpu_percent,
        });
    }

    /// Records network activity
    pub fn record_network_activity(&mut self, timestamp: u64, connections: u32, bytes_in: u64, bytes_out: u64, packets_dropped: u32) {
        self.network_samples.push_back(NetworkSample {
            timestamp,
            connections_count: connections,
            bytes_in,
            bytes_out,
            packets_dropped,
        });
    }

    /// Records transaction volume
    pub fn record_transaction_volume(&mut self, timestamp: u64, count: u64, total_value: f64) {
        self.volume_samples.push_back(VolumeSample {
            timestamp,
            transaction_count: count,
            total_value,
        });
    }

    /// Records latency
    pub fn record_latency(&mut self, timestamp: u64, latency_ms: f64, operation_type: String) {
        self.latency_samples.push_back(LatencySample {
            timestamp,
            latency_ms,
            operation_type,
        });
    }
}

/// Detects statistical anomalies using standard deviation
fn detect_statistical_anomaly(values: &[f64], threshold: f64) -> bool {
    if values.len() < 2 {
        return false;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / (values.len() - 1) as f64;
    let std_dev = variance.sqrt();

    // Check if the latest value is an outlier
    if let Some(&latest) = values.last() {
        let z_score = (latest - mean) / std_dev;
        z_score.abs() > threshold
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_creation() {
        let breaker = CircuitBreaker::new(
            "test".to_string(),
            vec![AnomalyType::HighFailureRate],
        );

        assert_eq!(breaker.id, "test");
        assert_eq!(breaker.state, CircuitState::Closed);
        assert_eq!(breaker.trip_count, 0);
    }

    #[test]
    fn test_failure_rate_detection() {
        let mut breaker = CircuitBreaker::new(
            "test".to_string(),
            vec![AnomalyType::HighFailureRate],
        );

        // Record some failures
        for _ in 0..15 {
            breaker.record_failure().unwrap();
        }

        // Should trip due to high failure rate
        assert_eq!(breaker.state, CircuitState::Open);
        assert_eq!(breaker.trip_count, 1);
    }

    #[test]
    fn test_circuit_prevents_operations_when_open() {
        let mut breaker = CircuitBreaker::new(
            "test".to_string(),
            vec![AnomalyType::HighFailureRate],
        );

        // Trip the circuit
        breaker.trip("Test trip".to_string(), None).unwrap();

        // Operations should be blocked
        assert!(breaker.record_success().is_err());
        assert!(breaker.record_failure().is_err());
        assert!(!breaker.allows_operations());
    }

    #[test]
    fn test_statistical_anomaly_detection() {
        let normal_values = vec![10.0, 11.0, 9.0, 10.5, 9.5, 10.2, 9.8, 10.1, 9.9, 10.3];
        let anomaly_values = vec![10.0, 11.0, 9.0, 10.5, 9.5, 10.2, 9.8, 10.1, 9.9, 50.0]; // Last value is outlier

        assert!(!detect_statistical_anomaly(&normal_values, 3.0));
        assert!(detect_statistical_anomaly(&anomaly_values, 3.0));
    }

    #[test]
    fn test_resource_monitoring() {
        let mut breaker = CircuitBreaker::new(
            "test".to_string(),
            vec![AnomalyType::MemorySpike, AnomalyType::CpuSpike],
        );

        // Record normal resource usage
        for _ in 0..10 {
            breaker.record_resource_usage(100.0, 20.0).unwrap();
        }

        // Record a spike
        breaker.record_resource_usage(1000.0, 20.0).unwrap();

        // Should detect memory anomaly
        assert_eq!(breaker.state, CircuitState::Open);
    }

    #[test]
    fn test_circuit_reset() {
        let mut breaker = CircuitBreaker::new(
            "test".to_string(),
            vec![AnomalyType::HighFailureRate],
        );

        // Trip the circuit
        breaker.trip("Test trip".to_string(), None).unwrap();
        assert_eq!(breaker.state, CircuitState::Open);

        // Reset the circuit
        breaker.reset().unwrap();
        assert_eq!(breaker.state, CircuitState::Closed);
        assert_eq!(breaker.metrics.total_operations, 0);
    }

    #[test]
    fn test_metrics_cleanup() {
        let mut breaker = CircuitBreaker::new(
            "test".to_string(),
            vec![AnomalyType::HighFailureRate],
        );

        // Record some operations
        breaker.record_success().unwrap();
        breaker.record_failure().unwrap();

        // Should have metrics
        assert_eq!(breaker.metrics.total_operations, 2);

        // Reset should clear metrics
        breaker.reset().unwrap();
        assert_eq!(breaker.metrics.total_operations, 0);
    }
}