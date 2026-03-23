//! Tests for memory exhaustion protection

use bitquan_network::protocol::{validate_message, InvType, InvVector, Message};
use bitquan_network::protocol::{MAX_ADDR_ENTRIES, MAX_HEADERS, MAX_INV_ITEMS};
use bitquan_types::BlockHeader;

#[test]
fn test_reject_excessive_inv_items() {
    let excessive_inv: Vec<InvVector> = (0..MAX_INV_ITEMS + 1)
        .map(|i| InvVector {
            inv_type: InvType::Block,
            hash: [i as u8; 32],
        })
        .collect();

    let msg = Message::Inv {
        inventory: excessive_inv,
    };

    let result = validate_message(&msg);
    assert!(result.is_err(), "Should reject excessive inv items");
}

#[test]
fn test_accept_normal_inv_items() {
    let normal_inv: Vec<InvVector> = (0..100)
        .map(|i| InvVector {
            inv_type: InvType::Block,
            hash: [i as u8; 32],
        })
        .collect();

    let msg = Message::Inv {
        inventory: normal_inv,
    };

    let result = validate_message(&msg);
    assert!(result.is_ok(), "Should accept normal inv items");
}

#[test]
fn test_reject_excessive_headers() {
    let excessive_headers: Vec<BlockHeader> = (0..MAX_HEADERS + 1)
        .map(|i| BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
           uncles_hash: [0u8; 32],
            time: i as u32,
            bits: 0x1d00ffff,
            nonce: 0,
            algo_id: 0,
        })
        .collect();

    let msg = Message::Headers {
        headers: excessive_headers,
    };

    let result = validate_message(&msg);
    assert!(result.is_err(), "Should reject excessive headers");
}

#[test]
fn test_reject_excessive_addresses() {
    let excessive_addrs = (0..MAX_ADDR_ENTRIES + 1)
        .map(|i| bitquan_network::protocol::PeerAddr {
            timestamp: i as u64,
            services: 1,
            ip: "127.0.0.1".to_string(),
            port: 8333,
        })
        .collect();

    let msg = Message::Addr {
        addrs: excessive_addrs,
    };

    let result = validate_message(&msg);
    assert!(result.is_err(), "Should reject excessive addresses");
}
