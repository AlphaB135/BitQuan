//! Comprehensive monitoring and health check system for BitQuan
//! 
//! This module provides:
//! - Prometheus metrics endpoint
//! - Health check endpoints
//! - System status monitoring
//! - Performance metrics collection

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use warp::{Filter, Reply};

use crate::metrics::MiningMetrics;
use bitquan_rpc::metrics::Metrics as RpcMetrics;

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub version: String,
    pub components: HashMap<String, ComponentHealth>,
}

/// Individual component health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: String,
    pub last_check: u64,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub details: Option<HashMap<String, String>>,
}

/// System performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub disk_usage_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub open_file_descriptors: u64,
    pub thread_count: u64,
}

/// Comprehensive monitoring system
pub struct MonitoringSystem {
    start_time: Instant,
    mining_metrics: Arc<MiningMetrics>,
    rpc_metrics: Arc<RpcMetrics>,
    health_checks: Arc<RwLock<HashMap<String, ComponentHealth>>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Total HTTP requests handled
    http_requests_total: Arc<AtomicU64>,
    /// Total WebSocket connections
    websocket_connections: Arc<AtomicU64>,
    /// Active WebSocket connections
    websocket_connections_active: Arc<AtomicU64>,
    /// System errors count
    system_errors_total: Arc<AtomicU64>,
    /// Last health check timestamp
    last_health_check: Arc<RwLock<Instant>>,
}

impl MonitoringSystem {
    /// Create new monitoring system
    pub fn new(mining_metrics: Arc<MiningMetrics>, rpc_metrics: Arc<RpcMetrics>) -> Self {
        Self {
            start_time: Instant::now(),
            mining_metrics,
            rpc_metrics,
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics {
                cpu_usage_percent: 0.0,
                memory_usage_mb: 0,
                disk_usage_percent: 0.0,
                network_rx_bytes: 0,
                network_tx_bytes: 0,
                open_file_descriptors: 0,
                thread_count: 0,
            })),
            http_requests_total: Arc::new(AtomicU64::new(0)),
            websocket_connections: Arc::new(AtomicU64::new(0)),
            websocket_connections_active: Arc::new(AtomicU64::new(0)),
            system_errors_total: Arc::new(AtomicU64::new(0)),
            last_health_check: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Record HTTP request
    pub fn record_http_request(&self) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record WebSocket connection
    pub fn record_websocket_connection(&self) {
        self.websocket_connections.fetch_add(1, Ordering::Relaxed);
        self.websocket_connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record WebSocket disconnection
    pub fn record_websocket_disconnection(&self) {
        self.websocket_connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record system error
    pub fn record_system_error(&self) {
        self.system_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Update component health status
    pub async fn update_component_health(
        &self,
        component: String,
        status: String,
        response_time_ms: Option<u64>,
        error_message: Option<String>,
        details: Option<HashMap<String, String>>,
    ) {
        let mut health_checks = self.health_checks.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        health_checks.insert(
            component,
            ComponentHealth {
                status,
                last_check: now,
                response_time_ms,
                error_message,
                details,
            },
        );

        // Update last health check timestamp
        let mut last_check = self.last_health_check.write().await;
        *last_check = Instant::now();
    }

    /// Update performance metrics
    pub async fn update_performance_metrics(&self, metrics: PerformanceMetrics) {
        let mut current_metrics = self.performance_metrics.write().await;
        *current_metrics = metrics;
    }

    /// Get comprehensive health status
    pub async fn get_health_status(&self) -> HealthStatus {
        let health_checks = self.health_checks.read().await;
        let uptime = self.start_time.elapsed().as_secs();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Determine overall system status
        let mut overall_status = "healthy".to_string();
        for component in health_checks.values() {
            match component.status.as_str() {
                "unhealthy" => {
                    overall_status = "unhealthy".to_string();
                    break;
                }
                "degraded" => {
                    overall_status = "degraded".to_string();
                }
                _ => {}
            }
        }

        HealthStatus {
            status: overall_status,
            timestamp: now,
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            components: health_checks.clone(),
        }
    }

    /// Get all metrics in Prometheus format
    pub fn get_prometheus_metrics(&self) -> String {
        let mut output = String::new();

        // Mining metrics
        output.push_str(&self.mining_metrics.format_prometheus());
        output.push_str("\n");

        // RPC metrics
        output.push_str(&self.rpc_metrics.export_prometheus("mainnet"));
        output.push_str("\n");

        // HTTP metrics
        output.push_str("# HELP http_requests_total Total HTTP requests handled\n");
        output.push_str("# TYPE http_requests_total counter\n");
        output.push_str(&format!(
            "http_requests_total {}\n",
            self.http_requests_total.load(Ordering::Relaxed)
        ));

        // WebSocket metrics
        output.push_str("# HELP websocket_connections_total Total WebSocket connections\n");
        output.push_str("# TYPE websocket_connections_total counter\n");
        output.push_str(&format!(
            "websocket_connections_total {}\n",
            self.websocket_connections.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP websocket_connections_active Active WebSocket connections\n");
        output.push_str("# TYPE websocket_connections_active gauge\n");
        output.push_str(&format!(
            "websocket_connections_active {}\n",
            self.websocket_connections_active.load(Ordering::Relaxed)
        ));

        // System error metrics
        output.push_str("# HELP system_errors_total Total system errors\n");
        output.push_str("# TYPE system_errors_total counter\n");
        output.push_str(&format!(
            "system_errors_total {}\n",
            self.system_errors_total.load(Ordering::Relaxed)
        ));

        // Uptime metric
        output.push_str("# HELP system_uptime_seconds System uptime in seconds\n");
        output.push_str("# TYPE system_uptime_seconds gauge\n");
        output.push_str(&format!(
            "system_uptime_seconds {}\n",
            self.start_time.elapsed().as_secs()
        ));

        output
    }

    /// Create Warp filters for monitoring endpoints
    pub fn routes(
        self: Arc<Self>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let monitoring1 = self.clone();
        let monitoring2 = self.clone();
        let monitoring3 = self.clone();
        let monitoring4 = self.clone();

        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .and(warp::any().map(move || monitoring1.clone()))
            .and_then(|monitoring: Arc<MonitoringSystem>| async move {
                monitoring.record_http_request();
                Ok::<_, warp::Rejection>(warp::reply::with_header(
                    warp::reply::html(monitoring.get_prometheus_metrics()),
                    "content-type",
                    "text/plain; version=0.0.4; charset=utf-8",
                ))
            });

        let health_route = warp::path("health")
            .and(warp::get())
            .and(warp::any().map(move || monitoring2.clone()))
            .and_then(|monitoring: Arc<MonitoringSystem>| async move {
                monitoring.record_http_request();
                let health = monitoring.get_health_status().await;
                Ok::<_, warp::Rejection>(warp::reply::json(&health))
            });

        let health_simple_route = warp::path("health")
            .and(warp::path("simple"))
            .and(warp::get())
            .and(warp::any().map(move || monitoring3.clone()))
            .and_then(|monitoring: Arc<MonitoringSystem>| async move {
                monitoring.record_http_request();
                let health = monitoring.get_health_status().await;
                let status = if health.status == "healthy" {
                    warp::http::StatusCode::OK
                } else if health.status == "degraded" {
                    warp::http::StatusCode::OK
                } else {
                    warp::http::StatusCode::SERVICE_UNAVAILABLE
                };
                Ok::<_, warp::Rejection>(warp::reply::with_status(
                    health.status,
                    status,
                ))
            });

        let performance_route = warp::path("performance")
            .and(warp::get())
            .and(warp::any().map(move || monitoring4.clone()))
            .and_then(|monitoring: Arc<MonitoringSystem>| async move {
                monitoring.record_http_request();
                let metrics = monitoring.performance_metrics.read().await;
                Ok::<_, warp::Rejection>(warp::reply::json(&*metrics))
            });

        metrics_route
            .or(health_route)
            .or(health_simple_route)
            .or(performance_route)
    }

    /// Run periodic health checks
    pub async fn run_health_checks(self: Arc<Self>) {
        let monitoring = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check database connectivity
                let db_start = Instant::now();
                // In a real implementation, this would check actual database connectivity
                let db_healthy = true; // Placeholder
                let db_response_time = db_start.elapsed().as_millis() as u64;
                
                monitoring.update_component_health(
                    "database".to_string(),
                    if db_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
                    Some(db_response_time),
                    None,
                    None,
                ).await;

                // Check network connectivity
                let network_start = Instant::now();
                // In a real implementation, this would check network connectivity
                let network_healthy = true; // Placeholder
                let network_response_time = network_start.elapsed().as_millis() as u64;
                
                monitoring.update_component_health(
                    "network".to_string(),
                    if network_healthy { "healthy".to_string() } else { "degraded".to_string() },
                    Some(network_response_time),
                    None,
                    None,
                ).await;

                // Check mining pool status
                let pool_start = Instant::now();
                // In a real implementation, this would check pool status
                let pool_healthy = true; // Placeholder
                let pool_response_time = pool_start.elapsed().as_millis() as u64;
                
                monitoring.update_component_health(
                    "mining_pool".to_string(),
                    if pool_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
                    Some(pool_response_time),
                    None,
                    None,
                ).await;

                // Update performance metrics (placeholder values)
                let performance_metrics = PerformanceMetrics {
                    cpu_usage_percent: 45.2,
                    memory_usage_mb: 1024,
                    disk_usage_percent: 67.8,
                    network_rx_bytes: 1024000,
                    network_tx_bytes: 512000,
                    open_file_descriptors: 256,
                    thread_count: 42,
                };
                
                monitoring.update_performance_metrics(performance_metrics).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitquan_consensus::pow::PowAlgo;

    #[tokio::test]
    async fn test_health_status() {
        let mining_metrics = Arc::new(MiningMetrics::new(&[PowAlgo::Sha256d]));
        let rpc_metrics = Arc::new(RpcMetrics::new());
        let monitoring = Arc::new(MonitoringSystem::new(mining_metrics, rpc_metrics));

        // Update component health
        monitoring
            .update_component_health(
                "test_component".to_string(),
                "healthy".to_string(),
                Some(100),
                None,
                None,
            )
            .await;

        let health = monitoring.get_health_status().await;
        assert_eq!(health.status, "healthy");
        assert!(health.components.contains_key("test_component"));
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let mining_metrics = Arc::new(MiningMetrics::new(&[PowAlgo::Sha256d]));
        let rpc_metrics = Arc::new(RpcMetrics::new());
        let monitoring = Arc::new(MonitoringSystem::new(mining_metrics, rpc_metrics));

        monitoring.record_http_request();
        monitoring.record_websocket_connection();
        monitoring.record_system_error();

        assert_eq!(monitoring.http_requests_total.load(Ordering::Relaxed), 1);
        assert_eq!(monitoring.websocket_connections.load(Ordering::Relaxed), 1);
        assert_eq!(monitoring.websocket_connections_active.load(Ordering::Relaxed), 1);
        assert_eq!(monitoring.system_errors_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_prometheus_format() {
        let mining_metrics = Arc::new(MiningMetrics::new(&[PowAlgo::Sha256d]));
        let rpc_metrics = Arc::new(RpcMetrics::new());
        let monitoring = MonitoringSystem::new(mining_metrics, rpc_metrics);

        let output = monitoring.get_prometheus_metrics();
        assert!(output.contains("http_requests_total"));
        assert!(output.contains("websocket_connections_total"));
        assert!(output.contains("system_uptime_seconds"));
    }
}