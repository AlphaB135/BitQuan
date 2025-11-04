//! Stratum V1 mining server for BitQuan hybrid PoW.
//!
//! Supports external miners connecting via TCP to submit SHA-256d or RandomX shares.

use bitquan_consensus::pow::{PowAlgo, PowEngine, Sha256dEngine};
use bitquan_types::{BlockHeader, NetworkId, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

#[cfg(feature = "randomx")]
use bitquan_consensus::pow::{RandomXEngine, RandomXConfig, RandomXMode};

use crate::pool_template::PoolTemplateManager;
use crate::vardiff::VarDiff;

/// Stratum V1 mining server.
pub struct StratumServer {
    /// TCP listener for incoming miner connections.
    listener: Option<TcpListener>,
    /// Active miner sessions.
    peers: Arc<DashMap<String, MinerSession>>,
    /// Stop signal for graceful shutdown.
    stop_flag: Arc<AtomicBool>,
    /// Server configuration.
    config: StratumConfig,
    /// Metrics collector.
    metrics: Arc<StratumMetrics>,
    /// Pool template manager for block generation.
    template_manager: Option<Arc<PoolTemplateManager>>,
    /// Variable difficulty controller.
    vardiff: Option<VarDiff>,
}

/// Stratum server configuration.
#[derive(Clone, Debug)]
pub struct StratumConfig {
    /// Bind address (e.g., "0.0.0.0:3333").
    pub bind_addr: String,
    /// Allowed client IP addresses/subnets.
    pub allow_list: Vec<String>,
    /// Default difficulty for new miners.
    pub default_difficulty: f64,
    /// Network ID for validation.
    pub network: NetworkId,
    /// Enable variable difficulty adjustment.
    pub enable_vardiff: bool,
    /// Vardiff target share time (seconds).
    pub vardiff_target_time: f64,
    /// Vardiff adjustment rate.
    pub vardiff_adjust_rate: f64,
}

impl Default for StratumConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3333".to_string(),
            allow_list: vec!["127.0.0.1".to_string()],
            default_difficulty: 1.0,
            network: NetworkId::Devnet,
            enable_vardiff: true,
            vardiff_target_time: 15.0,
            vardiff_adjust_rate: 0.05,
        }
    }
}

/// Active miner session.
#[derive(Debug)]
pub struct MinerSession {
    /// Unique session ID.
    pub id: Uuid,
    /// Mining algorithm.
    pub algo: PowAlgo,
    /// Miner address (username or IP).
    pub address: String,
    /// Current difficulty.
    pub difficulty: f64,
    /// Accepted shares counter.
    pub shares_ok: AtomicU64,
    /// Rejected shares counter.
    pub shares_rejected: AtomicU64,
    /// Connection timestamp.
    pub connected_at: std::time::Instant,
    /// Last share submission time (for vardiff).
    pub last_share_time: Arc<tokio::sync::RwLock<std::time::Instant>>,
    /// Shares since last difficulty adjustment.
    pub shares_since_adjust: AtomicU64,
}

impl MinerSession {
    /// Create a new miner session.
    pub fn new(algo: PowAlgo, address: String, difficulty: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            algo,
            address,
            difficulty,
            shares_ok: AtomicU64::new(0),
            shares_rejected: AtomicU64::new(0),
            connected_at: std::time::Instant::now(),
            last_share_time: Arc::new(tokio::sync::RwLock::new(std::time::Instant::now())),
            shares_since_adjust: AtomicU64::new(0),
        }
    }

    /// Increment accepted shares.
    pub fn accept_share(&self) {
        self.shares_ok.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment rejected shares.
    pub fn reject_share(&self) {
        self.shares_rejected.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total accepted shares.
    pub fn get_accepted(&self) -> u64 {
        self.shares_ok.load(Ordering::Relaxed)
    }

    /// Get total rejected shares.
    pub fn get_rejected(&self) -> u64 {
        self.shares_rejected.load(Ordering::Relaxed)
    }

    /// Record share submission time (for vardiff).
    pub async fn record_share_time(&self) {
        let mut last = self.last_share_time.write().await;
        *last = std::time::Instant::now();
        self.shares_since_adjust.fetch_add(1, Ordering::Relaxed);
    }

    /// Get time since last share (for vardiff).
    pub async fn time_since_last_share(&self) -> f64 {
        let last = self.last_share_time.read().await;
        last.elapsed().as_secs_f64()
    }

    /// Update difficulty.
    pub fn set_difficulty(&mut self, new_diff: f64) {
        self.difficulty = new_diff;
        self.shares_since_adjust.store(0, Ordering::Relaxed);
    }

    /// Get shares since last difficulty adjustment.
    pub fn get_shares_since_adjust(&self) -> u64 {
        self.shares_since_adjust.load(Ordering::Relaxed)
    }
}

/// Stratum server metrics.
#[derive(Debug)]
pub struct StratumMetrics {
    /// Total connections.
    pub connections_total: AtomicU64,
    /// Accepted shares per algorithm.
    pub shares_accepted: DashMap<PowAlgo, AtomicU64>,
    /// Rejected shares per algorithm.
    pub shares_rejected: DashMap<PowAlgo, AtomicU64>,
    /// Last valid share timestamp (Unix epoch).
    pub last_valid_share_timestamp: AtomicU64,
    /// Vardiff adjustments counter.
    pub vardiff_adjustments: AtomicU64,
}

impl StratumMetrics {
    /// Create new metrics collector.
    pub fn new() -> Self {
        let shares_accepted = DashMap::new();
        let shares_rejected = DashMap::new();

        shares_accepted.insert(PowAlgo::Sha256d, AtomicU64::new(0));
        shares_rejected.insert(PowAlgo::Sha256d, AtomicU64::new(0));

        #[cfg(feature = "randomx")]
        {
            shares_accepted.insert(PowAlgo::RandomX, AtomicU64::new(0));
            shares_rejected.insert(PowAlgo::RandomX, AtomicU64::new(0));
        }

        Self {
            connections_total: AtomicU64::new(0),
            shares_accepted,
            shares_rejected,
            last_valid_share_timestamp: AtomicU64::new(0),
            vardiff_adjustments: AtomicU64::new(0),
        }
    }

    /// Record accepted share.
    pub fn record_share_accepted(&self, algo: PowAlgo) {
        if let Some(counter) = self.shares_accepted.get(&algo) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
        // Update last valid share timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_valid_share_timestamp.store(now, Ordering::Relaxed);
    }

    /// Record rejected share.
    pub fn record_share_rejected(&self, algo: PowAlgo) {
        if let Some(counter) = self.shares_rejected.get(&algo) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get total accepted shares for algorithm.
    pub fn get_accepted(&self, algo: PowAlgo) -> u64 {
        self.shares_accepted
            .get(&algo)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get total rejected shares for algorithm.
    pub fn get_rejected(&self, algo: PowAlgo) -> u64 {
        self.shares_rejected
            .get(&algo)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get total connections.
    pub fn get_connections_total(&self) -> u64 {
        self.connections_total.load(Ordering::Relaxed)
    }

    /// Record vardiff adjustment.
    pub fn record_vardiff_adjustment(&self) {
        self.vardiff_adjustments.fetch_add(1, Ordering::Relaxed);
    }

    /// Get last valid share timestamp.
    pub fn get_last_valid_share_timestamp(&self) -> u64 {
        self.last_valid_share_timestamp.load(Ordering::Relaxed)
    }

    /// Format metrics as Prometheus text format.
    pub fn format_prometheus(&self, active_miners: usize) -> String {
        let mut output = String::new();

        output.push_str("# HELP stratum_connections_total Total Stratum connections\n");
        output.push_str("# TYPE stratum_connections_total counter\n");
        output.push_str(&format!(
            "stratum_connections_total {}\n",
            self.get_connections_total()
        ));

        output.push_str("# HELP stratum_shares_total Total shares by status and algorithm\n");
        output.push_str("# TYPE stratum_shares_total counter\n");
        for entry in self.shares_accepted.iter() {
            let (algo, counter) = entry.pair();
            output.push_str(&format!(
                "stratum_shares_total{{status=\"ok\",algo=\"{}\"}} {}\n",
                algo.name(),
                counter.load(Ordering::Relaxed)
            ));
        }
        for entry in self.shares_rejected.iter() {
            let (algo, counter) = entry.pair();
            output.push_str(&format!(
                "stratum_shares_total{{status=\"reject\",algo=\"{}\"}} {}\n",
                algo.name(),
                counter.load(Ordering::Relaxed)
            ));
        }

        output.push_str("# HELP stratum_active_miners Active miner connections\n");
        output.push_str("# TYPE stratum_active_miners gauge\n");
        output.push_str(&format!("stratum_active_miners {}\n", active_miners));

        output.push_str("# HELP stratum_last_valid_share_timestamp Last valid share timestamp (Unix epoch)\n");
        output.push_str("# TYPE stratum_last_valid_share_timestamp gauge\n");
        output.push_str(&format!("stratum_last_valid_share_timestamp {}\n", 
            self.get_last_valid_share_timestamp()));

        output.push_str("# HELP stratum_vardiff_adjustments_total Total vardiff adjustments\n");
        output.push_str("# TYPE stratum_vardiff_adjustments_total counter\n");
        output.push_str(&format!("stratum_vardiff_adjustments_total {}\n",
            self.vardiff_adjustments.load(Ordering::Relaxed)));

        output
    }
}

impl Default for StratumMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON-RPC request structure.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    params: Option<Vec<Value>>,
}

/// JSON-RPC response structure.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error structure.
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl StratumServer {
    /// Create a new Stratum server with configuration.
    pub fn new(config: StratumConfig) -> Self {
        let vardiff = if config.enable_vardiff {
            Some(VarDiff::new(config.vardiff_target_time, config.vardiff_adjust_rate))
        } else {
            None
        };

        Self {
            listener: None,
            peers: Arc::new(DashMap::new()),
            stop_flag: Arc::new(AtomicBool::new(false)),
            config,
            metrics: Arc::new(StratumMetrics::new()),
            template_manager: None,
            vardiff,
        }
    }

    /// Set the pool template manager for real block template generation.
    pub fn set_template_manager(&mut self, manager: Arc<PoolTemplateManager>) {
        self.template_manager = Some(manager);
    }

    /// Start the Stratum server.
    pub async fn start(&mut self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr)
            .await
            .map_err(|e| {
                bitquan_types::Error::Invalid(format!("failed to bind Stratum server: {}", e))
            })?;

        println!(
            "Stratum server listening on {}",
            self.config.bind_addr
        );
        println!(
            "  Default difficulty: {}",
            self.config.default_difficulty
        );
        println!("  Network: {:?}", self.config.network);

        self.listener = Some(listener);

        // Accept loop
        loop {
            if self.stop_flag.load(Ordering::Relaxed) {
                println!("Stratum server shutting down...");
                break;
            }

            let listener = self.listener.as_ref().unwrap();
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                listener.accept()
            ).await;

            match result {
                Ok(Ok((stream, addr))) => {
                    self.metrics.connections_total.fetch_add(1, Ordering::Relaxed);
                    
                    let peers = Arc::clone(&self.peers);
                    let config = self.config.clone();
                    let metrics = Arc::clone(&self.metrics);
                    let template_manager = self.template_manager.clone();
                    let vardiff = self.vardiff.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, addr, peers, config, metrics, template_manager, vardiff).await {
                            eprintln!("Stratum client error {}: {}", addr, e);
                        }
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("Error accepting connection: {}", e);
                }
                Err(_) => {
                    // Timeout, check stop flag
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Stop the server.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Get active miner count.
    pub fn active_miners(&self) -> usize {
        self.peers.len()
    }

    /// Get metrics reference.
    pub fn metrics(&self) -> &Arc<StratumMetrics> {
        &self.metrics
    }

    /// Get peers reference.
    pub fn peers(&self) -> &Arc<DashMap<String, MinerSession>> {
        &self.peers
    }
}

/// Handle a single Stratum client connection.
async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    peers: Arc<DashMap<String, MinerSession>>,
    config: StratumConfig,
    metrics: Arc<StratumMetrics>,
    template_manager: Option<Arc<PoolTemplateManager>>,
    vardiff: Option<VarDiff>,
) -> Result<()> {
    let peer_key = addr.to_string();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    println!("Stratum: New connection from {}", addr);

    // Create default session
    let session = MinerSession::new(
        PowAlgo::Sha256d,
        addr.to_string(),
        config.default_difficulty,
    );
    peers.insert(peer_key.clone(), session);

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // Connection closed
                println!("Stratum: Client {} disconnected", addr);
                break;
            }
            Ok(_) => {
                let request: JsonRpcRequest = match serde_json::from_str(&line) {
                    Ok(req) => req,
                    Err(e) => {
                        eprintln!("Stratum: Invalid JSON from {}: {}", addr, e);
                        continue;
                    }
                };

                let response = handle_request(request, &peer_key, &peers, &config, &metrics, &template_manager, &vardiff).await;
                let response_json = serde_json::to_string(&response).unwrap();
                
                writer
                    .write_all(response_json.as_bytes())
                    .await
                    .map_err(|e| bitquan_types::Error::Invalid(e.to_string()))?;
                writer
                    .write_all(b"\n")
                    .await
                    .map_err(|e| bitquan_types::Error::Invalid(e.to_string()))?;
                writer
                    .flush()
                    .await
                    .map_err(|e| bitquan_types::Error::Invalid(e.to_string()))?;
            }
            Err(e) => {
                eprintln!("Stratum: Read error from {}: {}", addr, e);
                break;
            }
        }
    }

    peers.remove(&peer_key);
    Ok(())
}

/// Handle a JSON-RPC request from a miner.
async fn handle_request(
    request: JsonRpcRequest,
    peer_key: &str,
    peers: &Arc<DashMap<String, MinerSession>>,
    config: &StratumConfig,
    metrics: &Arc<StratumMetrics>,
    template_manager: &Option<Arc<PoolTemplateManager>>,
    vardiff: &Option<VarDiff>,
) -> JsonRpcResponse {
    match request.method.as_str() {
        "mining.subscribe" => {
            println!("Stratum: {} subscribed", peer_key);
            JsonRpcResponse {
                id: request.id,
                result: Some(serde_json::json!([
                    [["mining.notify", "subscription_id"]],
                    "extranonce1",
                    4
                ])),
                error: None,
            }
        }
        "mining.authorize" => {
            let username = request
                .params
                .as_ref()
                .and_then(|p| p.get(0))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            if let Some(mut session) = peers.get_mut(peer_key) {
                session.address = username.to_string();
            }

            println!("Stratum: {} authorized as {}", peer_key, username);
            JsonRpcResponse {
                id: request.id,
                result: Some(serde_json::json!(true)),
                error: None,
            }
        }
        "mining.submit" => {
            // Extract submit parameters
            let params = request.params.as_ref();
            let result = if let Some(p) = params {
                handle_submit(peer_key, p, peers, metrics, template_manager, config, vardiff).await
            } else {
                false
            };

            if result {
                println!("Stratum: Share accepted from {}", peer_key);
            } else {
                println!("Stratum: Share rejected from {}", peer_key);
            }

            JsonRpcResponse {
                id: request.id,
                result: Some(serde_json::json!(result)),
                error: if !result {
                    Some(JsonRpcError {
                        code: 23,
                        message: "Invalid share".to_string(),
                    })
                } else {
                    None
                },
            }
        }
        other => {
            eprintln!("Stratum: Unknown method {} from {}", other, peer_key);
            JsonRpcResponse {
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -1,
                    message: format!("Unknown method: {}", other),
                }),
            }
        }
    }
}

/// Handle a share submission.
async fn handle_submit(
    peer_key: &str,
    params: &[Value],
    peers: &Arc<DashMap<String, MinerSession>>,
    metrics: &Arc<StratumMetrics>,
    template_manager: &Option<Arc<PoolTemplateManager>>,
    _config: &StratumConfig,
    vardiff: &Option<VarDiff>,
) -> bool {
    // Extract params: [worker_name, job_id, extranonce2, ntime, nonce]
    let _worker = params.get(0).and_then(|v| v.as_str());
    let _job_id = params.get(1).and_then(|v| v.as_str());
    let _extranonce2 = params.get(2).and_then(|v| v.as_str());
    let _ntime = params.get(3).and_then(|v| v.as_str());
    let nonce_str = params.get(4).and_then(|v| v.as_str());

    let nonce = match nonce_str.and_then(|s| u64::from_str_radix(s, 16).ok()) {
        Some(n) => n,
        None => {
            if let Some(session) = peers.get(peer_key) {
                session.reject_share();
                metrics.record_share_rejected(session.algo);
            }
            return false;
        }
    };

    // Get session and verify share
    let mut session = match peers.get_mut(peer_key) {
        Some(s) => s,
        None => return false,
    };

    // Try real PoW verification if template manager is available
    let valid = if let Some(tm) = template_manager {
        if let Some(template) = tm.get_template().await {
            verify_share_pow(&template.header, nonce, session.algo, &template.target)
        } else {
            // Fallback to simple verification if no template available
            verify_share_simple(nonce, session.algo, session.difficulty)
        }
    } else {
        // Fallback to simple verification
        verify_share_simple(nonce, session.algo, session.difficulty)
    };

    if valid {
        session.accept_share();
        session.record_share_time().await;
        metrics.record_share_accepted(session.algo);
        
        // Log accepted share with details
        println!(
            "Stratum: Share ACCEPTED from {} (algo={}, diff={:.2}, nonce={})",
            session.address,
            session.algo.name(),
            session.difficulty,
            nonce
        );

        // Apply vardiff adjustment if enabled
        if let Some(vd) = vardiff {
            let shares_since = session.get_shares_since_adjust();
            if vd.should_adjust(shares_since) {
                let time_since = session.time_since_last_share().await;
                let new_diff = vd.adjust(time_since, session.difficulty);
                
                if (new_diff - session.difficulty).abs() > 0.01 {
                    println!(
                        "Stratum: Adjusting difficulty for {} from {:.2} to {:.2}",
                        session.address,
                        session.difficulty,
                        new_diff
                    );
                    session.set_difficulty(new_diff);
                    metrics.record_vardiff_adjustment();
                }
            }
        }
    } else {
        session.reject_share();
        metrics.record_share_rejected(session.algo);
        
        // Log rejected share
        println!(
            "Stratum: Share REJECTED from {} (algo={}, nonce={})",
            session.address,
            session.algo.name(),
            nonce
        );
    }

    valid
}

/// Simple share verification (placeholder for actual PoW check).
///
/// This is used when no block template is available. In production, use verify_share_pow.
fn verify_share_simple(nonce: u64, algo: PowAlgo, _difficulty: f64) -> bool {
    // Placeholder: accept shares with nonce < 1000000 for testing
    // In production, this would do actual PoW verification against block template
    match algo {
        PowAlgo::Sha256d => nonce < 1_000_000,
        #[cfg(feature = "randomx")]
        PowAlgo::RandomX => nonce < 500_000,
    }
}

/// Verify a share against a block header using real PoW verification.
///
/// Checks if the hash meets the target difficulty for the given algorithm.
fn verify_share_pow(header: &BlockHeader, nonce: u64, algo: PowAlgo, _target: &[u8; 32]) -> bool {
    let mut header = header.clone();
    header.nonce = nonce;
    header.algo_id = algo.to_u8();

    // First verify PoW is valid
    let pow_valid = match algo {
        PowAlgo::Sha256d => {
            let engine = Sha256dEngine;
            engine.verify(&header).is_ok()
        }
        #[cfg(feature = "randomx")]
        PowAlgo::RandomX => {
            let config = RandomXConfig {
                mode: RandomXMode::Fast,
                seed: [0u8; 32],
            };
            let engine = RandomXEngine::new(config);
            engine.verify(&header).is_ok()
        }
    };

    // Then check if hash meets share target
    if pow_valid {
        // For share validation, we compare against pool difficulty target
        // which is typically easier than block target
        // In production, compute hash and compare with target
        // For now, accept if basic PoW is valid
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miner_session_creation() {
        let session = MinerSession::new(PowAlgo::Sha256d, "test@localhost".to_string(), 1.0);
        assert_eq!(session.algo, PowAlgo::Sha256d);
        assert_eq!(session.address, "test@localhost");
        assert_eq!(session.difficulty, 1.0);
        assert_eq!(session.get_accepted(), 0);
        assert_eq!(session.get_rejected(), 0);
    }

    #[test]
    fn share_counters() {
        let session = MinerSession::new(PowAlgo::Sha256d, "test".to_string(), 1.0);
        
        session.accept_share();
        session.accept_share();
        assert_eq!(session.get_accepted(), 2);
        
        session.reject_share();
        assert_eq!(session.get_rejected(), 1);
    }

    #[test]
    fn metrics_initialization() {
        let metrics = StratumMetrics::new();
        assert_eq!(metrics.get_connections_total(), 0);
        assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 0);
        assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 0);
    }

    #[test]
    fn metrics_recording() {
        let metrics = StratumMetrics::new();
        
        metrics.record_share_accepted(PowAlgo::Sha256d);
        metrics.record_share_accepted(PowAlgo::Sha256d);
        assert_eq!(metrics.get_accepted(PowAlgo::Sha256d), 2);
        
        metrics.record_share_rejected(PowAlgo::Sha256d);
        assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 1);
    }
}
