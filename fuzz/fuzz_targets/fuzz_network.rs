#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_network::protocol::{MessageEnvelope, MessageType};
use bitquan_types::NetworkId;

fuzz_target!(|data: &[u8]| {
    // Fuzz network message envelope deserialization
    if !data.is_empty() && data.len() <= 10_000_000 {
        // Test deserialization doesn't panic
        let _ = MessageEnvelope::deserialize(data, NetworkId::Devnet);
        
        // Test with different network IDs
        let networks = [NetworkId::Mainnet, NetworkId::Testnet, NetworkId::Devnet];
        for network in networks.iter() {
            let _ = MessageEnvelope::deserialize(data, *network);
        }
    }
    
    // Fuzz message type parsing
    if data.len() >= 1 {
        let msg_type = data[0];
        // Test all possible message types
        let _ = MessageType::from_u8(msg_type);
    }
    
    // Fuzz oversized messages (DoS protection)
    if data.len() > 10_000_000 {
        // Should handle oversized messages gracefully
        let _ = MessageEnvelope::deserialize(&data[..10_000_000.min(data.len())], NetworkId::Devnet);
    }
    
    // Fuzz malformed JSON payloads
    if let Ok(json_str) = std::str::from_utf8(data) {
        if json_str.len() <= 1_000_000 {
            // Test JSON parsing doesn't panic
            let _ = serde_json::from_str::<serde_json::Value>(json_str);
        }
    }
});