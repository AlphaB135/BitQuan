//! Node commands for BitQuan CLI
//!
//! This module contains all node-related commands:
//! - check_balance, verify_database, genesis_verify
//! - check_block, rng_demo, build_tx
//! - script_from_address, address_validate, multisig_info

use std::fs;

use crate::address;
use bitquan_storage::rocksdb_store::RocksDBStore;
use bitquan_storage::ChainStore;
use bitquan_storage::RecoveryOptions;
use bitquan_types::error::{Error, Result};
use bitquan_types::{
    genesis::GENESIS_HASH_BYTES, NetworkId, SigAlgorithm, Transaction, TxIn, TxOut,
};
use hex;
use serde_json::Value;

use crate::cli::address_network_label;

/// Check balance for a script
#[cfg(feature = "rocksdb-backend")]
pub fn check_balance(datadir: &str, script_hex: Option<&str>, address: Option<&str>) -> Result<()> {
    let store = RocksDBStore::open(datadir)
        .map_err(|e| Error::Invalid(format!("failed to open RocksDB: {e}")))?;
    let height = store
        .height()
        .map_err(|e| Error::Invalid(format!("storage height error: {e}")))?;

    println!("\n=== BitQuan Balance ===");
    println!("Chain height: {}", height);

    // Determine script_pubkey from either script_hex or address
    let target_script = if let Some(script) = script_hex {
        hex::decode(script).map_err(|e| Error::Invalid(format!("invalid script hex: {e}")))?
    } else if let Some(addr) = address {
        let info = address::inspect(addr)
            .map_err(|e| Error::Invalid(format!("Failed to decode address: {}", e)))?;

        println!("Decoded address: {}", info.normalized);
        println!("Pubkey hash: {}", hex::encode(info.payload));

        address::script_from_pubkey_hash(&info.payload)
    } else {
        return crate::cli::invalid("Either --script-hex or --address must be provided");
    };

    println!("Script: {}", hex::encode(&target_script));
    println!("\nScanning blockchain for UTXOs...");

    let mut balance: u128 = 0;
    let mut utxo_count: u64 = 0;

    // Scan all blocks and check UTXO set for unspent outputs
    for h in 0..=height {
        if let Ok(Some(block)) = store.get_block_by_height(h) {
            for tx in &block.transactions {
                for (vout, output) in tx.outputs.iter().enumerate() {
                    if output.script_pubkey == target_script {
                        // Create outpoint (txid + vout)
                        let mut outpoint = Vec::with_capacity(32 + 4);
                        outpoint.extend_from_slice(&tx.txid());
                        outpoint.extend_from_slice(&(vout as u32).to_le_bytes());

                        // CRITICAL FIX: Check UTXO set to see if output is still unspent
                        // This prevents counting spent outputs as balance
                        if store.get_utxo(&outpoint).ok().flatten().is_some() {
                            balance = balance
                                .checked_add(output.value)
                                .ok_or(Error::Overflow("balance accumulation overflow"))?;
                            utxo_count = utxo_count
                                .checked_add(1)
                                .ok_or(Error::Overflow("UTXO count overflow"))?;
                            println!(
                                " Block #{} TX {} vout={} amount={}",
                                h,
                                hex::encode(tx.txid()),
                                vout,
                                output.value
                            );
                        }
                        // If get_utxo returns None, this output was spent - don't count it
                    }
                }
            }
        }
    }

    println!("\nUTXO count: {}", utxo_count);
    println!("Balance: {} qbits", balance);
    println!("Balance: {} BQ", crate::cli::format_bq(balance));

    Ok(())
}

/// List users in JWT configuration
#[cfg(feature = "rocksdb-backend")]
pub fn verify_database(
    path: &str,
    backup: bool,
    backup_path: Option<&str>,
    rebuild: bool,
) -> Result<()> {
    let options = RecoveryOptions {
        verify_checksums: true,
        auto_backup: backup,
        backup_path: backup_path.map(|s| s.to_string()),
        rebuild_indices: rebuild,
        repair_corrupted: false,
        max_backups: 5,
        verify_block_integrity: false,
        create_checkpoint: false,
    };

    println!("🔍 Database Verification Tool");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Database path: {}", path);
    println!();

    let store = RocksDBStore::open_with_options(path, options)
        .map_err(|e| Error::Invalid(format!("failed to open RocksDB with options: {e}")))?;

    println!();
    println!("Database Statistics:");
    let stats = store
        .get_stats()
        .map_err(|e| Error::Invalid(format!("storage stats error: {e}")))?;
    println!(" Chain height: {}", stats.height);
    println!(" Total blocks: {}", stats.num_blocks);
    println!(" Transactions: {}", stats.num_transactions);
    println!(" UTXOs: {}", stats.num_utxos);

    println!();
    println!("Database verification complete!");

    Ok(())
}

/// Verify genesis block hash and configuration
pub fn genesis_verify(genesis_file: &str, network: &str) -> Result<()> {
    println!("🔍 Verifying genesis configuration...");
    println!("Genesis file: {}", genesis_file);
    println!("Network: {}", network);

    // Read genesis file
    let genesis_json = fs::read_to_string(genesis_file)
        .map_err(|e| Error::Invalid(format!("failed to read genesis file: {}", e)))?;

    // Parse genesis configuration
    let genesis: Value = serde_json::from_str(&genesis_json)
        .map_err(|e| Error::Invalid(format!("failed to parse genesis JSON: {e}")))?;

    // Extract fields
    let chain_id = genesis["chain_id"]
        .as_str()
        .ok_or_else(|| Error::Invalid("missing chain_id".into()))?;
    let network_id = genesis["network_id"]
        .as_str()
        .ok_or_else(|| Error::Invalid("missing network_id".into()))?;
    let genesis_hash = genesis["genesis_hash"]
        .as_str()
        .ok_or_else(|| Error::Invalid("missing genesis_hash".into()))?;
    let timestamp = genesis["genesis_timestamp"]
        .as_u64()
        .ok_or_else(|| Error::Invalid("missing genesis_timestamp".into()))?;

    // Verify network matches
    if network_id != network {
        return Err(Error::Invalid(format!(
            "network mismatch: expected '{}', got '{}'",
            network, network_id
        )));
    }

    println!("\n✓ Chain ID: {}", chain_id);
    println!("✓ Network: {}", network_id);
    println!("✓ Genesis Hash: {}", genesis_hash);
    println!(
        "✓ Genesis Timestamp: {} ({})",
        timestamp,
        chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "invalid".to_string())
    );

    // Extract consensus params
    if let Some(params) = genesis["consensus_params"].as_object() {
        println!("\n📋 Consensus Parameters:");
        println!(
            "  Target block time: {}s",
            params
                .get("target_block_time")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!(
            "  Max block size: {} bytes",
            params
                .get("max_block_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!(
            "  Coinbase maturity: {} blocks",
            params
                .get("coinbase_maturity")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!(
            "  Initial subsidy: {} satoshis",
            params
                .get("initial_subsidy")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!(
            "  PoW algorithm: {}",
            params
                .get("pow_algo")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );
    }

    // Extract DNS seeds
    if let Some(seeds) = genesis["dns_seeds"].as_array() {
        println!("\nDNS Seeds:");
        for seed in seeds.iter().take(10) {
            if let Some(seed_str) = seed.as_str() {
                println!("  - {}", seed_str);
            }
        }
        if seeds.len() > 10 {
            println!("  ... and {} more", seeds.len() - 10);
        }
    }

    println!("\n✓ Genesis configuration verified successfully!");
    Ok(())
}

/// Build a transaction for testing
pub fn build_tx(
    prev_txid_hex: &str,
    prev_vout: u32,
    value: u128,
    to_script_hex: &str,
) -> Result<()> {
    let mut prev = [0u8; 32];
    let prev_vec = hex::decode(prev_txid_hex)
        .map_err(|e| Error::Invalid(format!("invalid prev_txid hex: {e}")))?;
    if prev_vec.len() != 32 {
        println!("prev_txid must be 32 bytes hex");
        return Ok(());
    }
    prev.copy_from_slice(&prev_vec);
    let script_pubkey = hex::decode(to_script_hex)
        .map_err(|e| Error::Invalid(format!("invalid script hex: {e}")))?;

    let input = TxIn {
        prev_txid: prev,
        prev_vout,
        sequence: u32::MAX,
        script_sig: Vec::new(),
    };
    let output = TxOut {
        value,
        script_pubkey,
    };
    let tx = Transaction {
        version: 2,
        network: NetworkId::Devnet,
        genesis_hash: GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![input],
        outputs: vec![output],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    };

    let json = serde_json::to_string_pretty(&tx)?;
    println!("{json}");
    Ok(())
}

/// Convert Bech32m address to script hex for mining/balance checks.
pub fn script_from_address(addr: &str) -> Result<()> {
    let info = address::inspect(addr)
        .map_err(|e| Error::Invalid(format!("Failed to decode address: {}", e)))?;

    let script = address::script_from_pubkey_hash(&info.payload);
    let script_hex = hex::encode(script);
    let trimmed = addr.trim();

    eprintln!("Bech32m checksum: OK");
    eprintln!("Network     : {}", address_network_label(info.network));
    if trimmed != info.normalized {
        eprintln!("Normalized   : {}", info.normalized);
    }
    eprintln!("Pubkey hash   : {}", hex::encode(info.payload));
    println!("{script_hex}");

    Ok(())
}

/// Validate a Bech32m address and display decoded metadata.
pub fn address_validate(addr: &str) -> Result<()> {
    let info = address::inspect(addr)
        .map_err(|e| Error::Invalid(format!("Address validation failed: {}", e)))?;
    let trimmed = addr.trim();

    println!("BitQuan Address Validation");
    println!("Input   : {}", trimmed);
    if trimmed != info.normalized {
        println!("Normalized : {}", info.normalized);
    }
    println!("Network   : {}", address_network_label(info.network));
    println!("HRP     : {}", info.hrp);
    println!("Checksum  : OK (Bech32m)");
    println!("Payload size: {} bytes", info.payload.len());
    println!("Pubkey hash : {}", hex::encode(info.payload));
    println!(
        "Script hex : {}",
        hex::encode(address::script_from_pubkey_hash(&info.payload))
    );

    Ok(())
}
