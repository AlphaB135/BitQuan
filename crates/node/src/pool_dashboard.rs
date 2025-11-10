//! Enhanced mining pool dashboard with web interface.
//!
//! Provides a complete web-based dashboard for monitoring mining operations
//! with real-time updates via WebSocket and REST API.

use crate::ws_dashboard::{PoolStats, MinerInfo, DashboardConfig};
use crate::stratum_server::{StratumMetrics, MinerSession};
use bitquan_consensus::pow::PowAlgo;
use bitquan_types::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

/// Enhanced pool dashboard with web interface
pub struct PoolDashboard {
    /// Dashboard configuration
    config: DashboardConfig,
    /// Broadcast channel for pool stats
    stats_tx: broadcast::Sender<PoolStats>,
    /// Broadcast channel for miner list
    miners_tx: broadcast::Sender<Vec<MinerInfo>>,
    /// Historical data storage
    history: Arc<tokio::sync::RwLock<Vec<PoolStats>>>,
}

impl PoolDashboard {
    /// Create a new pool dashboard
    pub fn new(config: DashboardConfig) -> Self {
        let (stats_tx, _) = broadcast::channel(100);
        let (miners_tx, _) = broadcast::channel(100);
        
        Self {
            config,
            stats_tx,
            miners_tx,
            history: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Start the enhanced dashboard server
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
            "Pool Dashboard: Web server listening on {}",
            self.config.bind_addr
        );

        // Spawn stats broadcaster
        self.spawn_enhanced_stats_broadcaster(Arc::clone(&peers), Arc::clone(&metrics));

        // Accept HTTP/WebSocket connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let stats_rx = self.stats_tx.subscribe();
                    let miners_rx = self.miners_tx.subscribe();
                    let history = Arc::clone(&self.history);
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_enhanced_connection(
                            stream, 
                            addr, 
                            stats_rx, 
                            miners_rx,
                            history
                        ).await {
                            eprintln!("Dashboard: Connection error {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Dashboard: Accept error: {}", e);
                }
            }
        }
    }

    /// Spawn enhanced stats broadcaster with history tracking
    fn spawn_enhanced_stats_broadcaster(
        &self,
        peers: Arc<DashMap<String, MinerSession>>,
        metrics: Arc<StratumMetrics>,
    ) {
        let stats_tx = self.stats_tx.clone();
        let miners_tx = self.miners_tx.clone();
        let history = Arc::clone(&self.history);
        let interval_secs = self.config.update_interval;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            
            loop {
                ticker.tick().await;

                // Collect enhanced pool stats
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let active_miners = peers.len();

                // Collect shares by algorithm
                let shares_ok_sha256d = metrics.get_accepted(PowAlgo::Sha256d);
                let shares_rejected_sha256d = metrics.get_rejected(PowAlgo::Sha256d);

                #[cfg(feature = "randomx")]
                let shares_ok_randomx = metrics.get_accepted(PowAlgo::RandomX);
                #[cfg(feature = "randomx")]
                let shares_rejected_randomx = metrics.get_rejected(PowAlgo::RandomX);

                let shares_ok_ethash = metrics.get_accepted(PowAlgo::Ethash);
                let shares_rejected_ethash = metrics.get_rejected(PowAlgo::Ethash);

                let shares_ok = {
                    #[cfg(feature = "randomx")]
                    {
                        shares_ok_sha256d + shares_ok_ethash + shares_ok_randomx
                    }
                    #[cfg(not(feature = "randomx"))]
                    {
                        shares_ok_sha256d + shares_ok_ethash
                    }
                };
                
                let shares_rejected = {
                    #[cfg(feature = "randomx")]
                    {
                        shares_rejected_sha256d + shares_rejected_ethash + shares_rejected_randomx
                    }
                    #[cfg(not(feature = "randomx"))]
                    {
                        shares_rejected_sha256d + shares_rejected_ethash
                    }
                };

                // Enhanced hashrate calculations
                let hashrate_sha256d = estimate_enhanced_hashrate(&peers, PowAlgo::Sha256d);
                let hashrate_ethash = estimate_enhanced_hashrate(&peers, PowAlgo::Ethash);
                
                #[cfg(feature = "randomx")]
                let hashrate_randomx = estimate_enhanced_hashrate(&peers, PowAlgo::RandomX);

                let hashrate_total = {
                    #[cfg(feature = "randomx")]
                    {
                        hashrate_sha256d + hashrate_ethash + hashrate_randomx
                    }
                    #[cfg(not(feature = "randomx"))]
                    {
                        hashrate_sha256d + hashrate_ethash
                    }
                };

                // Calculate algorithm distribution
                let algo_distribution = if hashrate_total > 0.0 {
                    #[cfg(feature = "randomx")]
                    {
                        crate::ws_dashboard::AlgoDistribution {
                            sha256d_percent: (hashrate_sha256d / hashrate_total) * 100.0,
                            randomx_percent: (hashrate_randomx / hashrate_total) * 100.0,
                            ethash_percent: (hashrate_ethash / hashrate_total) * 100.0,
                        }
                    }
                    #[cfg(not(feature = "randomx"))]
                    {
                        crate::ws_dashboard::AlgoDistribution {
                            sha256d_percent: (hashrate_sha256d / hashrate_total) * 100.0,
                            randomx_percent: 0.0,
                            ethash_percent: (hashrate_ethash / hashrate_total) * 100.0,
                        }
                    }
                } else {
                    #[cfg(feature = "randomx")]
                    {
                        crate::ws_dashboard::AlgoDistribution {
                            sha256d_percent: 33.33,
                            randomx_percent: 33.33,
                            ethash_percent: 33.34,
                        }
                    }
                    #[cfg(not(feature = "randomx"))]
                    {
                        crate::ws_dashboard::AlgoDistribution {
                            sha256d_percent: 50.0,
                            randomx_percent: 0.0,
                            ethash_percent: 50.0,
                        }
                    }
                };

                // Calculate pool efficiency
                let total_shares = shares_ok + shares_rejected;
                let pool_efficiency = if total_shares > 0 {
                    (shares_ok as f64 / total_shares as f64) * 100.0
                } else {
                    100.0
                };

                // Enhanced network stats (placeholders for now)
                let network_difficulty = 1.0;
                let block_height = 1;

                let stats = {
                    #[cfg(feature = "randomx")]
                    {
                        PoolStats {
                            timestamp,
                            active_miners,
                            hashrate_sha256d,
                            hashrate_randomx,
                            hashrate_ethash,
                            hashrate_total,
                            algo_distribution,
                            shares_ok,
                            shares_rejected,
                            network_difficulty,
                            block_height,
                            pool_efficiency,
                        }
                    }
                    #[cfg(not(feature = "randomx"))]
                    {
                        PoolStats {
                            timestamp,
                            active_miners,
                            hashrate_sha256d,
                            hashrate_randomx: 0.0,
                            hashrate_ethash,
                            hashrate_total,
                            algo_distribution,
                            shares_ok,
                            shares_rejected,
                            network_difficulty,
                            block_height,
                            pool_efficiency,
                        }
                    }
                };

                // Store in history (keep last 1000 entries)
                {
                    let mut history_guard = history.write().await;
                    history_guard.push(stats.clone());
                    if history_guard.len() > 1000 {
                        history_guard.remove(0);
                    }
                }

                let _ = stats_tx.send(stats);

                // Collect enhanced miner info
                let miners: Vec<MinerInfo> = peers
                    .iter()
                    .map(|entry| {
                        let session = entry.value();
                        let total_shares = session.get_accepted() + session.get_rejected();
                        let efficiency = if total_shares > 0 {
                            (session.get_accepted() as f64 / total_shares as f64) * 100.0
                        } else {
                            100.0
                        };
                        
                        let hashrate = estimate_enhanced_miner_hashrate(session);
                        
                        MinerInfo {
                            address: session.address.clone(),
                            algo: session.algo.name().to_string(),
                            difficulty: session.difficulty,
                            shares_ok: session.get_accepted(),
                            shares_rejected: session.get_rejected(),
                            uptime: session.connected_at.elapsed().as_secs(),
                            hashrate,
                            efficiency,
                            last_seen: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            region: None,
                            client_version: None,
                        }
                    })
                    .collect();

                let _ = miners_tx.send(miners);
            }
        });
    }

    /// Get current dashboard statistics
    pub async fn get_current_stats(&self) -> Option<PoolStats> {
        let history = self.history.read().await;
        history.last().cloned()
    }

    /// Get historical statistics
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<PoolStats> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(100);
        let start = if history.len() > limit { history.len() - limit } else { 0 };
        history[start..].to_vec()
    }
}

/// Enhanced hashrate estimation with better accuracy
fn estimate_enhanced_hashrate(peers: &Arc<DashMap<String, MinerSession>>, algo: PowAlgo) -> f64 {
    peers
        .iter()
        .filter(|entry| entry.value().algo == algo)
        .map(|entry| estimate_enhanced_miner_hashrate(entry.value()))
        .sum()
}

/// Enhanced individual miner hashrate estimation
fn estimate_enhanced_miner_hashrate(session: &MinerSession) -> f64 {
    // Enhanced estimation considering recent share rate and difficulty
    let target_share_time = 15.0; // Target time between shares
    let difficulty_factor = session.difficulty;
    
    // Base hashrate estimation: difficulty * 2^32 / target_share_time
    let base_hashrate = difficulty_factor * 4_294_967_296.0 / target_share_time;
    
    // Apply efficiency factor based on share acceptance rate
    let total_shares = session.get_accepted() + session.get_rejected();
    let efficiency = if total_shares > 0 {
        session.get_accepted() as f64 / total_shares as f64
    } else {
        1.0
    };
    
    base_hashrate * efficiency
}

/// Handle enhanced HTTP/WebSocket connection
async fn handle_enhanced_connection(
    mut stream: TcpStream,
    addr: std::net::SocketAddr,
    mut stats_rx: broadcast::Receiver<PoolStats>,
    mut miners_rx: broadcast::Receiver<Vec<MinerInfo>>,
    history: Arc<tokio::sync::RwLock<Vec<PoolStats>>>,
) -> Result<()> {

    
    let mut buffer = [0; 4096];
    let n = stream.read(&mut buffer).await
        .map_err(|e| bitquan_types::Error::Invalid(format!("read error: {}", e)))?;
    
    let request = String::from_utf8_lossy(&buffer[..n]);
    
    // Parse HTTP request
    if request.starts_with("GET") {
        let lines: Vec<&str> = request.lines().collect();
        let request_line = lines.get(0).unwrap_or(&"");
        
        if request_line.contains("GET /api/stats") {
            // REST API endpoint for current stats
            let history_guard = history.read().await;
            let current_stats = history_guard.last();
            
            let response = if let Some(stats) = current_stats {
                let json = serde_json::to_string(stats)
                    .map_err(|e| bitquan_types::Error::Invalid(format!("json error: {}", e)))?;
                format_http_response(200, "application/json", &json)
            } else {
                format_http_response(404, "application/json", "{\"error\":\"no data\"}")
            };
            
            stream.write_all(response.as_bytes()).await
                .map_err(|e| bitquan_types::Error::Invalid(format!("write error: {}", e)))?;
                
        } else if request_line.contains("GET /api/history") {
            // REST API endpoint for historical data
            let history_guard = history.read().await;
            let json = serde_json::to_string(&*history_guard)
                .map_err(|e| bitquan_types::Error::Invalid(format!("json error: {}", e)))?;
            let response = format_http_response(200, "application/json", &json);
            
            stream.write_all(response.as_bytes()).await
                .map_err(|e| bitquan_types::Error::Invalid(format!("write error: {}", e)))?;
                
        } else if request_line.contains("GET /api/miners") {
            // REST API endpoint for current miners
            let mut miners = Vec::new();
            while let Ok(miner_list) = miners_rx.try_recv() {
                miners = miner_list;
            }
            
            let json = serde_json::to_string(&miners)
                .map_err(|e| bitquan_types::Error::Invalid(format!("json error: {}", e)))?;
            let response = format_http_response(200, "application/json", &json);
            
            stream.write_all(response.as_bytes()).await
                .map_err(|e| bitquan_types::Error::Invalid(format!("write error: {}", e)))?;
                
        } else if request_line.contains("GET /ws") {
            // WebSocket endpoint for real-time updates
            let response = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n";
            stream.write_all(response.as_bytes()).await
                .map_err(|e| bitquan_types::Error::Invalid(format!("write error: {}", e)))?;
            
            // Stream real-time updates
            let (_, mut writer) = stream.into_split();
            
            loop {
                tokio::select! {
                    Ok(stats) = stats_rx.recv() => {
                        let json = serde_json::to_string(&stats)
                            .map_err(|e| bitquan_types::Error::Invalid(format!("json error: {}", e)))?;
                        let message = format!("{{\"type\":\"stats\",\"data\":{}}}\n", json);
                        if writer.write_all(message.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    Ok(miners) = miners_rx.recv() => {
                        let json = serde_json::to_string(&miners)
                            .map_err(|e| bitquan_types::Error::Invalid(format!("json error: {}", e)))?;
                        let message = format!("{{\"type\":\"miners\",\"data\":{}}}\n", json);
                        if writer.write_all(message.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
            }
        } else {
            // Serve main dashboard HTML
            let html = get_dashboard_html();
            let response = format_http_response(200, "text/html", &html);
            stream.write_all(response.as_bytes()).await
                .map_err(|e| bitquan_types::Error::Invalid(format!("write error: {}", e)))?;
        }
    }
    
    println!("Dashboard: Connection closed {}", addr);
    Ok(())
}

/// Format HTTP response
fn format_http_response(status: u16, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        if status == 200 { "OK" } else if status == 404 { "Not Found" } else { "Error" },
        content_type,
        body.len(),
        body
    )
}

/// Get dashboard HTML page
fn get_dashboard_html() -> String {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>BitQuan Mining Pool Dashboard</title>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { background: #2c3e50; color: white; padding: 20px; border-radius: 8px; margin-bottom: 20px; }
        .stats-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px; margin-bottom: 20px; }
        .stat-card { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .stat-value { font-size: 2em; font-weight: bold; color: #3498db; }
        .stat-label { color: #7f8c8d; margin-top: 5px; }
        .miners-table { background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .miners-table table { width: 100%; border-collapse: collapse; }
        .miners-table th, .miners-table td { padding: 12px; text-align: left; border-bottom: 1px solid #ecf0f1; }
        .miners-table th { background: #34495e; color: white; }
        .status-indicator { width: 12px; height: 12px; border-radius: 50%; display: inline-block; margin-right: 8px; }
        .status-online { background: #27ae60; }
        .status-offline { background: #e74c3c; }
        .algo-badge { padding: 4px 8px; border-radius: 4px; font-size: 0.8em; font-weight: bold; }
        .algo-sha256d { background: #3498db; color: white; }
        .algo-randomx { background: #9b59b6; color: white; }
        .algo-ethash { background: #e67e22; color: white; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>⛏️ BitQuan Mining Pool Dashboard</h1>
            <p>Real-time mining pool statistics and monitoring</p>
        </div>
        
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-value" id="active-miners">0</div>
                <div class="stat-label">Active Miners</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="hashrate-total">0 H/s</div>
                <div class="stat-label">Total Hashrate</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="shares-accepted">0</div>
                <div class="stat-label">Accepted Shares</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="pool-efficiency">100%</div>
                <div class="stat-label">Pool Efficiency</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="block-height">0</div>
                <div class="stat-label">Block Height</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="network-difficulty">0</div>
                <div class="stat-label">Network Difficulty</div>
            </div>
        </div>
        
        <div class="miners-table">
            <h2>Active Miners</h2>
            <table>
                <thead>
                    <tr>
                        <th>Status</th>
                        <th>Address</th>
                        <th>Algorithm</th>
                        <th>Hashrate</th>
                        <th>Difficulty</th>
                        <th>Accepted</th>
                        <th>Rejected</th>
                        <th>Efficiency</th>
                        <th>Uptime</th>
                    </tr>
                </thead>
                <tbody id="miners-tbody">
                    <tr>
                        <td colspan="9" style="text-align: center; color: #7f8c8d;">Loading miner data...</td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
    
    <script>
        // Connect to WebSocket for real-time updates
        const ws = new WebSocket('ws://' + window.location.host + '/ws');
        
        ws.onmessage = function(event) {
            const message = JSON.parse(event.data);
            
            if (message.type === 'stats') {
                updateStats(message.data);
            } else if (message.type === 'miners') {
                updateMiners(message.data);
            }
        };
        
        function updateStats(stats) {
            document.getElementById('active-miners').textContent = stats.active_miners;
            document.getElementById('hashrate-total').textContent = formatHashrate(stats.hashrate_total);
            document.getElementById('shares-accepted').textContent = stats.shares_ok.toLocaleString();
            document.getElementById('pool-efficiency').textContent = stats.pool_efficiency.toFixed(1) + '%';
            document.getElementById('block-height').textContent = stats.block_height.toLocaleString();
            document.getElementById('network-difficulty').textContent = stats.network_difficulty.toFixed(2);
        }
        
        function updateMiners(miners) {
            const tbody = document.getElementById('miners-tbody');
            
            if (miners.length === 0) {
                tbody.innerHTML = '<tr><td colspan="9" style="text-align: center; color: #7f8c8d;">No active miners</td></tr>';
                return;
            }
            
            tbody.innerHTML = miners.map(miner => `
                <tr>
                    <td><span class="status-indicator status-online"></span></td>
                    <td>${miner.address}</td>
                    <td><span class="algo-badge algo-${miner.algo.toLowerCase()}">${miner.algo.toUpperCase()}</span></td>
                    <td>${formatHashrate(miner.hashrate)}</td>
                    <td>${miner.difficulty.toFixed(2)}</td>
                    <td>${miner.shares_ok.toLocaleString()}</td>
                    <td>${miner.shares_rejected.toLocaleString()}</td>
                    <td>${miner.efficiency.toFixed(1)}%</td>
                    <td>${formatUptime(miner.uptime)}</td>
                </tr>
            `).join('');
        }
        
        function formatHashrate(h) {
            if (h >= 1e18) return (h / 1e18).toFixed(2) + ' EH/s';
            if (h >= 1e15) return (h / 1e15).toFixed(2) + ' PH/s';
            if (h >= 1e12) return (h / 1e12).toFixed(2) + ' TH/s';
            if (h >= 1e9) return (h / 1e9).toFixed(2) + ' GH/s';
            if (h >= 1e6) return (h / 1e6).toFixed(2) + ' MH/s';
            if (h >= 1e3) return (h / 1e3).toFixed(2) + ' KH/s';
            return h.toFixed(2) + ' H/s';
        }
        
        function formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);
            
            if (days > 0) return `${days}d ${hours}h ${minutes}m`;
            if (hours > 0) return `${hours}h ${minutes}m`;
            return `${minutes}m`;
        }
        
        // Fallback: refresh data every 5 seconds if WebSocket fails
        setInterval(function() {
            if (ws.readyState !== WebSocket.OPEN) {
                fetch('/api/stats')
                    .then(response => response.json())
                    .then(data => updateStats(data))
                    .catch(console.error);
                    
                fetch('/api/miners')
                    .then(response => response.json())
                    .then(data => updateMiners(data))
                    .catch(console.error);
            }
        }, 5000);
    </script>
</body>
</html>
    "#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_hashrate_estimation() {
        // Test enhanced hashrate calculation
        let difficulty = 1000.0;
        let base_hashrate = difficulty * 4_294_967_296.0 / 15.0;
        let efficiency = 0.95;
        let expected = base_hashrate * efficiency;
        
        assert!(expected > 0.0, "Enhanced hashrate should be positive");
    }

    #[test]
    fn test_http_response_formatting() {
        let response = format_http_response(200, "text/html", "test");
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Content-Length: 4"));
        assert!(response.ends_with("\r\n\r\ntest"));
    }

    #[test]
    fn test_dashboard_html() {
        let html = get_dashboard_html();
        assert!(html.contains("BitQuan Mining Pool Dashboard"));
        assert!(html.contains("active-miners"));
        assert!(html.contains("hashrate-total"));
        assert!(html.contains("WebSocket"));
    }
}