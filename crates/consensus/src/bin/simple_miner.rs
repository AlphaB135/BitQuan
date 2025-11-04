//! Simple CPU miner for testing BitQuan PoW

use std::time::Instant;

use bitquan_consensus::{check_header_pow, compact_to_target_bytes};
use bitquan_types::BlockHeader;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "simple-miner", about = "Simple CPU miner for BitQuan")]
struct MinerArgs {
    /// Target difficulty (compact bits format, e.g., 0x1d00ffff)
    #[arg(long, default_value = "0x207fffff")]
    bits: String,

    /// Number of blocks to mine
    #[arg(long, default_value = "1")]
    blocks: u64,

    /// Print verbose mining info
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn parse_hex_u32(s: &str) -> Result<u32, String> {
    let s = s.trim_start_matches("0x");
    u32::from_str_radix(s, 16).map_err(|e| format!("invalid hex: {}", e))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = MinerArgs::parse();
    let bits = parse_hex_u32(&args.bits)?;

    println!("🪙 BitQuan Simple Miner");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Target bits: {:#010x}", bits);

    let target = compact_to_target_bytes(bits)?;
    println!(
        "Target (hex): {}",
        hex::encode(&target[..8]) + "..." + &hex::encode(&target[24..])
    );

    for block_num in 1..=args.blocks {
        println!("\n⛏️  Mining block {}/{}", block_num, args.blocks);

        let mut header = BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as u32,
            bits,
            nonce: 0,
        };

        let start = Instant::now();
        let mut hashes = 0u64;

        loop {
            if check_header_pow(&header)? {
                let elapsed = start.elapsed();
                let hashrate = hashes as f64 / elapsed.as_secs_f64();

                println!("✅ Block mined!");
                println!("   Nonce: {}", header.nonce);
                println!("   Hashes: {}", hashes);
                println!("   Time: {:.2}s", elapsed.as_secs_f64());
                println!("   Hash rate: {:.2} H/s", hashrate);

                let hash = bitquan_consensus::pow::header_hash(&header);
                println!("   Block hash: {}", hex::encode(&hash[..16]) + "...");
                break;
            }

            header.nonce += 1;
            hashes += 1;

            if args.verbose && hashes % 10000 == 0 {
                let elapsed = start.elapsed().as_secs_f64();
                let hashrate = hashes as f64 / elapsed;
                println!("   [{:.2}s] {} hashes, {:.2} H/s", elapsed, hashes, hashrate);
            }

            if hashes % 1_000_000 == 0 {
                let elapsed = start.elapsed().as_secs_f64();
                let hashrate = hashes as f64 / elapsed;
                print!("   ⚡ {:.1}M hashes, {:.2} H/s\r", hashes as f64 / 1_000_000.0, hashrate);
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Mining complete!");

    Ok(())
}
