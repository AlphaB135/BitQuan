//! BitQuan reference node entrypoint.

use anyhow::Result;
use bitquan_consensus::{ConsensusEngine, ConsensusParams};
use bitquan_storage::InMemoryChainStore;
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => run_node(&config),
        Commands::CheckBlock { path } => check_block(&path),
        Commands::Rng { label, length } => rng_demo(&label, length),
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
    let _engine = ConsensusEngine::new(params, registry)?;
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
    let mut engine = ConsensusEngine::new(params, registry)?;
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
