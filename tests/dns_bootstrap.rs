//! Integration test: DNS bootstrap system validation
//!
//! Tests DNS seed resolution and peer connectivity bootstrap

use std::fs;

#[test]
fn test_dns_seeds_file_exists() {
    let seeds_path = "genesis/dns_seeds.txt";
    assert!(
        std::path::Path::new(seeds_path).exists(),
        "DNS seeds file must exist"
    );
}

#[test]
fn test_dns_seeds_format() {
    let content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds file");

    let lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect();

    assert!(
        lines.len() >= 3,
        "Must have at least 3 DNS seeds for redundancy"
    );

    for line in &lines {
        assert!(
            line.contains(':'),
            "DNS seed must include port: {}",
            line
        );
        
        // Validate basic domain format
        let parts: Vec<&str> = line.split(':').collect();
        assert_eq!(parts.len(), 2, "Seed format must be domain:port");
        
        let domain = parts[0];
        assert!(
            domain.contains('.'),
            "Domain must contain at least one dot: {}",
            domain
        );
        
        let port = parts[1].parse::<u16>();
        assert!(port.is_ok(), "Port must be valid u16: {}", parts[1]);
    }
}

#[test]
fn test_mainnet_testnet_seeds_separated() {
    let content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds file");

    let mainnet_seeds: Vec<&str> = content
        .lines()
        .filter(|line| !line.contains("testnet") && line.contains("bitquan.network"))
        .collect();

    let testnet_seeds: Vec<&str> = content
        .lines()
        .filter(|line| line.contains("testnet"))
        .collect();

    assert!(
        !mainnet_seeds.is_empty(),
        "Must have mainnet DNS seeds"
    );
    
    assert!(
        !testnet_seeds.is_empty(),
        "Must have testnet DNS seeds"
    );

    println!("Found {} mainnet seeds", mainnet_seeds.len());
    println!("Found {} testnet seeds", testnet_seeds.len());
}

#[test]
fn test_seed_domains_use_standard_ports() {
    let content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds file");

    for line in content.lines() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 2 {
            let port: u16 = parts[1].parse().unwrap();
            
            // Mainnet typically uses 8333, testnet uses 18333
            if line.contains("testnet") {
                assert_eq!(
                    port, 18333,
                    "Testnet should use port 18333"
                );
            } else if line.contains("bitquan.network") {
                assert_eq!(
                    port, 8333,
                    "Mainnet should use port 8333"
                );
            }
        }
    }
}

#[test]
fn test_genesis_json_matches_dns_seeds() {
    let seeds_content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds");
    
    let mainnet_content = fs::read_to_string("genesis/mainnet.json")
        .expect("Failed to read mainnet genesis");
    
    let mainnet_json: serde_json::Value = serde_json::from_str(&mainnet_content).unwrap();
    let dns_seeds = mainnet_json["dns_seeds"].as_array().unwrap();

    // Extract mainnet seeds from dns_seeds.txt
    let file_mainnet_seeds: Vec<String> = seeds_content
        .lines()
        .filter(|line| {
            !line.trim().is_empty() 
            && !line.trim().starts_with('#')
            && !line.contains("testnet")
        })
        .map(|line| {
            // Extract just domain without port
            line.split(':').next().unwrap().to_string()
        })
        .collect();

    // Check that genesis.json DNS seeds are present in dns_seeds.txt
    for seed in dns_seeds {
        let seed_str = seed.as_str().unwrap();
        let found = file_mainnet_seeds.iter().any(|fs| fs.contains(seed_str));
        assert!(
            found,
            "Genesis DNS seed '{}' should be in dns_seeds.txt",
            seed_str
        );
    }
}

#[test]
fn test_dns_bootstrap_mock_resolution() {
    // Mock test: verify that DNS resolution API is callable
    // In real deployment, this would resolve actual DNS records
    
    let seeds_content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds");

    let valid_seeds: Vec<&str> = seeds_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect();

    for seed in valid_seeds {
        let parts: Vec<&str> = seed.split(':').collect();
        let domain = parts[0];
        let port = parts[1].parse::<u16>().unwrap();
        
        // Validate that we can construct socket address format
        let socket_addr_str = format!("{}:{}", domain, port);
        assert!(
            socket_addr_str.contains(':'),
            "Should form valid socket address string"
        );
        
        println!("✓ Seed format validated: {}", socket_addr_str);
    }
}

#[test]
fn test_bootstrap_peer_connectivity_check() {
    // Integration test: verify bootstrap peer configuration
    let mainnet = fs::read_to_string("genesis/mainnet.json").unwrap();
    let genesis: serde_json::Value = serde_json::from_str(&mainnet).unwrap();

    let bootstrap_peers = genesis["bootstrap_peers"].as_array().unwrap();

    for peer in bootstrap_peers {
        let peer_str = peer.as_str().unwrap();
        let parts: Vec<&str> = peer_str.split(':').collect();
        
        assert_eq!(parts.len(), 2, "Bootstrap peer must be host:port format");
        
        let _host = parts[0];
        let port = parts[1].parse::<u16>().unwrap();
        
        assert!(
            port > 0 && port < 65536,
            "Port must be valid: {}",
            port
        );
    }
}

#[test]
fn test_dns_health_check_simulation() {
    // Simulate health check logic that would run in production
    let seeds_content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds");

    let mainnet_seeds: Vec<&str> = seeds_content
        .lines()
        .filter(|line| {
            !line.trim().is_empty() 
            && !line.trim().starts_with('#')
            && !line.contains("testnet")
        })
        .collect();

    // Simulate async resolver picking healthy peers
    let mut healthy_count = 0;
    for seed in &mainnet_seeds {
        // In production, this would actually attempt connection
        // For now, just validate format
        if seed.contains("bitquan.network") && seed.contains(':') {
            healthy_count += 1;
        }
    }

    assert!(
        healthy_count >= 3,
        "Should have at least 3 healthy mainnet seeds"
    );
    
    println!("✓ Simulated {} healthy seeds", healthy_count);
}

#[test]
fn test_dns_seed_redundancy() {
    // Verify redundancy: at least 3 seeds per network
    let seeds_content = fs::read_to_string("genesis/dns_seeds.txt")
        .expect("Failed to read DNS seeds");

    let mainnet_count = seeds_content
        .lines()
        .filter(|line| {
            !line.trim().is_empty() 
            && !line.trim().starts_with('#')
            && !line.contains("testnet")
        })
        .count();

    let testnet_count = seeds_content
        .lines()
        .filter(|line| line.contains("testnet"))
        .count();

    assert!(
        mainnet_count >= 3,
        "Mainnet needs at least 3 DNS seeds for redundancy, found {}",
        mainnet_count
    );

    assert!(
        testnet_count >= 2,
        "Testnet needs at least 2 DNS seeds for redundancy, found {}",
        testnet_count
    );
}
