//! BitQuan reference node entrypoint.

use anyhow::Result;
use bitquan_consensus::{check_header_pow, header_hash, ConsensusEngine, ConsensusParams, DifficultyState};
use bitquan_storage::{ChainStore, InMemoryChainStore};
use bitquan_types::Block;
use bq_crypto::{
    rng::{RandomSource, RngService},
    CryptoRegistry,
};
use clap::{Parser, Subcommand};
use hex::encode as hex_encode;

#[derive(Parser)]
#[command(
    name = "bitquan-node",
    version,
    about = "BitQuan reference node (prototype)",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a placeholder node loop.
    Run {
        /// Path to the node configuration file.
        #[arg(long, default_value = "config/bitquan.toml")]
        config: String,
    },
    /// Validates a block provided via an external source (placeholder).
    CheckBlock {
        /// Path to a serialized block file.
        #[arg(long)]
        path: String,
    },
    /// Generates random bytes and derived streams using the BitQuan RNG.
    Rng {
        /// Domain separation label for the derived stream.
        #[arg(long, default_value = "wallet-seed")]
        label: String,
        /// Number of bytes to emit for the sample payloads.
        #[arg(long, default_value_t = 32)]
        length: usize,
    },
    /// Mines a single block template by iterating nonces up to a limit (demo CPU miner).
    MineOnce {
        /// Maximum nonce attempts to try.
        #[arg(long, default_value_t = 1_000_000u64)]
        max_tries: u64,
        /// Hex-encoded script_pubkey for coinbase payout.
        #[arg(long, default_value = "76a9140088ac")]
        payout_script_hex: String,
        /// Compact bits target (e.g., 0x207fffff for very easy target).
        #[arg(long, default_value_t = 0x207fffff)]
        bits: u32,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => run_node(&config),
        Commands::CheckBlock { path } => check_block(&path),
        Commands::Rng { label, length } => rng_demo(&label, length),
        Commands::MineOnce { max_tries, payout_script_hex, bits } => mine_once(max_tries, &payout_script_hex, bits),
    }
}

fn run_node(config_path: &str) -> Result<()> {
    println!(
        "Starting BitQuan node with configuration: {config_path}\n\
         Networking, consensus, and storage subsystems are not yet implemented."
    );

    // Bootstraps placeholder subsystems to illustrate crate integration.
    let registry = CryptoRegistry::default();
    let params = ConsensusParams::phase3_defaults();
    let _engine = ConsensusEngine::new(params, registry);
    let _storage = InMemoryChainStore::new();

    Ok(())
}

fn check_block(path: &str) -> Result<()> {
    println!(
        "Block validation placeholder invoked for file: {path}. \
         Actual parsing logic will be implemented in Phase 4."
    );

    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::default();
    let mut engine = ConsensusEngine::new(params, registry);
    let block = load_block_placeholder()?;

    match engine.validate_block(&block, 0) {
        Ok(report) => {
            println!(
                "Block validated successfully. weight={}, signatures={}, subsidy={}",
                report.block_weight, report.signature_count, report.block_subsidy
            );
        }
        Err(err) => {
            println!("Block validation failed: {err}");
        }
    }

    Ok(())
}

fn rng_demo(label: &str, length: usize) -> Result<()> {
    if length == 0 {
        println!("Length must be greater than zero.");
        return Ok(());
    }

    let mut master = RngService::new()?;
    let mut derived = master.derive_stream(label);

    let master_bytes = master.bytes(length)?;
    let derived_bytes = derived.bytes(length)?;

    println!(
        "Master stream sample  ({length} bytes): {}",
        hex_encode(master_bytes)
    );
    println!(
        "Derived stream `{label}` ({length} bytes): {}",
        hex_encode(derived_bytes)
    );

    Ok(())
}

fn load_block_placeholder() -> Result<Block> {
    // TODO: Replace with real parsing from disk
    let block = Block {
        header: bitquan_types::BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 0,
            bits: 0,
            nonce: 0,
        },
        transactions: Vec::new(),
    };
    Ok(block)
}

fn mine_once(max_tries: u64, payout_script_hex: &str, mut bits: u32) -> Result<()> {
    use bitquan_types::{Block, BlockHeader, Transaction, TxOut, SigAlgorithm};
    let mut store = InMemoryChainStore::new();

    // Determine timestamp using MTP/tip first
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;
    let mut time = now;
    if let Some(mtp) = store.mtp() {
        time = time.max(mtp.saturating_add(1));
    } else if let Some(tip) = store.tip() {
        time = time.max(tip.time.saturating_add(1));
    }

    // Build coinbase (placeholder coinbase input and payout output)
    let payout_script = hex::decode(payout_script_hex)?;
    // Construct coinbase input: prev=00..00:vout=0xffffffff, sequence=0xffffffff, script_sig=[height_le|extranonce]
    let height_le = (store.height() as u32 + 1).to_le_bytes();
    let mut script_sig = height_le.to_vec();
    script_sig.extend_from_slice(&time.to_le_bytes()); // simple extranonce = time
    let coinbase_in = bitquan_types::TxIn {
        prev_txid: [0u8; 32],
        prev_vout: u32::MAX,
        sequence: u32::MAX,
        script_sig,
    };
    let subsidy = bitquan_consensus::ConsensusParams::phase3_defaults().reward_schedule.subsidy_at_height(store.height());
    let coinbase = Transaction {
        version: 2,
        lock_time: 0,
        inputs: vec![coinbase_in],
        outputs: vec![TxOut { value: subsidy, script_pubkey: payout_script }],
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![],
    };

    // Merkle root for block (support multi-tx in future)
    let merkle_root = bitquan_types::compute_merkle_root_from_txids(&[coinbase.txid()]);

    // Determine prev_block from tip if any
    let mut prev = [0u8; 32];
    if let Some(tip) = store.tip() {
        prev = header_hash(tip);
    }

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;
    let mut time = now;
    if let Some(mtp) = store.mtp() {
        time = time.max(mtp.saturating_add(1));
    } else if let Some(tip) = store.tip() {
        time = time.max(tip.time.saturating_add(1));
    }

    // Auto-calc bits if zero using DifficultyState anchored at tip
    if bits == 0 {
        let params = ConsensusParams::phase3_defaults();
        let (anchor_bits, anchor_time) = if let Some(tip) = store.tip() { (tip.bits, tip.time as u64) } else { (0x207fffff, now as u64) };
        let mut state = DifficultyState::new(0, anchor_time, anchor_bits);
        bits = state.update(1, time as u64, &params);
    }

    let mut header = BlockHeader {
        version: 1,
        prev_block: prev,
        merkle_root,
        pqc_agg_hint: [0u8; 32],
        time,
        bits,
        nonce: 0,
    };

    for n in 0..max_tries {
        header.nonce = n;
        if check_header_pow(&header) {
            let id = header_hash(&header);
            println!("FOUND nonce={n} hash={}", hex::encode(id));
            let block = Block { header: header.clone(), transactions: vec![coinbase] };
            store.insert_block(block);
            println!("Inserted block tip={}", hex::encode(id));
            return Ok(());
        }
        if n % 100_000 == 0 {
            let h = header_hash(&header);
            println!("... tried {n} nonces, latest hash={} ", hex::encode(h));
        }
    }
    println!("No valid nonce found within {max_tries} tries.");
    Ok(())
}
