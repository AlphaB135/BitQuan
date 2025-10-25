use super::*;

#[test]
fn subsidy_initial_and_halving() {
    let rs = RewardSchedule::phase3_defaults();
    assert_eq!(rs.subsidy_at_height(0), 5_000_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 - 1), 5_000_000_000);
    assert_eq!(rs.subsidy_at_height(210_000), 2_500_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 * 6), 78_125_000);
}

#[test]
fn subsidy_tail_emission_after_seven_halvings() {
    let rs = RewardSchedule::phase3_defaults();
    assert_eq!(rs.subsidy_at_height(210_000 * 7), 50_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 * 1000), 50_000_000);
}

fn mtp(timestamps: &[u64]) -> u64 {
    let mut v = timestamps.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

#[test]
fn difficultystate_with_mtp_anchor_chain() {
    let params = ConsensusParams::phase3_defaults();
    // Simulate 11 previous block timestamps spaced by 600s
    let base: u64 = 1_700_000_000;
    let prev_times: Vec<u64> = (0..11).map(|i| base + i * 600).collect();
    let anchor_height = 1000;
    let anchor_bits = 0x1d00ffff; // classic initial target
    let mut state = DifficultyState::new(anchor_height, prev_times[10], anchor_bits);

    // Next block time uses MTP of the previous 11
    let next_time = mtp(&prev_times);
    let next_bits = state.update(anchor_height + 1, next_time, &params);
    assert!(next_bits > 0);
}

#[test]
fn difficultystate_with_chainstore_mtp_anchor() {
    use bitquan_storage::{ChainStore, InMemoryChainStore};
    use bitquan_types::{Block, BlockHeader, Transaction};

    let params = ConsensusParams::phase3_defaults();
    let mut store = InMemoryChainStore::new();
    let base: u64 = 1_700_100_000;
    let bits: u32 = 0x1d00ffff;

    // Build a chain of 11 headers
    let mut headers: Vec<BlockHeader> = Vec::new();
    for i in 0..11u64 {
        let header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: (base + i * params.target_block_time) as u32,
            bits,
            nonce: 0,
        };
        headers.push(header.clone());
        store.insert_block(Block { header, transactions: Vec::<Transaction>::new() });
    }

    let tip = store.tip().expect("tip").clone();
    let anchor_height = 10; // zero-based index of tip in this test chain
    let mut state = DifficultyState::new(anchor_height, tip.time as u64, tip.bits);

    let times: Vec<u64> = headers.iter().map(|h| h.time as u64).collect();
    let next_time = mtp(&times);
    let next_bits = state.update(anchor_height + 1, next_time, &params);
    assert!(next_bits > 0);
}
