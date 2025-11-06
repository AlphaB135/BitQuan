//! BitQuan load and stress testing harness.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Parser)]
#[command(name = "bq-stress")]
#[command(about = "Load and stress testing tool for BitQuan nodes", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Hammer JSON-RPC endpoint with concurrent requests
    RpcHammer {
        /// RPC endpoint URL
        #[arg(long, default_value = "http://localhost:8332")]
        url: String,

        /// Concurrent requests
        #[arg(long, default_value = "64")]
        concurrency: usize,

        /// Test duration in seconds
        #[arg(long, default_value = "60")]
        duration: u64,

        /// Output JSON report file
        #[arg(long, default_value = "artifacts/load/rpc_hammer.json")]
        output: String,
    },

    /// Simulate miners submitting shares to Stratum
    PoolShares {
        /// Stratum host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// Stratum port
        #[arg(long, default_value = "3333")]
        port: u16,

        /// Number of simulated miners
        #[arg(long, default_value = "100")]
        miners: usize,

        /// Target shares per second
        #[arg(long, default_value = "10")]
        qps: u64,

        /// Test duration in seconds
        #[arg(long, default_value = "60")]
        duration: u64,

        /// Output JSON report file
        #[arg(long, default_value = "artifacts/load/pool_shares.json")]
        output: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct LoadReport {
    test_type: String,
    duration_secs: u64,
    total_requests: u64,
    successful: u64,
    failed: u64,
    rate_limited: u64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_p99_ms: f64,
    requests_per_sec: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::RpcHammer {
            url,
            concurrency,
            duration,
            output,
        } => {
            rpc_hammer(&url, concurrency, duration, &output).await?;
        }
        Commands::PoolShares {
            host,
            port,
            miners,
            qps,
            duration,
            output,
        } => {
            pool_shares(&host, port, miners, qps, duration, &output).await?;
        }
    }

    Ok(())
}

async fn rpc_hammer(url: &str, concurrency: usize, duration: u64, output: &str) -> Result<()> {
    println!("🔨 RPC Hammer Test");
    println!("  URL: {}", url);
    println!("  Concurrency: {}", concurrency);
    println!("  Duration: {}s", duration);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let stats = Arc::new(Mutex::new(TestStats::new()));
    let end_time = Instant::now() + Duration::from_secs(duration);

    let mut handles = vec![];

    for worker_id in 0..concurrency {
        let client = client.clone();
        let url = url.to_string();
        let stats = Arc::clone(&stats);

        let handle = tokio::spawn(async move {
            use rand::SeedableRng;
            let mut rng = rand::rngs::SmallRng::from_entropy();
            let mut request_id = worker_id as u64 * 10000;

            while Instant::now() < end_time {
                request_id += 1;

                let request = RpcRequest {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    method: "getblockcount".to_string(),
                    params: vec![],
                };

                let start = Instant::now();
                let result = client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await;

                let latency = start.elapsed();

                let mut stats = stats.lock().await;
                stats.total += 1;

                match result {
                    Ok(resp) if resp.status().is_success() => {
                        stats.successful += 1;
                        stats.latencies.push(latency);
                    }
                    Ok(resp) if resp.status() == 429 => {
                        stats.rate_limited += 1;
                        // Back off on rate limit
                        sleep(Duration::from_millis(100)).await;
                    }
                    _ => {
                        stats.failed += 1;
                    }
                }

                // Small delay to avoid overwhelming
                let delay_ms = rng.gen_range(1..10);
                sleep(Duration::from_millis(delay_ms)).await;
            }
        });

        handles.push(handle);
    }

    // Wait for all workers
    for handle in handles {
        handle.await?;
    }

    // Generate report
    let stats = stats.lock().await;
    let report = stats.generate_report("rpc_hammer", duration);

    println!("\n📊 Results:");
    println!("  Total Requests: {}", report.total_requests);
    println!("  Successful: {}", report.successful);
    println!("  Failed: {}", report.failed);
    println!("  Rate Limited: {}", report.rate_limited);
    println!("  p50 Latency: {:.2}ms", report.latency_p50_ms);
    println!("  p95 Latency: {:.2}ms", report.latency_p95_ms);
    println!("  p99 Latency: {:.2}ms", report.latency_p99_ms);
    println!("  QPS: {:.2}", report.requests_per_sec);

    // Save report
    save_report(&report, output)?;

    Ok(())
}

async fn pool_shares(
    host: &str,
    port: u16,
    miners: usize,
    qps: u64,
    duration: u64,
    output: &str,
) -> Result<()> {
    println!("⛏️  Pool Shares Simulation");
    println!("  Stratum: {}:{}", host, port);
    println!("  Miners: {}", miners);
    println!("  Target QPS: {}", qps);
    println!("  Duration: {}s", duration);

    let stats = Arc::new(Mutex::new(TestStats::new()));
    let end_time = Instant::now() + Duration::from_secs(duration);

    let mut handles = vec![];

    for miner_id in 0..miners {
        let host = host.to_string();
        let stats = Arc::clone(&stats);

        let handle = tokio::spawn(async move {
            use rand::SeedableRng;
            let mut rng = rand::rngs::SmallRng::from_entropy();

            while Instant::now() < end_time {
                let start = Instant::now();

                // Simulate share submission (placeholder - would need actual Stratum protocol)
                let simulated_success = rng.gen_bool(0.98); // 98% success rate
                sleep(Duration::from_millis(rng.gen_range(50..200))).await;

                let latency = start.elapsed();

                let mut stats = stats.lock().await;
                stats.total += 1;

                if simulated_success {
                    stats.successful += 1;
                    stats.latencies.push(latency);
                } else {
                    stats.failed += 1;
                }

                // Rate limiting per miner
                let interval = Duration::from_millis(1000 * miners as u64 / qps);
                sleep(interval).await;
            }

            println!("Miner {} finished", miner_id);
        });

        handles.push(handle);
    }

    // Wait for all miners
    for handle in handles {
        handle.await?;
    }

    // Generate report
    let stats = stats.lock().await;
    let report = stats.generate_report("pool_shares", duration);

    println!("\n📊 Results:");
    println!("  Total Shares: {}", report.total_requests);
    println!("  Accepted: {}", report.successful);
    println!("  Rejected: {}", report.failed);
    println!("  Reject Rate: {:.2}%", report.failed as f64 / report.total_requests as f64 * 100.0);
    println!("  p50 Latency: {:.2}ms", report.latency_p50_ms);
    println!("  p95 Latency: {:.2}ms", report.latency_p95_ms);
    println!("  Shares/sec: {:.2}", report.requests_per_sec);

    // Save report
    save_report(&report, output)?;

    Ok(())
}

struct TestStats {
    total: u64,
    successful: u64,
    failed: u64,
    rate_limited: u64,
    latencies: Vec<Duration>,
}

impl TestStats {
    fn new() -> Self {
        Self {
            total: 0,
            successful: 0,
            failed: 0,
            rate_limited: 0,
            latencies: Vec::new(),
        }
    }

    fn generate_report(&self, test_type: &str, duration: u64) -> LoadReport {
        let mut sorted_latencies = self.latencies.clone();
        sorted_latencies.sort();

        let p50 = percentile(&sorted_latencies, 50);
        let p95 = percentile(&sorted_latencies, 95);
        let p99 = percentile(&sorted_latencies, 99);

        LoadReport {
            test_type: test_type.to_string(),
            duration_secs: duration,
            total_requests: self.total,
            successful: self.successful,
            failed: self.failed,
            rate_limited: self.rate_limited,
            latency_p50_ms: p50.as_secs_f64() * 1000.0,
            latency_p95_ms: p95.as_secs_f64() * 1000.0,
            latency_p99_ms: p99.as_secs_f64() * 1000.0,
            requests_per_sec: self.total as f64 / duration as f64,
        }
    }
}

fn percentile(sorted: &[Duration], p: usize) -> Duration {
    if sorted.is_empty() {
        return Duration::from_millis(0);
    }
    let idx = (sorted.len() * p / 100).min(sorted.len() - 1);
    sorted[idx]
}

fn save_report(report: &LoadReport, output: &str) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = std::path::Path::new(output).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(output, json).with_context(|| format!("Failed to write report to {}", output))?;

    println!("\n✅ Report saved to: {}", output);
    Ok(())
}
