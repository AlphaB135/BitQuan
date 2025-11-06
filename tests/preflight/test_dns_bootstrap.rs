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
    let seeds_path = "genesis/dns_seeds.txt";
    let content = fs::read_to_string(seeds_path)
        .expect("Failed to read DNS seeds file");
    
    let seeds: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    
    assert!(
        seeds.len() > 0,
        "At least one DNS seed must be configured"
    );
    
    for seed in seeds {
        // Verify format: domain:port
        let parts: Vec<&str> = seed.split(':').collect();
        assert_eq!(
            parts.len(),
            2,
            "DNS seed must be in format domain:port, got: {}",
            seed
        );
        
        let domain = parts[0];
        let port = parts[1];
        
        assert!(
            !domain.is_empty(),
            "Domain must not be empty"
        );
        
        assert!(
            port.parse::<u16>().is_ok(),
            "Port must be valid u16, got: {}",
            port
        );
    }
}

#[test]
fn test_dns_bootstrap_min_threshold_mock() {
    // Mock test: verify threshold logic
    let total = 5;
    let reachable = 3;
    let threshold = 60;
    
    let percentage = (reachable * 100) / total;
    
    assert!(
        percentage >= threshold,
        "Mock: {}% should meet {}% threshold",
        percentage,
        threshold
    );
}

#[test]
fn test_mainnet_seeds_present() {
    let seeds_path = "genesis/dns_seeds.txt";
    let content = fs::read_to_string(seeds_path)
        .expect("Failed to read DNS seeds file");
    
    let mainnet_seeds: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter(|line| line.contains("seed") && !line.contains("testnet"))
        .collect();
    
    assert!(
        mainnet_seeds.len() >= 3,
        "At least 3 mainnet seeds required for redundancy"
    );
}

#[test]
fn test_testnet_seeds_present() {
    let seeds_path = "genesis/dns_seeds.txt";
    let content = fs::read_to_string(seeds_path)
        .expect("Failed to read DNS seeds file");
    
    let testnet_seeds: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter(|line| line.contains("testnet"))
        .collect();
    
    assert!(
        testnet_seeds.len() >= 2,
        "At least 2 testnet seeds required"
    );
}
