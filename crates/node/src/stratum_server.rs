//! Stratum V1 mining server for BitQuan hybrid PoW.
//!
//! Supports external miners connecting via TCP to submit SHA-256d or RandomX shares.

use bitquan_consensus::pow::{meets_target, sha256d_pow_hash, target_from_bits, PowAlgo};
use bitquan_types::{Block, NetworkId, Result};
use dashmap::DashMap;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

#[cfg(feature = "randomx")]
use bitquan_consensus::pow::randomx_pow_hash;

use crate::block_submit::{BlockSubmitter, SubmitResult};
use crate::pool_template::{BlockTemplate, PoolTemplateManager};
use crate::vardiff::VarDiff;

/// Bounded channel capacity for share verification queue.
const STRATUM_QUEUE_CAP: usize = 1024;

/// Share submission task for worker pool.
#[derive(Debug, Clone)]
struct ShareTask {
    peer_key: String,
    algo: PowAlgo,
    nonce: u64,
    template: BlockTemplate,
}

/// Share verification result from worker pool.
#[derive(Debug, Clone)]
struct ShareResult {
    peer_key: String,
    verdict: ShareVerdict,
    nonce: u64,
    is_block: bool,
    block: Option<Block>,
}

/// Share verifier worker.
struct ShareVerifier {
    task_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<ShareTask>>>,
    result_tx: mpsc::Sender<ShareResult>,
}

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
    /// Share task sender (for submitting shares to workers).
    share_tx: Option<mpsc::Sender<ShareTask>>,
    /// Share result receiver (for getting verification results).
    result_rx: Option<mpsc::Receiver<ShareResult>>,
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

/// Share verification result.
#[derive(Debug, Clone)]
pub enum ShareVerdict {
    /// Share accepted - meets difficulty.
    Accept {
        /// PoW hash that met the target.
        hash: [u8; 32],
        /// Target that was met.
        target: [u8; 32],
    },
    /// Share rejected with reason.
    Reject {
        /// Rejection reason code.
        reason: RejectReason,
    },
}

/// Share rejection reasons (for metrics and logging).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Hash does not meet target difficulty.
    LowDifficulty,
    /// Algorithm mismatch between share and template.
    AlgoMismatch,
    /// Template stale (height or prev_hash changed).
    Stale,
    /// Duplicate share submission.
    Duplicate,
    /// Invalid header format or serialization.
    InvalidHeader,
}

impl RejectReason {
    /// Get string representation for metrics label.
    pub fn as_str(&self) -> &'static str {
        match self {
            RejectReason::LowDifficulty => "low_difficulty",
            RejectReason::AlgoMismatch => "algo_mismatch",
            RejectReason::Stale => "stale",
            RejectReason::Duplicate => "duplicate",
            RejectReason::InvalidHeader => "invalid_header",
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
    /// Duplicate share cache (LRU of recent nonces).
    pub duplicate_cache: Arc<Mutex<LruCache<u64, ()>>>,
    /// Extranonce1 assigned at subscribe.
    pub extranonce1: u32,
    /// Current job_id being worked on.
    pub current_job_id: Arc<tokio::sync::RwLock<u64>>,
}

impl MinerSession {
    /// Create a new miner session.
    pub fn new(algo: PowAlgo, address: String, difficulty: f64) -> Self {
        // Duplicate cache: keep last 4096 nonces
        let cache_size = NonZeroUsize::new(4096).unwrap();
        let duplicate_cache = Arc::new(Mutex::new(LruCache::new(cache_size)));

        // Assign random extranonce1
        let extranonce1 = rand::random::<u32>();

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
            duplicate_cache,
            extranonce1,
            current_job_id: Arc::new(tokio::sync::RwLock::new(0)),
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

    /// Check if nonce is duplicate and mark it if not.
    pub async fn check_and_mark_duplicate(&self, nonce: u64) -> bool {
        let mut cache = self.duplicate_cache.lock().await;
        if cache.contains(&nonce) {
            true // Duplicate
        } else {
            cache.put(nonce, ());
            false // New
        }
    }

    /// Update current job_id.
    pub async fn set_job_id(&self, job_id: u64) {
        let mut current = self.current_job_id.write().await;
        *current = job_id;
    }

    /// Get current job_id.
    pub async fn get_job_id(&self) -> u64 {
        *self.current_job_id.read().await
    }
}

/// Stratum server metrics.
#[derive(Debug)]
pub struct StratumMetrics {
    /// Total connections.
    pub connections_total: AtomicU64,
    /// Accepted shares per algorithm.
    pub shares_accepted: DashMap<PowAlgo, AtomicU64>,
    /// Rejected shares per algorithm and reason.
    pub shares_rejected: DashMap<(PowAlgo, &'static str), AtomicU64>,
    /// Last valid share timestamp (Unix epoch).
    pub last_valid_share_timestamp: AtomicU64,
    /// Vardiff adjustments counter.
    pub vardiff_adjustments: AtomicU64,
    /// Blocks submitted (total attempts).
    pub blocks_submitted_total: AtomicU64,
    /// Blocks accepted by network.
    pub blocks_accepted_total: AtomicU64,
    /// Blocks rejected by network.
    pub blocks_rejected_total: AtomicU64,
    /// Last block submission timestamp.
    pub last_block_submit_timestamp: AtomicU64,
    /// Backpressure events (share queue full).
    pub stratum_backpressure_total: AtomicU64,
}

impl StratumMetrics {
    /// Create new metrics collector.
    pub fn new() -> Self {
        let shares_accepted = DashMap::new();
        shares_accepted.insert(PowAlgo::Sha256d, AtomicU64::new(0));

        #[cfg(feature = "randomx")]
        {
            shares_accepted.insert(PowAlgo::RandomX, AtomicU64::new(0));
        }

        Self {
            connections_total: AtomicU64::new(0),
            shares_accepted,
            shares_rejected: DashMap::new(), // Now DashMap<(PowAlgo, &'static str), AtomicU64>
            last_valid_share_timestamp: AtomicU64::new(0),
            vardiff_adjustments: AtomicU64::new(0),
            blocks_submitted_total: AtomicU64::new(0),
            blocks_accepted_total: AtomicU64::new(0),
            blocks_rejected_total: AtomicU64::new(0),
            last_block_submit_timestamp: AtomicU64::new(0),
            stratum_backpressure_total: AtomicU64::new(0),
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
        self.last_valid_share_timestamp
            .store(now, Ordering::Relaxed);
    }

    /// Record rejected share with reason.
    pub fn record_share_rejected(&self, algo: PowAlgo, reason: RejectReason) {
        let key = (algo, reason.as_str());
        self.shares_rejected
            .entry(key)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get total accepted shares for algorithm.
    pub fn get_accepted(&self, algo: PowAlgo) -> u64 {
        self.shares_accepted
            .get(&algo)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get total rejected shares for algorithm (all reasons).
    pub fn get_rejected(&self, algo: PowAlgo) -> u64 {
        self.shares_rejected
            .iter()
            .filter(|entry| entry.key().0 == algo)
            .map(|entry| entry.value().load(Ordering::Relaxed))
            .sum()
    }

    /// Get rejected shares for specific reason.
    pub fn get_rejected_by_reason(&self, algo: PowAlgo, reason: RejectReason) -> u64 {
        let key = (algo, reason.as_str());
        self.shares_rejected
            .get(&key)
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

    /// Record block submission attempt.
    pub fn record_block_submitted(&self) {
        self.blocks_submitted_total.fetch_add(1, Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_block_submit_timestamp
            .store(now, Ordering::Relaxed);
    }

    /// Record block accepted by network.
    pub fn record_block_accepted(&self) {
        self.blocks_accepted_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record block rejected by network.
    pub fn record_block_rejected(&self) {
        self.blocks_rejected_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total blocks submitted.
    pub fn get_blocks_submitted(&self) -> u64 {
        self.blocks_submitted_total.load(Ordering::Relaxed)
    }

    /// Get total blocks accepted.
    pub fn get_blocks_accepted(&self) -> u64 {
        self.blocks_accepted_total.load(Ordering::Relaxed)
    }

    /// Get total blocks rejected.
    pub fn get_blocks_rejected(&self) -> u64 {
        self.blocks_rejected_total.load(Ordering::Relaxed)
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
            let ((algo, reason), counter) = entry.pair();
            output.push_str(&format!(
                "stratum_shares_total{{status=\"reject\",reason=\"{}\",algo=\"{}\"}} {}\n",
                reason,
                algo.name(),
                counter.load(Ordering::Relaxed)
            ));
        }

        output.push_str("# HELP stratum_active_miners Active miner connections\n");
        output.push_str("# TYPE stratum_active_miners gauge\n");
        output.push_str(&format!("stratum_active_miners {}\n", active_miners));

        output.push_str(
            "# HELP stratum_last_valid_share_timestamp Last valid share timestamp (Unix epoch)\n",
        );
        output.push_str("# TYPE stratum_last_valid_share_timestamp gauge\n");
        output.push_str(&format!(
            "stratum_last_valid_share_timestamp {}\n",
            self.get_last_valid_share_timestamp()
        ));

        output.push_str("# HELP stratum_vardiff_adjustments_total Total vardiff adjustments\n");
        output.push_str("# TYPE stratum_vardiff_adjustments_total counter\n");
        output.push_str(&format!(
            "stratum_vardiff_adjustments_total {}\n",
            self.vardiff_adjustments.load(Ordering::Relaxed)
        ));

        output
            .push_str("# HELP stratum_blocks_submitted_total Total blocks submitted to network\n");
        output.push_str("# TYPE stratum_blocks_submitted_total counter\n");
        output.push_str(&format!(
            "stratum_blocks_submitted_total {}\n",
            self.blocks_submitted_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP stratum_blocks_accepted_total Total blocks accepted by network\n");
        output.push_str("# TYPE stratum_blocks_accepted_total counter\n");
        output.push_str(&format!(
            "stratum_blocks_accepted_total {}\n",
            self.blocks_accepted_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP stratum_blocks_rejected_total Total blocks rejected by network\n");
        output.push_str("# TYPE stratum_blocks_rejected_total counter\n");
        output.push_str(&format!(
            "stratum_blocks_rejected_total {}\n",
            self.blocks_rejected_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP stratum_last_block_submit_timestamp Last block submission timestamp (Unix epoch)\n");
        output.push_str("# TYPE stratum_last_block_submit_timestamp gauge\n");
        output.push_str(&format!(
            "stratum_last_block_submit_timestamp {}\n",
            self.last_block_submit_timestamp.load(Ordering::Relaxed)
        ));

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
            Some(VarDiff::new(
                config.vardiff_target_time,
                config.vardiff_adjust_rate,
            ))
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
            share_tx: None,
            result_rx: None,
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

        println!("Stratum server listening on {}", self.config.bind_addr);
        println!("  Default difficulty: {}", self.config.default_difficulty);
        println!("  Network: {:?}", self.config.network);

        self.listener = Some(listener);

        // Create bounded channels for share verification
        let (share_tx, share_rx) = mpsc::channel(STRATUM_QUEUE_CAP);
        let (result_tx, result_rx) = mpsc::channel(STRATUM_QUEUE_CAP);

        // Wrap share_rx in Arc<Mutex> for worker pool sharing
        let share_rx = Arc::new(tokio::sync::Mutex::new(share_rx));

        // Spawn ShareVerifier worker pool
        let num_workers = std::cmp::max(2, num_cpus::get() / 2);
        println!("  ShareVerifier workers: {}", num_workers);

        for worker_id in 0..num_workers {
            let share_rx = Arc::clone(&share_rx);
            let result_tx = result_tx.clone();

            tokio::spawn(async move {
                let mut worker = ShareVerifier {
                    task_rx: share_rx,
                    result_tx,
                };
                worker.run(worker_id).await;
            });
        }

        // Drop original sender to avoid holding extra ref
        drop(result_tx);

        self.share_tx = Some(share_tx);
        self.result_rx = Some(result_rx);

        // Spawn result processor task
        let result_rx = self.result_rx.take().unwrap();
        let peers_clone = Arc::clone(&self.peers);
        let metrics_clone = Arc::clone(&self.metrics);
        let config_clone = self.config.clone();

        tokio::spawn(async move {
            process_share_results(result_rx, peers_clone, metrics_clone, config_clone).await;
        });

        // Accept loop
        loop {
            if self.stop_flag.load(Ordering::Relaxed) {
                println!("Stratum server shutting down...");
                break;
            }

            let listener = self.listener.as_ref().unwrap();
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(1), listener.accept()).await;

            match result {
                Ok(Ok((stream, addr))) => {
                    self.metrics
                        .connections_total
                        .fetch_add(1, Ordering::Relaxed);

                    let peers = Arc::clone(&self.peers);
                    let config = self.config.clone();
                    let metrics = Arc::clone(&self.metrics);
                    let template_manager = self.template_manager.clone();
                    let vardiff = self.vardiff.clone();
                    let share_tx = self.share_tx.as_ref().unwrap().clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_client(
                            stream,
                            addr,
                            peers,
                            config,
                            metrics,
                            template_manager,
                            vardiff,
                            share_tx,
                        )
                        .await
                        {
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
/// Handle an individual Stratum client connection.
#[allow(clippy::too_many_arguments)]
async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    peers: Arc<DashMap<String, MinerSession>>,
    config: StratumConfig,
    metrics: Arc<StratumMetrics>,
    template_manager: Option<Arc<PoolTemplateManager>>,
    vardiff: Option<VarDiff>,
    share_tx: mpsc::Sender<ShareTask>,
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

                let response = handle_request(
                    request,
                    &peer_key,
                    &peers,
                    &config,
                    &metrics,
                    &template_manager,
                    &vardiff,
                    &share_tx,
                )
                .await;
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
#[allow(clippy::too_many_arguments)]
async fn handle_request(
    request: JsonRpcRequest,
    peer_key: &str,
    peers: &Arc<DashMap<String, MinerSession>>,
    _config: &StratumConfig,
    metrics: &Arc<StratumMetrics>,
    template_manager: &Option<Arc<PoolTemplateManager>>,
    _vardiff: &Option<VarDiff>,
    share_tx: &mpsc::Sender<ShareTask>,
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
                .and_then(|p| p.first())
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

            // Quick parameter extraction
            let (nonce, algo) = match params {
                Some(p) => {
                    let nonce_str = p.get(4).and_then(|v| v.as_str());
                    let nonce = match nonce_str.and_then(|s| u64::from_str_radix(s, 16).ok()) {
                        Some(n) => n,
                        None => {
                            return JsonRpcResponse {
                                id: request.id,
                                result: Some(serde_json::json!(false)),
                                error: Some(JsonRpcError {
                                    code: 23,
                                    message: "Invalid nonce".to_string(),
                                }),
                            };
                        }
                    };

                    // Get session info
                    let session = match peers.get(peer_key) {
                        Some(s) => s,
                        None => {
                            return JsonRpcResponse {
                                id: request.id,
                                result: Some(serde_json::json!(false)),
                                error: Some(JsonRpcError {
                                    code: 25,
                                    message: "No session".to_string(),
                                }),
                            };
                        }
                    };

                    (nonce, session.algo)
                }
                None => {
                    return JsonRpcResponse {
                        id: request.id,
                        result: Some(serde_json::json!(false)),
                        error: Some(JsonRpcError {
                            code: 23,
                            message: "Missing params".to_string(),
                        }),
                    };
                }
            };

            // Get template
            let template = match template_manager {
                Some(tm) => match tm.get_template().await {
                    Some(t) => t,
                    None => {
                        return JsonRpcResponse {
                            id: request.id,
                            result: Some(serde_json::json!(false)),
                            error: Some(JsonRpcError {
                                code: 24,
                                message: "No template".to_string(),
                            }),
                        };
                    }
                },
                None => {
                    return JsonRpcResponse {
                        id: request.id,
                        result: Some(serde_json::json!(false)),
                        error: Some(JsonRpcError {
                            code: 24,
                            message: "Template manager unavailable".to_string(),
                        }),
                    };
                }
            };

            // Try to send to worker pool
            let task = ShareTask {
                peer_key: peer_key.to_string(),
                algo,
                nonce,
                template,
            };

            match share_tx.try_send(task) {
                Ok(_) => {
                    // Queued successfully - respond with acceptance
                    // Note: actual verification happens async in worker pool
                    JsonRpcResponse {
                        id: request.id,
                        result: Some(serde_json::json!(true)),
                        error: None,
                    }
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Backpressure! Queue is full
                    metrics
                        .stratum_backpressure_total
                        .fetch_add(1, Ordering::Relaxed);
                    println!("Stratum: Backpressure - share queue full from {}", peer_key);

                    JsonRpcResponse {
                        id: request.id,
                        result: Some(serde_json::json!(false)),
                        error: Some(JsonRpcError {
                            code: -20001,
                            message: "Server busy - try again".to_string(),
                        }),
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // Channel closed - should not happen
                    eprintln!("Stratum: Share channel closed!");
                    JsonRpcResponse {
                        id: request.id,
                        result: Some(serde_json::json!(false)),
                        error: Some(JsonRpcError {
                            code: -20002,
                            message: "Internal error".to_string(),
                        }),
                    }
                }
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

/// Handle a share submission with REAL PoW verification.
/// Submit a mined block to the network asynchronously.
async fn submit_block_async(block: Block, metrics: Arc<StratumMetrics>, network_id: NetworkId) {
    metrics.record_block_submitted();

    let submitter = BlockSubmitter::mock(network_id); // Use mock mode for now

    match submitter.submit(&block, None).await {
        Ok(SubmitResult::Accepted { hash, height }) => {
            metrics.record_block_accepted();
            let hash_hex = hex::encode(hash);
            println!(
                "[INFO] ✅ Block ACCEPTED by network! hash={} height={:?}",
                hash_hex, height
            );
        }
        Ok(SubmitResult::Rejected { reason }) => {
            metrics.record_block_rejected();
            eprintln!("[WARN] ❌ Block REJECTED by network: reason={}", reason);
        }
        Ok(SubmitResult::Error { message }) => {
            metrics.record_block_rejected();
            eprintln!("[ERROR] Block submission ERROR: {}", message);
        }
        Err(e) => {
            metrics.record_block_rejected();
            eprintln!("[ERROR] Block submission failed: {}", e);
        }
    }
}

/// Process share verification results from workers.
async fn process_share_results(
    mut result_rx: mpsc::Receiver<ShareResult>,
    peers: Arc<DashMap<String, MinerSession>>,
    metrics: Arc<StratumMetrics>,
    config: StratumConfig,
) {
    println!("[ResultProcessor] Started");

    while let Some(result) = result_rx.recv().await {
        let peer_key = &result.peer_key;

        // Get session
        let session = match peers.get(peer_key) {
            Some(s) => s,
            None => {
                // Session disconnected, skip
                continue;
            }
        };

        match result.verdict {
            ShareVerdict::Accept { hash, .. } => {
                // Share accepted!
                session.accept_share();
                metrics.record_share_accepted(session.algo);

                let hash_hex = hex::encode(&hash[..4]);
                println!(
                    "Stratum: Share ACCEPTED from {} (algo={}, nonce={}, hash={}…)",
                    session.address,
                    session.algo.name(),
                    result.nonce,
                    hash_hex
                );

                // Check if this is a block
                if result.is_block {
                    if let Some(block) = result.block {
                        println!(
                            "🎉 NEW BLOCK FOUND by {} (algo={}, hash={})",
                            session.address,
                            session.algo.name(),
                            hex::encode(hash)
                        );

                        // Submit block async
                        let metrics_clone = Arc::clone(&metrics);
                        let network = config.network;
                        tokio::spawn(async move {
                            submit_block_async(block, metrics_clone, network).await;
                        });
                    }
                }
            }
            ShareVerdict::Reject { reason } => {
                // Share rejected
                session.reject_share();
                metrics.record_share_rejected(session.algo, reason);

                eprintln!(
                    "Stratum: Share REJECTED from {} (reason={}, algo={}, nonce={})",
                    session.address,
                    reason.as_str(),
                    session.algo.name(),
                    result.nonce
                );
            }
        }
    }

    println!("[ResultProcessor] Stopped");
}

impl ShareVerifier {
    /// Run the worker loop - receives tasks, verifies shares in spawn_blocking, sends results.
    async fn run(&mut self, worker_id: usize) {
        println!("[ShareVerifier-{}] Started", worker_id);

        loop {
            // Lock receiver and get next task
            let task = {
                let mut rx = self.task_rx.lock().await;
                rx.recv().await
            };

            let task = match task {
                Some(t) => t,
                None => {
                    println!("[ShareVerifier-{}] Channel closed", worker_id);
                    break;
                }
            };

            // Clone data needed for blocking task
            let template = task.template.clone();
            let nonce = task.nonce;
            let algo = task.algo;

            // Verify share in blocking task (CPU-heavy PoW hashing)
            let verdict =
                tokio::task::spawn_blocking(move || verify_share_pow(&template, nonce, algo)).await;

            let verdict = match verdict {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => {
                    eprintln!("[ShareVerifier-{}] Verification error: {}", worker_id, e);
                    ShareVerdict::Reject {
                        reason: RejectReason::InvalidHeader,
                    }
                }
                Err(e) => {
                    eprintln!("[ShareVerifier-{}] Spawn error: {}", worker_id, e);
                    ShareVerdict::Reject {
                        reason: RejectReason::InvalidHeader,
                    }
                }
            };

            // Check if share is also a valid block
            let (is_block, block_opt) = match &verdict {
                ShareVerdict::Accept { hash, .. } => {
                    // Check against block target
                    let block_target = target_from_bits(task.template.header.bits);
                    let is_block = block_target.is_ok_and(|target| meets_target(hash, &target));

                    let block_opt = if is_block {
                        let mut header = task.template.header.clone();
                        header.nonce = task.nonce;
                        header.algo_id = task.algo.to_u8();

                        Some(Block {
                            header,
                            transactions: task.template.txs.clone(),
                        })
                    } else {
                        None
                    };

                    (is_block, block_opt)
                }
                _ => (false, None),
            };

            // Send result
            let result = ShareResult {
                peer_key: task.peer_key,
                verdict,
                nonce: task.nonce,
                is_block,
                block: block_opt,
            };

            if let Err(e) = self.result_tx.send(result).await {
                eprintln!("[ShareVerifier-{}] Failed to send result: {}", worker_id, e);
                break;
            }
        }

        println!("[ShareVerifier-{}] Stopped", worker_id);
    }
}

/// Verify a share against a block template using REAL PoW verification.
///
/// Returns ShareVerdict with accept/reject and reason.
fn verify_share_pow(
    tpl: &BlockTemplate,
    nonce: u64,
    algo: PowAlgo,
) -> std::result::Result<ShareVerdict, bitquan_types::Error> {
    // 1) Check algorithm match
    if algo != tpl.algo {
        return Ok(ShareVerdict::Reject {
            reason: RejectReason::AlgoMismatch,
        });
    }

    // 2) Build header with provided nonce
    let mut header = tpl.header.clone();
    header.nonce = nonce;
    header.algo_id = algo.to_u8();

    // 3) Serialize header for PoW
    let preimage = header.to_bytes();

    // 4) Compute PoW hash according to algorithm
    let pow_hash = match algo {
        PowAlgo::Sha256d => sha256d_pow_hash(&preimage),
        #[cfg(feature = "randomx")]
        PowAlgo::RandomX => {
            // Use seed from genesis hash (same as HybridMiner)
            let seed = [0u8; 32]; // TODO: get from consensus or config
            randomx_pow_hash(&preimage, &seed)
        }
    };

    // 5) Derive target from template bits
    let target = target_from_bits(tpl.header.bits)
        .map_err(|e| bitquan_types::Error::Invalid(format!("invalid bits: {}", e)))?;

    // 6) Compare hash with target
    if meets_target(&pow_hash, &target) {
        Ok(ShareVerdict::Accept {
            hash: pow_hash,
            target,
        })
    } else {
        Ok(ShareVerdict::Reject {
            reason: RejectReason::LowDifficulty,
        })
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

        metrics.record_share_rejected(PowAlgo::Sha256d, RejectReason::LowDifficulty);
        assert_eq!(metrics.get_rejected(PowAlgo::Sha256d), 1);
        assert_eq!(
            metrics.get_rejected_by_reason(PowAlgo::Sha256d, RejectReason::LowDifficulty),
            1
        );
    }
}
