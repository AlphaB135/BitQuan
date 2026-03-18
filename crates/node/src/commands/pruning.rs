//! Pruning commands for BitQuan CLI
//!
//! This module contains commands for managing blockchain data pruning:
//! - prune: Execute block pruning
//! - info: Show pruning status

use bitquan_storage::{PruningMode, RocksDBStore};
use bitquan_types::error::{Error, Result};

/// Prune blockchain data to reduce disk usage.
///
/// ## Arguments
///
/// * `datadir` - Path to the blockchain data directory
/// * `keep_blocks` - If set, keep only the last N blocks of full data
/// * `utxo_only` - If true, prune to UTXO-only mode (headers + UTXO set only)
/// * `dry_run` - If true, show what would be pruned without actually pruning
#[cfg(feature = "rocksdb-backend")]
pub fn prune_blocks(
    datadir: &str,
    keep_blocks: Option<u64>,
    utxo_only: bool,
    dry_run: bool,
) -> Result<()> {
    println!("🌳 BitQuan Block Pruning");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Data directory: {}", datadir);
    println!();

    // Open the store
    let store = RocksDBStore::open(datadir)
        .map_err(|e| Error::Invalid(format!("Failed to open RocksDB: {}", e)))?;

    // Get current chain height
    let height = store
        .height()
        .map_err(|e| Error::Invalid(format!("Failed to get height: {}", e)))?;

    // Get current pruning metadata
    let metadata = store
        .get_pruning_metadata()
        .map_err(|e| Error::Invalid(format!("Failed to get pruning metadata: {}", e)))?;

    println!("Current chain height: {}", height);
    println!("Current pruning mode: {:?}", metadata.mode);
    if let Some(ph) = metadata.pruning_height {
        println!("Already pruned below height: {}", ph);
        println!("Total blocks pruned: {}", metadata.total_pruned);
    }
    println!();

    // Determine the new pruning mode
    let new_mode = if utxo_only {
        PruningMode::UtxoOnly
    } else if let Some(keep) = keep_blocks {
        PruningMode::Pruned { keep_blocks: keep }
    } else {
        return Err(Error::Invalid(
            "Either --keep-blocks or --utxo-only must be specified".to_string(),
        ));
    };

    // Validate the new mode
    new_mode
        .validate()
        .map_err(|e| Error::Invalid(format!("Invalid pruning configuration: {}", e)))?;

    println!("New pruning mode: {:?}", new_mode);

    // Calculate what would be pruned
    let prune_before_height = match new_mode {
        PruningMode::Full => {
            println!("No pruning needed (Full mode)");
            return Ok(());
        }
        PruningMode::Pruned { keep_blocks } => height.saturating_sub(keep_blocks),
        PruningMode::UtxoOnly => height.saturating_sub(PruningMode::MIN_SAFE_DEPTH),
    };

    if prune_before_height == 0 {
        println!("Chain too short to prune (height: {})", height);
        return Ok(());
    }

    let blocks_to_prune = prune_before_height;
    println!("Would prune blocks below height: {}", blocks_to_prune);
    println!("Blocks to be pruned: {}", blocks_to_prune);

    // Estimate space savings (rough estimate: ~500KB per block with Dilithium signatures)
    let estimated_savings_mb = (blocks_to_prune * 500) / 1024;
    println!("Estimated space savings: ~{} MB", estimated_savings_mb);

    if dry_run {
        println!();
        println!("🔍 DRY RUN - No changes made");
        println!("Run without --dry-run to execute pruning");
        return Ok(());
    }

    println!();
    println!("⚠️  WARNING: Pruning will permanently delete block data!");
    println!("Headers will be kept for SPV verification.");
    println!();

    // Set the new pruning mode
    store
        .set_pruning_mode(new_mode)
        .map_err(|e| Error::Invalid(format!("Failed to set pruning mode: {}", e)))?;

    // Perform the pruning
    println!("Pruning blocks...");
    let pruned = store
        .prune()
        .map_err(|e| Error::Invalid(format!("Pruning failed: {}", e)))?;

    println!();
    println!("✓ Pruning complete!");
    println!("Blocks pruned: {}", pruned);

    // Show updated stats
    let updated_metadata = store
        .get_pruning_metadata()
        .map_err(|e| Error::Invalid(format!("Failed to get updated metadata: {}", e)))?;

    println!("Total blocks pruned: {}", updated_metadata.total_pruned);

    Ok(())
}

/// Show pruning status for the blockchain.
///
/// ## Arguments
///
/// * `datadir` - Path to the blockchain data directory
#[cfg(feature = "rocksdb-backend")]
pub fn pruning_status(datadir: &str) -> Result<()> {
    println!("📊 Pruning Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Data directory: {}", datadir);
    println!();

    let store = RocksDBStore::open(datadir)
        .map_err(|e| Error::Invalid(format!("Failed to open RocksDB: {}", e)))?;

    let height = store
        .height()
        .map_err(|e| Error::Invalid(format!("Failed to get height: {}", e)))?;

    let metadata = store
        .get_pruning_metadata()
        .map_err(|e| Error::Invalid(format!("Failed to get pruning metadata: {}", e)))?;

    println!("Chain height: {}", height);
    println!();

    println!("Pruning Configuration:");
    println!("  Mode: {:?}", metadata.mode);
    println!("  Is Pruned: {}", metadata.is_pruned());
    println!();

    if let Some(pruning_height) = metadata.pruning_height {
        println!("Pruning State:");
        println!("  Pruned below height: {}", pruning_height);
        println!("  Available blocks: {}-{}", pruning_height, height);
        println!("  Total blocks pruned: {}", metadata.total_pruned);

        if let Ok(stats) = store.get_stats() {
            let full_blocks = stats.num_blocks.saturating_sub(metadata.total_pruned);
            println!("  Full blocks stored: {}", full_blocks);
        }

        let last_pruned_ts = metadata.last_pruned;
        if last_pruned_ts > 0 {
            use chrono::DateTime;
            if let Some(dt) = DateTime::from_timestamp(last_pruned_ts as i64, 0) {
                println!("  Last pruned: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        }
    } else {
        println!("Pruning State:");
        println!("  No pruning has been performed");
        println!("  All blocks are stored (full node)");
    }

    println!();

    // Show storage estimates
    match metadata.mode {
        PruningMode::Full => {
            println!("Storage Mode: Full Node");
            println!("  All historical block data is retained");
        }
        PruningMode::Pruned { keep_blocks } => {
            println!("Storage Mode: Pruned");
            println!("  Keeping last {} blocks of full data", keep_blocks);
            println!("  Older blocks: headers only");
        }
        PruningMode::UtxoOnly => {
            println!("Storage Mode: UTXO-Only");
            println!("  Only headers and UTXO set retained");
            println!("  Minimum storage footprint");
        }
    }

    Ok(())
}

/// Check disk space availability.
///
/// ## Arguments
///
/// * `datadir` - Path to the blockchain data directory
pub fn check_disk_space(datadir: &str) -> Result<()> {
    println!("💾 Disk Space Check");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Data directory: {}", datadir);
    println!();

    // Get disk space information
    #[cfg(unix)]
    {
        let path = std::path::Path::new(datadir);
        let stats = nix::sys::statvfs::statvfs(path)
            .map_err(|e| Error::Invalid(format!("Failed to get disk stats: {}", e)))?;

        let total_bytes = (stats.blocks() as u64) * (stats.fragment_size() as u64);
        let available_bytes = (stats.files_available() as u64) * (stats.fragment_size() as u64);
        let used_bytes = total_bytes.saturating_sub(available_bytes);

        let total_gb = total_bytes / (1024 * 1024 * 1024);
        let available_gb = available_bytes / (1024 * 1024 * 1024);
        let used_gb = used_bytes / (1024 * 1024 * 1024);
        let usage_percent = (used_bytes * 100 / total_bytes.max(1)) as u64;

        println!("Disk Space:");
        println!("  Total: {} GB", total_gb);
        println!("  Used: {} GB", used_gb);
        println!("  Available: {} GB", available_gb);
        println!("  Usage: {}%", usage_percent);
        println!();

        // Warnings
        if available_gb < 5 {
            println!("⛔ CRITICAL: Less than 5 GB available!");
            println!("   Pruning or disk expansion required immediately.");
        } else if available_gb < 10 {
            println!("⚠️  WARNING: Less than 10 GB available!");
            println!("   Consider pruning to free up space.");
        } else {
            println!("✓ Disk space is healthy");
        }
    }

    #[cfg(not(unix))]
    {
        println!("Disk space checking not available on this platform");
    }

    Ok(())
}
