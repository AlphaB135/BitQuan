use bitquan_consensus::fork::{header_hash, ForkChoice};
use bitquan_types::block::BlockHeader;

fn make_header(prev: [u8; 32], bits: u32, timestamp: u64) -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_block: prev,
        merkle_root: [0u8; 32],
        timestamp,
        bits,
        nonce: 0,
    }
}

#[test]
fn debug_reorg() {
    let mut fc = ForkChoice::new();

    // Genesis
    let genesis = make_header([0u8; 32], 0x207fffff, 0);
    fc.add_genesis(genesis.clone()).unwrap();
    let genesis_hash = header_hash(&genesis);

    // Add A1
    let a1 = make_header(genesis_hash, 0x207fffff, 1);
    let (is_tip_a1, reorg_a1) = fc.add_block(a1.clone()).unwrap();
    println!(
        "After A1: is_tip={}, reorg={:?}, height={}",
        is_tip_a1,
        reorg_a1.is_some(),
        fc.height()
    );

    // Add A2
    let a1_hash = header_hash(&a1);
    let a2 = make_header(a1_hash, 0x207fffff, 2);
    let (is_tip_a2, reorg_a2) = fc.add_block(a2).unwrap();
    println!(
        "After A2: is_tip={}, reorg={:?}, height={}",
        is_tip_a2,
        reorg_a2.is_some(),
        fc.height()
    );

    // Add B1
    let b1 = make_header(genesis_hash, 0x207fffff, 10);
    let (is_tip_b1, reorg_b1) = fc.add_block(b1.clone()).unwrap();
    println!(
        "After B1: is_tip={}, reorg={:?}, height={}",
        is_tip_b1,
        reorg_b1.is_some(),
        fc.height()
    );

    // Add B2
    let b1_hash = header_hash(&b1);
    let b2 = make_header(b1_hash, 0x207fffff, 11);
    let (is_tip_b2, reorg_b2) = fc.add_block(b2.clone()).unwrap();
    println!(
        "After B2: is_tip={}, reorg={:?}, height={}",
        is_tip_b2,
        reorg_b2.is_some(),
        fc.height()
    );

    // Add B3
    let b2_hash = header_hash(&b2);
    let b3 = make_header(b2_hash, 0x207fffff, 12);
    let (is_tip_b3, reorg_b3) = fc.add_block(b3).unwrap();
    println!(
        "After B3: is_tip={}, reorg={:?}, height={}",
        is_tip_b3,
        reorg_b3.is_some(),
        fc.height()
    );

    assert!(is_tip_b3, "B3 should become new tip");
    assert!(reorg_b3.is_some(), "B3 should trigger reorg");
}
