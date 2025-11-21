#![no_main]

use libfuzzer_sys::fuzz_target;
use bq_sdk::address::{Address, Network};

fuzz_target!(|data: &[u8]| {
    // Fuzz address parsing
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = Address::parse(s);
    }

    // Fuzz address creation
    if data.len() >= 20 {
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&data[0..20]);
        
        if let Ok(addr) = Address::p2pkh(Network::Mainnet, &hash) {
            let encoded = addr.to_string();
            if let Ok(parsed) = Address::parse(&encoded) {
                assert_eq!(addr, parsed);
            }
        }
    }
});
