//! Circuit breaker manager for coordinating multiple circuit breakers.
//!
//! This module provides a centralized manager for coordinating multiple
//! circuit breakers across different system components and implementing
//! rate limiting for the overall circuit breaker system.

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerError, CircuitState, AnomalyType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Default global rate limit for circuit breaker operations
pub const DEFAULT_GLOBAL_RATE_LIMIT: u32 = 1000;

/// Default rate limit window (seconds)
pub const DEFAULT_RATE_LIMIT_WINDOW: u64 = 60;

/// Maximum number of circuit breakers allowed
pub const MAX_CIRCUIT_BREAKERS: usize = 100;

/// Errors that can occur during circuit breaker management
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CircuitManagerError {
    /// Circuit breaker not found
    #[error("circuit breaker not found: {id}")]
    NotFound { id: String },

    /// Too many circuit breakers
    #[error("too many circuit breakers (max: {max})")]
    TooManyBreakers { max: usize },

    /// Rate limit exceeded
    #[error("rate limit exceeded: {current}/{limit} in {window}s")]
    RateLimitExceeded { current: u32, limit: u32, window: u64 },

    /// Invalid configuration
    #[error("invalid manager configuration: {reason}")]
    InvalidConfiguration { reason: String },

    /// Circuit breaker operation failed
    #[error("circuit breaker operation failed: {id}, reason: {reason}")]
    OperationFailed { id: String, reason: String },
}

/// Global circuit breaker manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerManager {
    /// All managed circuit breakers
    pub circuit_breakers: HashMap<String, CircuitBreaker>,
    /// Global rate limiter
    pub rate_limiter: RateLimiter,
    /// Manager configuration
    pub config: CircuitManagerConfig,
    /// Global metrics
    pub global_metrics: GlobalMetrics,
    /// Last cleanup timestamp
    pub last_cleanup: u64,
}

/// Configuration for the circuit breaker manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitManagerConfig {
    /// Global rate limit for operations
    pub global_rate_limit: u32,
    /// Rate limit window (seconds)
    pub rate_limit_window: u64,
    /// Cleanup interval (seconds)
    pub cleanup_interval: u64,
    /// Whether automatic cleanup is enabled
    pub auto_cleanup: bool,
    /// Maximum number of circuit breakers
    pub max_circuit_breakers: usize,
}

/// Global rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    /// Operation timestamps
    pub operation_timestamps: Vec<u64>,
    /// Rate limit
    pub limit: u32,
    /// Window size
    pub window: u64,
}

/// Global metrics across all circuit breakers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    /// Total operations across all breakers
    pub total_operations: u64,
    /// Total failures across all breakers
    pub total_failures: u64,
    /// Total trips across all breakers
    pub total_trips: u64,
    /// Currently open breakers
    pub open_breakers: u64,
    /// Currently half-open breakers
    pub half_open_breakers: u64,
    /// Currently closed breakers
    pub closed_breakers: u64,
    /// Last updated timestamp
    pub last_updated: u64,
}

impl CircuitBreakerManager {
    /// Creates a new circuit breaker manager with default configuration
    pub fn new() -> Self {
        Self::with_config(CircuitManagerConfig::default())
    }

    /// Creates a new circuit breaker manager with custom configuration
    pub fn with_config(config: CircuitManagerConfig) -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            circuit_breakers: HashMap::new(),
            rate_limiter: RateLimiter::new(config.global_rate_limit, config.rate_limit_window),
            config,
            global_metrics: GlobalMetrics::new(),
            last_cleanup: current_time,
        }
    }

    /// Adds a new circuit breaker
    pub fn add_circuit_breaker(&mut self, breaker: CircuitBreaker) -> Result<(), CircuitManagerError> {
        if self.circuit_breakers.len() >= self.config.max_circuit_breakers {
            return Err(CircuitManagerError::TooManyBreakers {
                max: self.config.max_circuit_breakers,
            });
        }

        if self.circuit_breakers.contains_key(&breaker.id) {
            return Err(CircuitManagerError::OperationFailed {
                id: breaker.id,
                reason: "circuit breaker already exists".to_string(),
            });
        }

        self.circuit_breakers.insert(breaker.id.clone(), breaker);
        self.update_global_metrics();

        Ok(())
    }

    /// Removes a circuit breaker
    pub fn remove_circuit_breaker(&mut self, id: &str) -> Result<(), CircuitManagerError> {
        if !self.circuit_breakers.contains_key(id) {
            return Err(CircuitManagerError::NotFound { id: id.to_string() });
        }

        self.circuit_breakers.remove(id);
        self.update_global_metrics();

        Ok(())
    }

    /// Gets a circuit breaker by ID
    pub fn get_circuit_breaker(&self, id: &str) -> Option<&CircuitBreaker> {
        self.circuit_breakers.get(id)
    }

    /// Gets a mutable reference to a circuit breaker by ID
    pub fn get_circuit_breaker_mut(&mut self, id: &str) -> Option<&mut CircuitBreaker> {
        self.circuit_breakers.get_mut(id)
    }

    /// Records a successful operation for a specific circuit breaker
    pub fn record_success(&mut self, breaker_id: &str) -> Result<(), CircuitManagerError> {
        // Check global rate limit
        self.check_rate_limit()?;

        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        breaker.record_success()
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        self.global_metrics.total_operations += 1;
        self.global_metrics.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(())
    }

    /// Records a failed operation for a specific circuit breaker
    pub fn record_failure(&mut self, breaker_id: &str) -> Result<(), CircuitManagerError> {
        // Check global rate limit
        self.check_rate_limit()?;

        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        let old_state = breaker.state.clone();
        breaker.record_failure()
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        // Update global metrics
        self.global_metrics.total_operations += 1;
        self.global_metrics.total_failures += 1;

        // Check if circuit just tripped
        if old_state != CircuitState::Open && breaker.state == CircuitState::Open {
            self.global_metrics.total_trips += 1;
        }

        self.global_metrics.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(())
    }

    /// Records resource usage for a specific circuit breaker
    pub fn record_resource_usage(&mut self, breaker_id: &str, memory_mb: f64, cpu_percent: f64) -> Result<(), CircuitManagerError> {
        // Check global rate limit
        self.check_rate_limit()?;

        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        breaker.record_resource_usage(memory_mb, cpu_percent)
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Records network activity for a specific circuit breaker
    pub fn record_network_activity(&mut self, breaker_id: &str, connections: u32, bytes_in: u64, bytes_out: u64, packets_dropped: u32) -> Result<(), CircuitManagerError> {
        // Check global rate limit
        self.check_rate_limit()?;

        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        breaker.record_network_activity(connections, bytes_in, bytes_out, packets_dropped)
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Records transaction volume for a specific circuit breaker
    pub fn record_transaction_volume(&mut self, breaker_id: &str, count: u64, total_value: f64) -> Result<(), CircuitManagerError> {
        // Check global rate limit
        self.check_rate_limit()?;

        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        breaker.record_transaction_volume(count, total_value)
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Records latency for a specific circuit breaker
    pub fn record_latency(&mut self, breaker_id: &str, latency_ms: f64, operation_type: String) -> Result<(), CircuitManagerError> {
        // Check global rate limit
        self.check_rate_limit()?;

        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        breaker.record_latency(latency_ms, operation_type)
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Trips a specific circuit breaker
    pub fn trip_circuit_breaker(&mut self, breaker_id: &str, reason: String, anomaly_type: Option<AnomalyType>) -> Result<(), CircuitManagerError> {
        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        let old_state = breaker.state.clone();
        breaker.trip(reason, anomaly_type)
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        // Update global metrics if circuit just tripped
        if old_state != CircuitState::Open && breaker.state == CircuitState::Open {
            self.global_metrics.total_trips += 1;
        }

        Ok(())
    }

    /// Resets a specific circuit breaker
    pub fn reset_circuit_breaker(&mut self, breaker_id: &str) -> Result<(), CircuitManagerError> {
        let breaker = self.circuit_breakers.get_mut(breaker_id)
            .ok_or_else(|| CircuitManagerError::NotFound { id: breaker_id.to_string() })?;

        breaker.reset()
            .map_err(|e| CircuitManagerError::OperationFailed {
                id: breaker_id.to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Gets all circuit breakers in a specific state
    pub fn get_circuit_breakers_by_state(&self, state: CircuitState) -> Vec<&CircuitBreaker> {
        self.circuit_breakers
            .values()
            .filter(|breaker| breaker.state == state)
            .collect()
    }

    /// Gets circuit breakers monitoring specific anomaly types
    pub fn get_circuit_breakers_by_anomaly_type(&self, anomaly_type: &AnomalyType) -> Vec<&CircuitBreaker> {
        self.circuit_breakers
            .values()
            .filter(|breaker| breaker.anomaly_types.contains(anomaly_type))
            .collect()
    }

    /// Checks if any circuit breakers are currently open
    pub fn has_open_circuits(&self) -> bool {
        self.circuit_breakers
            .values()
            .any(|breaker| breaker.state == CircuitState::Open)
    }

    /// Gets the percentage of open circuit breakers
    pub fn get_open_circuit_percentage(&self) -> f64 {
        if self.circuit_breakers.is_empty() {
            0.0
        } else {
            let open_count = self.circuit_breakers
                .values()
                .filter(|breaker| breaker.state == CircuitState::Open)
                .count();
            (open_count as f64 / self.circuit_breakers.len() as f64) * 100.0
        }
    }

    /// Performs automatic cleanup of old data
    pub fn cleanup(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check if cleanup is needed
        if current_time - self.last_cleanup < self.config.cleanup_interval {
            return;
        }

        // Cleanup rate limiter
        self.rate_limiter.cleanup(current_time);

        // Update global metrics
        self.update_global_metrics();

        self.last_cleanup = current_time;
    }

    /// Updates global metrics based on current circuit breaker states
    fn update_global_metrics(&mut self) {
        let mut open_count = 0;
        let mut half_open_count = 0;
        let mut closed_count = 0;

        for breaker in self.circuit_breakers.values() {
            match breaker.state {
                CircuitState::Open => open_count += 1,
                CircuitState::HalfOpen => half_open_count += 1,
                CircuitState::Closed => closed_count += 1,
            }
        }

        self.global_metrics.open_breakers = open_count;
        self.global_metrics.half_open_breakers = half_open_count;
        self.global_metrics.closed_breakers = closed_count;
        self.global_metrics.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Checks the global rate limit
    fn check_rate_limit(&mut self) -> Result<(), CircuitManagerError> {
        if self.rate_limiter.check_limit() {
            Ok(())
        } else {
            Err(CircuitManagerError::RateLimitExceeded {
                current: self.rate_limiter.current_count(),
                limit: self.rate_limiter.limit,
                window: self.rate_limiter.window,
            })
        }
    }

    /// Gets a summary of all circuit breaker states
    pub fn get_status_summary(&self) -> CircuitStatusSummary {
        CircuitStatusSummary {
            total_breakers: self.circuit_breakers.len(),
            open_breakers: self.global_metrics.open_breakers as usize,
            half_open_breakers: self.global_metrics.half_open_breakers as usize,
            closed_breakers: self.global_metrics.closed_breakers as usize,
            total_operations: self.global_metrics.total_operations,
            total_failures: self.global_metrics.total_failures,
            total_trips: self.global_metrics.total_trips,
            open_percentage: self.get_open_circuit_percentage(),
        }
    }
}

/// Summary of circuit breaker status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStatusSummary {
    pub total_breakers: usize,
    pub open_breakers: usize,
    pub half_open_breakers: usize,
    pub closed_breakers: usize,
    pub total_operations: u64,
    pub total_failures: u64,
    pub total_trips: u64,
    pub open_percentage: f64,
}

impl RateLimiter {
    /// Creates a new rate limiter
    pub fn new(limit: u32, window: u64) -> Self {
        Self {
            operation_timestamps: Vec::new(),
            limit,
            window,
        }
    }

    /// Checks if the operation is within the rate limit
    pub fn check_limit(&mut self) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove old timestamps
        self.cleanup(current_time);

        // Check if we're at the limit
        if self.operation_timestamps.len() >= self.limit as usize {
            false
        } else {
            self.operation_timestamps.push(current_time);
            true
        }
    }

    /// Gets the current count of operations in the window
    pub fn current_count(&self) -> u32 {
        self.operation_timestamps.len() as u32
    }

    /// Cleans up old timestamps
    pub fn cleanup(&mut self, current_time: u64) {
        let cutoff_time = current_time - self.window;
        self.operation_timestamps.retain(|&timestamp| timestamp >= cutoff_time);
    }
}

impl Default for CircuitManagerConfig {
    fn default() -> Self {
        Self {
            global_rate_limit: DEFAULT_GLOBAL_RATE_LIMIT,
            rate_limit_window: DEFAULT_RATE_LIMIT_WINDOW,
            cleanup_interval: 300, // 5 minutes
            auto_cleanup: true,
            max_circuit_breakers: MAX_CIRCUIT_BREAKERS,
        }
    }
}

impl GlobalMetrics {
    /// Creates new global metrics
    pub fn new() -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            total_operations: 0,
            total_failures: 0,
            total_trips: 0,
            open_breakers: 0,
            half_open_breakers: 0,
            closed_breakers: 0,
            last_updated: current_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_breaker::{CircuitBreaker, AnomalyType};

    #[test]
    fn test_manager_creation() {
        let manager = CircuitBreakerManager::new();
        assert_eq!(manager.circuit_breakers.len(), 0);
        assert_eq!(manager.global_metrics.total_operations, 0);
    }

    #[test]
    fn test_add_circuit_breaker() {
        let mut manager = CircuitBreakerManager::new();
        let breaker = CircuitBreaker::new("test".to_string(), vec![AnomalyType::HighFailureRate]);

        assert!(manager.add_circuit_breaker(breaker).is_ok());
        assert_eq!(manager.circuit_breakers.len(), 1);
        assert!(manager.get_circuit_breaker("test").is_some());
    }

    #[test]
    fn test_rate_limiting() {
        let mut config = CircuitManagerConfig::default();
        config.global_rate_limit = 5;
        config.rate_limit_window = 1;

        let mut manager = CircuitBreakerManager::with_config(config);
        let breaker = CircuitBreaker::new("test".to_string(), vec![AnomalyType::HighFailureRate]);
        manager.add_circuit_breaker(breaker).unwrap();

        // Should allow first 5 operations
        for _ in 0..5 {
            assert!(manager.record_success("test").is_ok());
        }

        // 6th operation should be rate limited
        assert!(manager.record_success("test").is_err());
    }

    #[test]
    fn test_global_metrics() {
        let mut manager = CircuitBreakerManager::new();
        let breaker = CircuitBreaker::new("test".to_string(), vec![AnomalyType::HighFailureRate]);
        manager.add_circuit_breaker(breaker).unwrap();

        // Record some operations
        manager.record_success("test").unwrap();
        manager.record_failure("test").unwrap();

        let summary = manager.get_status_summary();
        assert_eq!(summary.total_operations, 2);
        assert_eq!(summary.total_failures, 1);
        assert_eq!(summary.total_breakers, 1);
    }

    #[test]
    fn test_circuit_breaker_filtering() {
        let mut manager = CircuitBreakerManager::new();
        
        let breaker1 = CircuitBreaker::new("test1".to_string(), vec![AnomalyType::HighFailureRate]);
        let breaker2 = CircuitBreaker::new("test2".to_string(), vec![AnomalyType::MemorySpike]);
        let breaker3 = CircuitBreaker::new("test3".to_string(), vec![AnomalyType::HighFailureRate]);

        manager.add_circuit_breaker(breaker1).unwrap();
        manager.add_circuit_breaker(breaker2).unwrap();
        manager.add_circuit_breaker(breaker3).unwrap();

        // Filter by anomaly type
        let failure_rate_breakers = manager.get_circuit_breakers_by_anomaly_type(&AnomalyType::HighFailureRate);
        assert_eq!(failure_rate_breakers.len(), 2);

        let memory_breakers = manager.get_circuit_breakers_by_anomaly_type(&AnomalyType::MemorySpike);
        assert_eq!(memory_breakers.len(), 1);
    }

    #[test]
    fn test_cleanup() {
        let mut manager = CircuitBreakerManager::new();
        let breaker = CircuitBreaker::new("test".to_string(), vec![AnomalyType::HighFailureRate]);
        manager.add_circuit_breaker(breaker).unwrap();

        // Perform some operations
        manager.record_success("test").unwrap();

        // Cleanup should not remove valid data
        manager.cleanup();
        assert_eq!(manager.global_metrics.total_operations, 1);
    }
}