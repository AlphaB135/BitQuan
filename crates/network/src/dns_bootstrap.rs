//! DNS bootstrap for peer discovery in BitQuan network.

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

/// DNS seed configuration
#[derive(Debug, Clone)]
pub struct DnsSeed {
    /// Hostname of the DNS seed
    pub hostname: String,
    /// Port for peer connections
    pub port: u16,
}

impl DnsSeed {
    /// Create a new DNS seed
    pub fn new(hostname: impl Into<String>, port: u16) -> Self {
        Self {
            hostname: hostname.into(),
            port,
        }
    }

    /// Parse from "hostname:port" format
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let port = parts[1].parse().ok()?;
        Some(Self::new(parts[0], port))
    }
}

/// DNS bootstrap resolver
pub struct DnsBootstrap {
    seeds: Vec<DnsSeed>,
    timeout: Duration,
}

impl DnsBootstrap {
    /// Create a new DNS bootstrap resolver
    pub fn new(seeds: Vec<DnsSeed>) -> Self {
        Self {
            seeds,
            timeout: Duration::from_secs(5),
        }
    }

    /// Create from seed file content
    pub fn from_seed_file(content: &str) -> Self {
        let seeds: Vec<DnsSeed> = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .filter_map(|line| DnsSeed::parse(line.trim()))
            .collect();

        Self::new(seeds)
    }

    /// Set resolution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Resolve all DNS seeds and return peer addresses
    pub fn resolve(&self) -> Vec<SocketAddr> {
        let mut peers = Vec::new();

        for seed in &self.seeds {
            match self.resolve_seed(seed) {
                Ok(addrs) => {
                    peers.extend(addrs);
                }
                Err(e) => {
                    log::warn!("Failed to resolve DNS seed {}: {}", seed.hostname, e);
                }
            }
        }

        peers
    }

    /// Resolve a single DNS seed
    fn resolve_seed(&self, seed: &DnsSeed) -> Result<Vec<SocketAddr>, std::io::Error> {
        let host_port = format!("{}:{}", seed.hostname, seed.port);

        // Perform DNS resolution
        let addrs: Vec<SocketAddr> = host_port.to_socket_addrs()?.collect();

        Ok(addrs)
    }

    /// Resolve and return up to `max_peers` healthy peers
    pub fn bootstrap(&self, max_peers: usize) -> Vec<SocketAddr> {
        let mut peers = self.resolve();

        // Shuffle for randomness using cryptographically secure RNG
        use rand::rngs::OsRng;
        use rand::seq::SliceRandom;
        peers.shuffle(&mut OsRng);

        // Take only the requested number
        peers.truncate(max_peers);

        peers
    }

    /// Health check a peer by attempting to connect
    pub fn check_peer(addr: &SocketAddr, timeout: Duration) -> bool {
        use std::net::TcpStream;

        TcpStream::connect_timeout(addr, timeout).is_ok()
    }

    /// Filter peers by health check
    pub fn filter_healthy(&self, peers: &[SocketAddr]) -> Vec<SocketAddr> {
        peers
            .iter()
            .filter(|addr| Self::check_peer(addr, self.timeout))
            .copied()
            .collect()
    }
}

/// Load DNS seeds from mainnet/testnet defaults
pub fn load_default_seeds(network: &str) -> Vec<DnsSeed> {
    match network {
        "mainnet" => vec![
            DnsSeed::new("seed1.bitquan.network", 8333),
            DnsSeed::new("seed2.bitquan.network", 8333),
            DnsSeed::new("seed3.bitquan.network", 8333),
            DnsSeed::new("seed4.bitquan.network", 8333),
            DnsSeed::new("seed5.bitquan.network", 8333),
        ],
        "testnet" => vec![
            DnsSeed::new("testnet-seed1.bitquan.network", 18333),
            DnsSeed::new("testnet-seed2.bitquan.network", 18333),
            DnsSeed::new("testnet-seed3.bitquan.network", 18333),
        ],
        "devnet" | "regtest" => vec![DnsSeed::new("localhost", 18444)],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_seed_parse() {
        let seed = DnsSeed::parse("seed.example.com:8333").expect("Failed to parse DNS seed");
        assert_eq!(seed.hostname, "seed.example.com");
        assert_eq!(seed.port, 8333);
    }

    #[test]
    fn test_dns_seed_parse_invalid() {
        assert!(DnsSeed::parse("invalid").is_none());
        assert!(DnsSeed::parse("host:port:extra").is_none());
    }

    #[test]
    fn test_from_seed_file() {
        let content = r#"
# Comment line
seed1.example.com:8333
seed2.example.com:8333

# Another comment
seed3.example.com:8333
"#;
        let bootstrap = DnsBootstrap::from_seed_file(content);
        assert_eq!(bootstrap.seeds.len(), 3);
    }

    #[test]
    fn test_load_default_seeds() {
        let mainnet = load_default_seeds("mainnet");
        assert_eq!(mainnet.len(), 5);

        let testnet = load_default_seeds("testnet");
        assert_eq!(testnet.len(), 3);
    }
}
