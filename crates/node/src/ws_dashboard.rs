//! WebSocket realtime dashboard for mining pool stats.
//!
//! Streams live metrics to connected clients for visualization.

use bitquan_consensus::pow::PowAlgo;
use bitquan_types::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

use crate::stratum_server::{MinerSession, StratumMetrics};

/// Dashboard configuration.
#[derive(Clone, Debug)]
pub struct DashboardConfig {
    /// WebSocket bind address.
    pub bind_addr: String,
    /// Update interval in seconds.
    pub update_interval: u64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8081".to_string(),
            update_interval: 5,
        }
    }
}

/// Aggregated pool statistics snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolStats {
    /// Unix timestamp.
    pub timestamp: u64,
    /// Number of active miners.
    pub active_miners: usize,
    /// Estimated SHA-256d hashrate (H/s).
    pub hashrate_sha256d: f64,
    /// Estimated RandomX hashrate (H/s).
    #[cfg(feature = "randomx")]
    pub hashrate_randomx: f64,
    /// Total accepted shares.
    pub shares_ok: u64,
    /// Total rejected shares.
    pub shares_rejected: u64,
}

/// Individual miner information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinerInfo {
    /// Miner address/username.
    pub address: String,
    /// Mining algorithm.
    pub algo: String,
    /// Current difficulty.
    pub difficulty: f64,
    /// Accepted shares.
    pub shares_ok: u64,
    /// Rejected shares.
    pub shares_rejected: u64,
    /// Connection uptime (seconds).
    pub uptime: u64,
}

/// WebSocket dashboard server.
pub struct WsDashboard {
    /// Configuration.
    config: DashboardConfig,
    /// Broadcast channel for pool stats.
    stats_tx: broadcast::Sender<PoolStats>,
    /// Broadcast channel for miner list.
    miners_tx: broadcast::Sender<Vec<MinerInfo>>,
}

impl WsDashboard {
    /// Create a new dashboard server.
    pub fn new(config: DashboardConfig) -> Self {
        let (stats_tx, _) = broadcast::channel(16);
        let (miners_tx, _) = broadcast::channel(16);

        Self {
            config,
            stats_tx,
            miners_tx,
        }
    }

    /// Start the dashboard server.
    pub async fn start(
        &self,
        peers: Arc<DashMap<String, MinerSession>>,
        metrics: Arc<StratumMetrics>,
    ) -> Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr)
            .await
            .map_err(|e| {
                bitquan_types::Error::Invalid(format!("failed to bind dashboard: {}", e))
            })?;

        println!(
            "Dashboard: WebSocket server listening on {}",
            self.config.bind_addr
        );

        // Spawn stats broadcaster
        self.spawn_stats_broadcaster(Arc::clone(&peers), Arc::clone(&metrics));

        // Accept WebSocket connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let stats_rx = self.stats_tx.subscribe();
                    let miners_rx = self.miners_tx.subscribe();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_ws_connection(stream, addr, stats_rx, miners_rx).await
                        {
                            eprintln!("Dashboard: WebSocket error {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Dashboard: Accept error: {}", e);
                }
            }
        }
    }

    /// Spawn background task to broadcast stats.
    fn spawn_stats_broadcaster(
        &self,
        peers: Arc<DashMap<String, MinerSession>>,
        metrics: Arc<StratumMetrics>,
    ) {
        let stats_tx = self.stats_tx.clone();
        let miners_tx = self.miners_tx.clone();
        let interval_secs = self.config.update_interval;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;

                // Collect pool stats
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let active_miners = peers.len();

                let shares_ok_sha256d = metrics.get_accepted(PowAlgo::Sha256d);
                let shares_rejected_sha256d = metrics.get_rejected(PowAlgo::Sha256d);

                #[cfg(feature = "randomx")]
                let _shares_ok_randomx = metrics.get_accepted(PowAlgo::RandomX);
                #[cfg(feature = "randomx")]
                let _shares_rejected_randomx = metrics.get_rejected(PowAlgo::RandomX);

                let shares_ok = shares_ok_sha256d;
                let shares_rejected = shares_rejected_sha256d;

                // Estimate hashrate (simplified)
                let hashrate_sha256d = estimate_hashrate(&peers, PowAlgo::Sha256d);

                #[cfg(feature = "randomx")]
                let hashrate_randomx = estimate_hashrate(&peers, PowAlgo::RandomX);

                let stats = PoolStats {
                    timestamp,
                    active_miners,
                    hashrate_sha256d,
                    #[cfg(feature = "randomx")]
                    hashrate_randomx,
                    shares_ok,
                    shares_rejected,
                };

                let _ = stats_tx.send(stats);

                // Collect miner info
                let miners: Vec<MinerInfo> = peers
                    .iter()
                    .map(|entry| {
                        let session = entry.value();
                        MinerInfo {
                            address: session.address.clone(),
                            algo: session.algo.name().to_string(),
                            difficulty: session.difficulty,
                            shares_ok: session.get_accepted(),
                            shares_rejected: session.get_rejected(),
                            uptime: session.connected_at.elapsed().as_secs(),
                        }
                    })
                    .collect();

                let _ = miners_tx.send(miners);
            }
        });
    }
}

/// Estimate hashrate for a given algorithm based on active miners.
fn estimate_hashrate(peers: &Arc<DashMap<String, MinerSession>>, algo: PowAlgo) -> f64 {
    peers
        .iter()
        .filter(|entry| entry.value().algo == algo)
        .map(|entry| {
            let session = entry.value();
            // Rough estimate: difficulty / target_time
            // Assume 15s target share time
            let target_time = 15.0;
            session.difficulty * 4_294_967_296.0 / target_time
        })
        .sum()
}

/// Handle a WebSocket connection (simplified HTTP upgrade).
async fn handle_ws_connection(
    stream: TcpStream,
    addr: SocketAddr,
    mut stats_rx: broadcast::Receiver<PoolStats>,
    mut miners_rx: broadcast::Receiver<Vec<MinerInfo>>,
) -> Result<()> {
    // Note: This is a simplified implementation that sends JSON over raw TCP
    // In production, use a proper WebSocket library like tokio-tungstenite

    println!("Dashboard: New connection from {}", addr);

    use tokio::io::AsyncWriteExt;
    let (_, mut writer) = stream.into_split();

    // Send initial HTTP response (simplified)
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n";
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|e| bitquan_types::Error::Invalid(format!("write error: {}", e)))?;

    // Stream stats as newline-delimited JSON
    loop {
        tokio::select! {
            Ok(stats) = stats_rx.recv() => {
                let json = serde_json::to_string(&stats).unwrap();
                let line = format!("{{\"type\":\"stats\",\"data\":{}}}\n", json);
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
            Ok(miners) = miners_rx.recv() => {
                let json = serde_json::to_string(&miners).unwrap();
                let line = format!("{{\"type\":\"miners\",\"data\":{}}}\n", json);
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
        }
    }

    println!("Dashboard: Connection closed {}", addr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_stats_serialization() {
        let stats = PoolStats {
            timestamp: 1730500000,
            active_miners: 14,
            hashrate_sha256d: 1.3e9,
            #[cfg(feature = "randomx")]
            hashrate_randomx: 8.1e7,
            shares_ok: 2034,
            shares_rejected: 57,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"timestamp\":1730500000"));
        assert!(json.contains("\"active_miners\":14"));
    }

    #[test]
    fn test_miner_info_serialization() {
        let miner = MinerInfo {
            address: "miner1".to_string(),
            algo: "sha256d".to_string(),
            difficulty: 1.0,
            shares_ok: 100,
            shares_rejected: 5,
            uptime: 3600,
        };

        let json = serde_json::to_string(&miner).unwrap();
        assert!(json.contains("\"address\":\"miner1\""));
        assert!(json.contains("\"algo\":\"sha256d\""));
    }
}
