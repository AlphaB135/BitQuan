//! BitQuan reference node entrypoint.

use anyhow::Result;
use bitquan_consensus::{validate_block, ConsensusParams};
use bitquan_crypto::CryptoRegistry;
use bitquan_storage::InMemoryChainStore;
use bitquan_types::Block;
use clap::{Parser, Subcommand};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => run_node(&config),
        Commands::CheckBlock { path } => check_block(&path),
    }
}

fn run_node(config_path: &str) -> Result<()> {
    println!(
        "Starting BitQuan node with configuration: {config_path}\n\
         Networking, consensus, and storage subsystems are not yet implemented."
    );

    // Bootstraps placeholder subsystems to illustrate crate integration.
    let _crypto = CryptoRegistry::default();
    let _params = ConsensusParams::phase3_defaults();
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
    let block = load_block_placeholder()?;

    match validate_block(&block, &params, &registry) {
        Ok(report) => {
            println!(
                "Block validated successfully. weight={}, signatures={}",
                report.block_weight, report.signature_count
            );
        }
        Err(err) => {
            println!("Block validation failed: {err}");
        }
    }

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
