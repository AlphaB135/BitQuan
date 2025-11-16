#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_network::protocol::{MessageEnvelope, Message};

fuzz_target!(|data: &[u8]| {
    // Fuzz network message envelope deserialization
    if !data.is_empty() && data.len() <= 10_000_000 {
        // Test deserialization doesn't panic
        let _ = MessageEnvelope::deserialize(data);
        
        // Test creating envelope from message
        let _ = MessageEnvelope::new(Message::VerAck);
    }
    
    // Fuzz message creation
    if data.len() >= 8 {
        let nonce = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]
        ]);
        // Test message creation doesn't panic
        let _ = Message::Ping { nonce };
        let _ = Message::Pong { nonce };
    }
    
    // Fuzz oversized messages (DoS protection)
    if data.len() > 10_000_000 {
        // Should handle oversized messages gracefully
        let _ = MessageEnvelope::deserialize(&data[..10_000_000.min(data.len())]);
    }
    
    // Fuzz malformed JSON payloads
    if let Ok(json_str) = std::str::from_utf8(data) {
        if json_str.len() <= 1_000_000 {
            // Test JSON parsing doesn't panic
            let _ = serde_json::from_str::<serde_json::Value>(json_str);
        }
    }
});