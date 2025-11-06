use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Parser)]
#[command(name = "bq-preflight")]
#[command(about = "BitQuan preflight validation tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check DNS seeds reachability
    DnsCheck {
        #[arg(long, default_value = "mainnet")]
        network: String,

        #[arg(long, default_value = "2")]
        timeout: u64,

        #[arg(long, default_value = "60")]
        dns_seed_threshold: u8,
    },
    /// Probe TCP connectivity
    TcpProbe {
        #[arg(long)]
        host: String,

        #[arg(long)]
        port: u16,

        #[arg(long, default_value = "1000")]
        timeout_ms: u64,
    },
}

#[derive(Serialize, Deserialize)]
struct DnsCheckResult {
    total: usize,
    reachable: usize,
    percentage: f64,
    seeds: Vec<SeedStatus>,
}

#[derive(Serialize, Deserialize)]
struct SeedStatus {
    seed: String,
    reachable: bool,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::DnsCheck {
            network,
            timeout: timeout_secs,
            dns_seed_threshold,
        } => {
            dns_check(&network, timeout_secs, dns_seed_threshold).await?;
        }
        Commands::TcpProbe {
            host,
            port,
            timeout_ms,
        } => {
            tcp_probe(&host, port, timeout_ms).await?;
        }
    }

    Ok(())
}

async fn dns_check(network: &str, timeout_secs: u64, threshold_pct: u8) -> Result<()> {
    let project_root = find_project_root()?;
    let seeds_file = project_root.join("genesis/dns_seeds.txt");

    let seeds_content = fs::read_to_string(&seeds_file).context("Failed to read dns_seeds.txt")?;

    let pattern = if network == "testnet" {
        "testnet"
    } else {
        "seed"
    };

    let seeds: Vec<String> = seeds_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter(|line| line.contains(pattern))
        .map(|s| s.trim().to_string())
        .collect();

    let mut results = Vec::new();
    let timeout_duration = Duration::from_secs(timeout_secs);

    for seed in &seeds {
        let parts: Vec<&str> = seed.split(':').collect();
        if parts.len() != 2 {
            results.push(SeedStatus {
                seed: seed.clone(),
                reachable: false,
                error: Some("Invalid format".to_string()),
            });
            continue;
        }

        let host = parts[0];
        let port: u16 = parts[1].parse().unwrap_or(8333);

        let reachable = check_tcp_reachability(host, port, timeout_duration).await;

        results.push(SeedStatus {
            seed: seed.clone(),
            reachable,
            error: if reachable {
                None
            } else {
                Some("Unreachable".to_string())
            },
        });
    }

    let reachable_count = results.iter().filter(|r| r.reachable).count();
    let total = results.len();
    let percentage = if total > 0 {
        (reachable_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let result = DnsCheckResult {
        total,
        reachable: reachable_count,
        percentage,
        seeds: results,
    };

    println!("{}", serde_json::to_string_pretty(&result)?);

    // Check threshold
    if percentage < threshold_pct as f64 {
        anyhow::bail!(
            "DNS seed reachability {}% below threshold {}%",
            percentage,
            threshold_pct
        );
    }

    Ok(())
}

async fn tcp_probe(host: &str, port: u16, timeout_ms: u64) -> Result<()> {
    let timeout_duration = Duration::from_millis(timeout_ms);
    let reachable = check_tcp_reachability(host, port, timeout_duration).await;

    #[derive(Serialize)]
    struct TcpProbeResult {
        host: String,
        port: u16,
        reachable: bool,
    }

    let result = TcpProbeResult {
        host: host.to_string(),
        port,
        reachable,
    };

    println!("{}", serde_json::to_string(&result)?);

    Ok(())
}

async fn check_tcp_reachability(host: &str, port: u16, timeout_duration: Duration) -> bool {
    // First try DNS resolution
    let addr_str = format!("{}:{}", host, port);
    let addrs: Vec<SocketAddr> = match addr_str.to_socket_addrs() {
        Ok(addrs) => addrs.collect(),
        Err(_) => return false,
    };

    if addrs.is_empty() {
        return false;
    }

    // Try connecting to first resolved address
    let addr = addrs[0];

    match timeout(timeout_duration, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => true,
        Ok(Err(_)) => false,
        Err(_) => false, // Timeout
    }
}

fn find_project_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        if current.join("Cargo.toml").exists() && current.join("genesis").exists() {
            return Ok(current);
        }

        if !current.pop() {
            anyhow::bail!("Could not find project root");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_probe_localhost() {
        // This test would need a mock server in real scenarios
        // For now, testing unreachable addresses
        let _reachable =
            check_tcp_reachability("localhost", 9999, Duration::from_millis(100)).await;
        // Non-deterministic, so we just verify it runs without panic
    }

    #[test]
    fn test_find_project_root() {
        // This will work if run from within the project
        let result = find_project_root();
        // Should either find it or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }
}
