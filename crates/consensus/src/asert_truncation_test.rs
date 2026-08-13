use bitquan_consensus::pow::DEVNET_MAX_BITS;
use bitquan_consensus::{asert_next_target, ConsensusParams, compact_to_target, target_to_compact};

fn main() {
    let params = ConsensusParams::devnet_hybrid();
    let max_target = compact_to_target(DEVNET_MAX_BITS);
    
    let next = asert_next_target(max_target, 1, 120, &params, None);
    let next_bits = target_to_compact(&next);
    
    println!("Max bits : {:#010x}", DEVNET_MAX_BITS);
    println!("Next bits: {:#010x}", next_bits);
}
