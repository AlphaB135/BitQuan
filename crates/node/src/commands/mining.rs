//! Mining commands for BitQuan CLI
//!
//! This module contains all mining-related commands:
//! - mine_genesis, mine_once, mine_continuous
//! - stratum_server
//!
//! # PUBLIC API JUSTIFICATION
//!
//! This module provides helper functions and constants for the mining commands.
//! Even though some items are not directly used in the binary, they are part of the
//! library's public API and may be used by external users.
//!
//! Therefore, dead_code warnings are allowed for this module.

#![allow(dead_code)]

use bitquan_consensus::{
    asert_next_target, check_header_pow, clamp_bits_within_bounds, compact_to_target, header_hash,
    target_to_compact, ConsensusEngine, ConsensusParams, DifficultyState, DEVNET_MAX_BITS,
};
use bitquan_network::protocol::Message;
use bitquan_storage::{ChainStore, InMemoryChainStore, RocksDBStore};
use bitquan_types::{
    error::{Error, Result},
    genesis::GENESIS_HASH_BYTES,
    Block, NetworkId, SigAlgorithm, Transaction, TxIn, TxOut,
};
use bq_crypto::{
    rng::{RandomSource, RngService},
    CryptoRegistry,
};
use std::collections::VecDeque;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::cli::invalid;

/// 1 BQ = 10^18 qbits (like wei to ETH)
const QBITS_PER_BQ: u128 = 1_000_000_000_000_000_000;

/// Format qbits as BQ using pure integer arithmetic.
/// SECURITY: Never use f64 for money! Floating point causes precision loss.
/// Example: 1_500_000_000_000_000_000 -> "1.500000000000000000"
pub fn format_bq(qbits: u128) -> String {
    let whole = qbits / QBITS_PER_BQ;
    let frac = qbits % QBITS_PER_BQ;
    format!("{}.{:018}", whole, frac)
}

/// Load difficulty_bits from network config file
pub fn load_difficulty_from_config(network: NetworkId) -> Result<u32> {
    let config_file = match network {
        NetworkId::Mainnet => "config/mainnet.toml",
        NetworkId::Testnet => "config/testnet.toml",
        NetworkId::Devnet => "config/devnet.toml",
        NetworkId::Regtest => return Ok(0x207fffff), // Regtest uses easiest difficulty
    };

    // Try to read the config file
    let content = std::fs::read_to_string(config_file).unwrap_or_default();

    // Simple parser to find difficulty_bits line
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("difficulty_bits") {
            if let Some(value_part) = trimmed.split('=').nth(1) {
                let value = value_part.trim().trim_matches('"').trim();
                if let Some(hex_str) = value.strip_prefix("0x") {
                    if let Ok(bits) = u32::from_str_radix(hex_str, 16) {
                        return Ok(bits);
                    }
                }
            }
        }
    }

    // Fallback to defaults if config not found or invalid
    Ok(match network {
        NetworkId::Mainnet => 0x1c00ffff,
        NetworkId::Testnet => 0x1d00ffff,
        NetworkId::Devnet => 0x207fffff,
        NetworkId::Regtest => 0x207fffff,
    })
}

/// Load block from disk or create test block
pub fn load_block_placeholder() -> Result<Block> {
    // Load block from disk or create test block
    let block = Block {
        header: bitquan_types::BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            uncles_hash: [0u8; 32],
            time: 0,
            bits: 0,
            nonce: 0,
            algo_id: 0,
        },
        uncles: Vec::new(),
        transactions: Vec::new(),
    };
    Ok(block)
}

type PendingTransactionsResult = (Vec<Transaction>, Vec<[u8; 32]>, Box<dyn FnOnce()>);

/// Load and VALIDATE pending transactions from pending_transactions.jsonl file
/// Returns (valid_transactions, included_txids, cleanup_fn)
/// - Validates Dilithium5 signatures before inclusion
/// - Cleanup removes only successfully included transactions
pub fn load_pending_transactions() -> PendingTransactionsResult {
    use std::io::BufRead;

    let pending_path = PathBuf::from("data/pending_transactions.jsonl");
    let mut valid_transactions = Vec::new();
    let mut valid_txids = Vec::new();

    if pending_path.exists() {
        if let Ok(file) = std::fs::File::open(&pending_path) {
            let reader = std::io::BufReader::new(file);
            for line_result in reader.lines() {
                let line = match line_result {
                    Ok(l) => l,
                    Err(_) => break,
                };
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(tx_str) = entry.get("tx").and_then(|v| v.as_str()) {
                        // Deserialize from JSON string (avoids u128 overflow when embedded)
                        if let Ok(tx) = serde_json::from_str::<Transaction>(tx_str) {
                            // SECURITY: Validate Dilithium5 signature before inclusion
                            let is_valid = validate_transaction_signature(&tx);

                            if is_valid {
                                let txid = tx.txid();
                                valid_txids.push(txid);
                                valid_transactions.push(tx);
                            }
                        } // Skip invalid transactions
                    }
                }
            }
        }
    }

    // Cleanup function: remove included transactions, keep failed ones
    // Calculates txid from each transaction instead of relying on file's txid field
    let cleanup_path = pending_path.clone();
    let cleanup_txids = valid_txids.clone();
    let cleanup = Box::new(move || {
        if !cleanup_path.exists() {
            return;
        }

        // Read all entries, filter out included ones by calculating txid from transaction
        if let Ok(content) = std::fs::read_to_string(&cleanup_path) {
            let remaining: Vec<&str> = content
                .lines()
                .filter(|line| {
                    // Deserialize the entry to get the transaction
                    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                        // Extract transaction string
                        if let Some(tx_str) = entry.get("tx").and_then(|v| v.as_str()) {
                            // Deserialize transaction and calculate its txid
                            if let Ok(tx) = serde_json::from_str::<Transaction>(tx_str) {
                                let calculated_txid = tx.txid();
                                // Keep if NOT in cleanup_txids
                                !cleanup_txids.iter().any(|id| id == &calculated_txid)
                            } else {
                                true // Keep entries with invalid transactions
                            }
                        } else {
                            true // Keep entries without tx field
                        }
                    } else {
                        true // Keep unparseable lines
                    }
                })
                .collect();

            if remaining.is_empty() {
                let _ = std::fs::remove_file(&cleanup_path);
            } else {
                let _ = std::fs::write(&cleanup_path, remaining.join("\n") + "\n");
            }
        }
    });

    (valid_transactions, valid_txids, cleanup)
}

/// Validate transaction signature using Dilithium5
/// Returns true if signature is valid, false otherwise
pub fn validate_transaction_signature(tx: &Transaction) -> bool {
    use pqc_dilithium_seeded as dilithium;

    // Coinbase transactions don't have signatures
    if tx.inputs.len() == 1 && tx.inputs[0].prev_txid == [0u8; 32] {
        return true;
    }

    // Must have at least one witness
    if tx.witnesses.is_empty() {
        return false;
    }

    // Get the message that was signed (transaction without witnesses)
    // IMPORTANT: Must match wallet.rs signing: serde_json::to_string().as_bytes()
    let mut tx_for_signing = tx.clone();
    tx_for_signing.witnesses.clear();
    let msg = match serde_json::to_string(&tx_for_signing) {
        Ok(s) => s.into_bytes(),
        Err(_) => return false,
    };

    // Verify each witness signature
    for witness in &tx.witnesses {
        for sig_payload in &witness.signatures {
            // Verify Dilithium5 signature
            if dilithium::crypto_sign_verify(&sig_payload.signature, &msg, &sig_payload.public_key)
                .is_err()
            {
                return false;
            }
        }
    }

    true
}

/// Mine the genesis block
pub fn mine_genesis(max_tries: u64, output: &str) -> Result<()> {
    use bitquan_types::{create_genesis_block, is_valid_genesis, GENESIS_BITS, GENESIS_TIME};
    use std::time::Instant;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║   BitQuan Genesis Block Miner        ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Parameters:");
    println!(" Time:    {}", GENESIS_TIME);
    println!(" Bits:    0x{:08x}", GENESIS_BITS);
    println!(" Max tries: {}", max_tries);
    println!(" Output:   {}", output);
    println!();

    // Create genesis block template
    let mut genesis = create_genesis_block();

    println!("Genesis Message:");
    let msg = &genesis.transactions[0].inputs[0].script_sig;
    println!(" {}", String::from_utf8_lossy(msg));
    println!();

    println!("🔨 Mining genesis block...");
    println!();

    let start_time = Instant::now();
    let mut found = false;

    for nonce in 0..max_tries {
        genesis.header.nonce = nonce;

        if let Ok(true) = check_header_pow(&genesis.header) {
            let hash = header_hash(&genesis.header);
            let elapsed = start_time.elapsed();
            let hashrate = (nonce as f64) / elapsed.as_secs_f64();

            println!("GENESIS BLOCK FOUND!");
            println!();
            println!("Nonce:   {}", nonce);
            println!("Hash:    {}", hex::encode(hash));
            println!("Time:    {:.2}s", elapsed.as_secs_f64());
            println!("Hashrate:  {:.2} H/s", hashrate);
            println!();

            // Validate genesis
            // Validate genesis
            if !is_valid_genesis(&genesis) {
                return Err(bitquan_types::Error::Invalid(
                    "Invalid genesis block".into(),
                ));
            }

            // Save to JSON
            let json = serde_json::to_string_pretty(&genesis)?;
            fs::write(output, json)?;

            println!("Genesis block saved to: {}", output);
            println!();
            println!("Next steps:");
            println!(" 1. Update GENESIS_HASH in crates/types/src/genesis.rs");
            println!(" 2. Commit genesis block to repository");
            println!(" 3. Use this block to initialize blockchain");
            println!();

            found = true;
            break;
        }

        if nonce % 100_000 == 0 && nonce > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let hashrate = (nonce as f64) / elapsed;
            let hash = header_hash(&genesis.header);
            println!(
                " ... {} attempts ({:.2} H/s) | Hash: {}",
                nonce,
                hashrate,
                &hex::encode(hash)[..16]
            );
        }
    }

    if !found {
        println!(
            "Failed to find valid genesis block in {} attempts",
            max_tries
        );
        println!("Try increasing --max-tries or adjusting difficulty");
    }

    Ok(())
}

/// Validate a block provided from an external source (placeholder for Phase 4)
///
/// This is a placeholder command that will be implemented to parse and validate
/// blocks from external sources. Currently demonstrates block validation logic.
pub fn check_block(path: &str) -> Result<()> {
    println!(
        "Block validation placeholder invoked for file: {path}. \
     Actual parsing logic will be implemented in Phase 4."
    );

    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::default();
    let mut engine = ConsensusEngine::new(params, registry);
    let block = load_block_placeholder()?;

    match engine.validate_block(&block, 0, 0, &[], &std::collections::HashSet::new()) {
        Ok(report) => {
            println!("Block validation successful!");
            println!("  Weight: {} WU", report.block_weight);
            println!("  Signatures: {}", report.signature_count);
            println!("  Subsidy: {} qbits", report.block_subsidy);
        }
        Err(e) => {
            return invalid(format!("Block validation failed: {}", e));
        }
    }

    Ok(())
}

/// Generate random bytes and derived streams using the BitQuan RNG
///
/// Demonstrates the RNG service by generating master and derived random streams.
/// Useful for testing cryptographic operations and randomness quality.
pub fn rng_demo(label: &str, length: usize) -> Result<()> {
    if length == 0 {
        println!("Length must be greater than zero.");
        return Ok(());
    }

    let mut master =
        RngService::new().map_err(|e| Error::Invalid(format!("rng init failed: {e}")))?;
    let mut derived = master.derive_stream(label);

    let master_bytes = master
        .bytes(length)
        .map_err(|e| Error::Invalid(format!("rng bytes failed: {e}")))?;
    let derived_bytes = derived
        .bytes(length)
        .map_err(|e| Error::Invalid(format!("rng bytes failed: {e}")))?;

    println!(
        "Master stream sample ({length} bytes): {}",
        hex::encode(master_bytes)
    );
    println!(
        "Derived stream `{label}` ({length} bytes): {}",
        hex::encode(derived_bytes)
    );

    Ok(())
}

/// Mine a single block template by iterating nonces up to a limit (demo CPU miner)
///
/// This is a simple demonstration miner that iterates through nonces to find
/// a valid proof-of-work solution. Useful for testing but not efficient for
/// serious mining.
pub fn mine_once(
    max_tries: u64,
    payout_script_hex: &str,
    mut bits: u32,
    network: NetworkId,
    pow_mode: crate::PowMode,
) -> Result<()> {
    use bitquan_types::{
        genesis::GENESIS_HASH_BYTES, Block, BlockHeader, SigAlgorithm, Transaction, TxOut,
    };
    let mut store = InMemoryChainStore::new();

    let allow_mock = matches!(pow_mode, crate::PowMode::Mock);

    // Determine timestamp safely with bounds checking
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    if now == 0 {
        eprintln!("Error: System time is before UNIX epoch");
        return Ok(());
    }

    let mut time = now;
    if let Some(mtp) = store.mtp() {
        time = time.max(mtp.saturating_add(1));
    } else if let Ok(Some(tip)) = store.tip() {
        time = time.max(tip.time.saturating_add(1));
    }

    // Build coinbase (placeholder coinbase input and payout output)
    let payout_script = hex::decode(payout_script_hex)
        .map_err(|e| Error::Invalid(format!("invalid payout script hex: {e}")))?;
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
    let subsidy = bitquan_consensus::ConsensusParams::phase3_defaults()
        .reward_schedule
        .subsidy_at_height(store.height());
    let coinbase = Transaction {
        version: 2,
        network,
        genesis_hash: GENESIS_HASH_BYTES,
        lock_time: 0,
        inputs: vec![coinbase_in],
        outputs: vec![TxOut {
            value: subsidy,
            script_pubkey: payout_script,
        }],
        sig_algo: SigAlgorithm::Dilithium5,
        witnesses: vec![],
    };

    // Load pending transactions from file (with signature validation)
    let (pending_txs, _valid_txids, cleanup) = load_pending_transactions();
    if !pending_txs.is_empty() {
        println!(
            "Found {} valid pending transaction(s) to include",
            pending_txs.len()
        );
    }

    // Build transactions list: coinbase + pending
    let mut all_txs = vec![coinbase];
    all_txs.extend(pending_txs);

    // Merkle/witness roots from all transactions
    let txids: Vec<[u8; 32]> = all_txs.iter().map(|tx| tx.txid()).collect();
    let wtxids: Vec<[u8; 32]> = all_txs.iter().map(|tx| tx.wtxid()).collect();
    let merkle_root = bitquan_types::merkle_root_from_txids(&txids)?;
    let witness_root = bitquan_types::merkle_root_from_txids(&wtxids)?;

    // Determine prev_block from tip if any
    let mut prev = [0u8; 32];
    if let Ok(Some(tip)) = store.tip() {
        prev = header_hash(&tip);
    }

    // Auto-calc bits if zero using DifficultyState anchored at tip
    if bits == 0 {
        let params = ConsensusParams::phase3_defaults();
        let (anchor_bits, anchor_time) = if let Ok(Some(tip)) = store.tip() {
            (tip.bits, tip.time as u64)
        } else {
            (0x1c00ffff, now as u64)
        };
        let mut state = DifficultyState::new(0, anchor_time, anchor_bits, 0);
        bits = state.update(1, time as u64, &params);
    }

    bits = clamp_bits_within_bounds(bits);

    let mut header = BlockHeader {
        version: 1,
        prev_block: prev,
        merkle_root,
        pqc_agg_hint: witness_root,
        uncles_hash: [0u8; 32],
        time,
        bits,
        nonce: 0,
        algo_id: 0,
    };

    if allow_mock {
        println!(
            "[mock-pow] enabled on {:?}: nonce=0 or bits>=0x{:08x} will satisfy difficulty",
            network, DEVNET_MAX_BITS
        );
    }

    for n in 0..max_tries {
        header.nonce = n;
        let pow_valid = if allow_mock {
            header.nonce == 0 || header.bits >= DEVNET_MAX_BITS
        } else {
            check_header_pow(&header)
                .map_err(|e| Error::Invalid(format!("pow verification failed: {e}")))?
        };

        if pow_valid {
            let id = header_hash(&header);
            println!("FOUND nonce={n} hash={}", hex::encode(id));
            let block = Block {
                header: header.clone(),
                uncles: vec![],
                transactions: all_txs,
            };
            let _ = store.insert_block(block);
            println!("Inserted block tip={}", hex::encode(id));

            // Cleanup: remove included transactions from pending file
            cleanup();

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

/// Options for continuous mining operations
pub struct MiningOptions {
    /// Data directory for blockchain storage
    pub datadir: String,
    /// Payout script in hexadecimal format
    pub payout_script_hex: String,
    /// Override for difficulty bits
    pub bits_override: u32,
    /// Maximum nonce value to try
    pub max_nonce: u64,
    /// Number of mining threads
    pub threads: usize,
    /// Optional limit on number of blocks to mine
    pub limit_blocks: Option<u64>,
    /// Network identifier
    pub network: NetworkId,
    /// Proof-of-work algorithm mode
    pub pow_mode: crate::PowMode,
    /// Optional weights for hybrid mining algorithms
    pub hybrid_weights: Option<Vec<(bitquan_consensus::pow::PowAlgo, f32)>>,
    /// List of peer addresses to connect
    pub peers: Vec<String>,
}

/// Continuous mining with persistent RocksDB storage
#[cfg(feature = "rocksdb-backend")]
pub fn mine_continuous(options: MiningOptions) -> Result<()> {
    let MiningOptions {
        datadir,
        payout_script_hex,
        bits_override,
        max_nonce,
        threads,
        limit_blocks,
        network,
        pow_mode,
        hybrid_weights,
        peers,
    } = options;

    #[derive(Clone, Copy)]
    struct BlockLog {
        height: u64,
        timestamp: i64,
        bits: u32, // Correct: Store exact bits from header
    }

    let params = ConsensusParams::phase3_defaults();
    let window = params.difficulty.burst_guard_window as usize;

    // Open or create RocksDB store
    let store = RocksDBStore::open(&datadir)
        .map_err(|e| Error::Invalid(format!("failed to open RocksDB: {e}")))?;
    let store = Arc::new(Mutex::new(store));

    let payout_script = hex::decode(payout_script_hex)
        .map_err(|e| Error::Invalid(format!("invalid payout script hex: {e}")))?;
    let found = Arc::new(AtomicBool::new(false));
    let blocks_mined = Arc::new(AtomicU64::new(0));

    // Initialize PeerManager (with automatic seed bootstrap if no peers specified)
    let peer_manager = {
        use bitquan_network::{NoiseConfig, PeerManager};

        // Path for peers.json persistence
        let peers_json = PathBuf::from("peers.json");
        let peers_file_exists = peers_json.exists();

        println!("\n=== P2P Network Configuration ===");

        // Generate Noise Protocol keypair for P2P encryption
        let noise_config = Arc::new(
            NoiseConfig::generate()
                .map_err(|e| Error::Invalid(format!("failed to generate noise config: {e}")))?,
        );
        println!(
            "P2P Encryption enabled (public key: {})",
            noise_config.public_key_hex()
        );

        let pm = Arc::new(PeerManager::new(
            125, // max_peers
            network,
            noise_config,
        ));

        // Load existing peers from file if available
        if peers_file_exists {
            match pm.load_address_book(&peers_json) {
                Ok(()) => {
                    // peer_count() returns usize directly, use block_on in sync context
                    let rt = tokio::runtime::Handle::try_current();
                    if let Ok(handle) = rt {
                        let count = handle.block_on(pm.peer_count());
                        println!("Loaded {} peers from peers.json", count);
                    }
                }
                Err(e) => {
                    println!("Failed to load peers.json: {}, starting fresh", e);
                }
            }
        }

        // Determine bootstrap peers: CLI args > cached peers > TESTNET_SEEDS
        let bootstrap_peers: Vec<String> = if !peers.is_empty() {
            // CLI-provided peers take priority
            println!("Connecting to {} peer(s) from CLI...", peers.len());
            peers.clone()
        } else {
            // No CLI peers: check if we have cached peers
            let known_count = pm.known_peers_count().unwrap_or(0);
            if known_count > 0 {
                println!("Using {} cached peers from address book", known_count);
                pm.get_known_peers()
                    .unwrap_or_default()
                    .into_iter()
                    .take(10) // Connect to top 10 cached peers
                    .map(|addr| format!("{}:{}", addr.ip, addr.port))
                    .collect()
            } else {
                // No cached peers: use TESTNET_SEEDS
                println!("No cached peers, using TESTNET_SEEDS...");
                bitquan_network::TESTNET_SEEDS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
        };

        // Update peer manager with current chain height
        let current_height = {
            let s = store
                .lock()
                .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
            s.height()
                .map_err(|e| Error::Invalid(format!("storage height error: {e}")))?
        };

        // update_height() is async and returns (), use block_on in sync context
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(pm.update_height(current_height));
        }

        // Connect to bootstrap peers
        let mut connected_count = 0;
        for peer_addr in &bootstrap_peers {
            let addr: SocketAddr = match peer_addr.parse() {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Invalid peer address '{}': {}", peer_addr, e);
                    continue;
                }
            };

            print!(" Connecting to {}... ", peer_addr);
            // connect_peer() is async, use block_on in sync context
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                match handle.block_on(pm.connect_peer(addr)) {
                    Ok(()) => {
                        println!("Connected");
                        connected_count += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed: {}", e);
                    }
                }
            }
        }

        if connected_count > 0 {
            println!(
                "\nConnected to {}/{} peers",
                connected_count,
                bootstrap_peers.len()
            );
            // ready_peer_count() is async and returns usize directly
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                let ready = handle.block_on(pm.ready_peer_count());
                println!("Ready peers: {}", ready);
            }
            println!("================================\n");
            Some(pm)
        } else {
            eprintln!("Warning: Failed to connect to any peers. Mining will continue without network connectivity.\n");
            Some(pm) // Return pm anyway for future peer discovery
        }
    };

    let mut history: VecDeque<BlockLog> = VecDeque::with_capacity(window + 2);
    let mut last_timestamp: Option<i64> = None;
    let mut bits = bits_override;
    let allow_mock = matches!(pow_mode, crate::PowMode::Mock);

    // Load difficulty from config file if not overridden
    if bits == 0 {
        bits = load_difficulty_from_config(network)?;
        println!(
            "Loaded difficulty from config: 0x{:08x} for {:?}",
            bits, network
        );
    } else {
        println!("Using override difficulty: 0x{:08x}", bits);
    }

    println!("BitQuan Continuous Miner - TESTING PENDING TX FILE");
    println!("Data directory: {}", datadir);
    println!(
        "Threads: {}",
        if threads == 0 {
            num_cpus::get()
        } else {
            threads
        }
    );
    println!("Network: {:?}", network);
    println!("PoW mode: {:?}", pow_mode);
    if allow_mock {
        println!(
            "[mock-pow] enabled: nonce=0 or bits>=0x{:08x} will satisfy difficulty",
            DEVNET_MAX_BITS
        );
    }

    // Initialize hybrid miner if applicable
    let hybrid_miner = if matches!(pow_mode, crate::PowMode::Hybrid) {
        use bitquan_consensus::pow::PowAlgo;
        let weights = if let Some(w) = hybrid_weights {
            w
        } else {
            vec![(PowAlgo::Sha256d, 1.0), (PowAlgo::Ethash, 2.0)]
        };

        println!("\n=== Hybrid Mining Enabled ===");
        println!("Algorithms:");
        for (algo, weight) in &weights {
            println!(" - {} (weight: {:.1})", algo.name(), weight);
        }
        println!("=============================\n");

        let miner = crate::miner::HybridMiner::new(&weights, threads, network)?;
        Some(miner)
    } else {
        None
    };

    {
        let s = store
            .lock()
            .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
        let current_height = s
            .height()
            .map_err(|e| Error::Invalid(format!("storage height error: {e}")))?;
        if current_height > 0 {
            let start = current_height
                .saturating_sub(params.difficulty.burst_guard_window)
                .saturating_add(1);
            for h in start..=current_height {
                if let Some(block) = s
                    .get_block_by_height(h)
                    .map_err(|e| Error::Invalid(format!("storage block fetch error: {e}")))?
                {
                    let log = BlockLog {
                        height: h,
                        timestamp: block.header.time as i64,
                        bits: block.header.bits, // Correct: No conversion to f64
                    };
                    last_timestamp = Some(log.timestamp);
                    history.push_back(log);
                }
            }
        }
    }

    if bits != 0 {
        bits = clamp_bits_within_bounds(bits);
    }

    let mut total_intervals = 0u64;
    let mut interval_count = 0u64;
    let mut guard_total = 0u64;

    loop {
        let height = {
            let s = store
                .lock()
                .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
            s.height()
                .map_err(|e| Error::Invalid(format!("storage height error: {e}")))?
        };

        print!("Block #{} ", height + 1);

        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);

        if now == 0 {
            eprintln!("ERROR: System time is before UNIX epoch");
            print_session_summary(interval_count, total_intervals, guard_total);
            return Ok(());
        }

        let mut time = now;
        {
            let s = store
                .lock()
                .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
            if let Ok(Some(tip)) = s.tip() {
                time = time.max(tip.time.saturating_add(1));
            }
        }

        // Build coinbase
        let height_le = ((height + 1) as u32).to_le_bytes();
        let mut script_sig = height_le.to_vec();
        script_sig.extend_from_slice(&time.to_le_bytes());

        let coinbase_in = TxIn {
            prev_txid: [0u8; 32],
            prev_vout: u32::MAX,
            sequence: u32::MAX,
            script_sig,
        };

        let subsidy = params.reward_schedule.subsidy_at_height(height);
        let coinbase = Transaction {
            version: 2,
            network,
            genesis_hash: GENESIS_HASH_BYTES,
            lock_time: 0,
            inputs: vec![coinbase_in],
            outputs: vec![TxOut {
                value: subsidy,
                script_pubkey: payout_script.clone(),
            }],
            witnesses: vec![],
            sig_algo: SigAlgorithm::Dilithium5,
        };

        // Load pending transactions from file (with signature validation)
        println!("TRACE: About to load pending transactions...");
        let (pending_txs, _valid_txids, cleanup) = load_pending_transactions();
        println!("TRACE: Loaded {} pending transactions", pending_txs.len());
        if !pending_txs.is_empty() {
            println!(
                "\nFound {} valid pending transaction(s) to include",
                pending_txs.len()
            );
        }

        // Build transactions list: coinbase + pending
        let mut all_txs = vec![coinbase.clone()];
        all_txs.extend(pending_txs);

        // Merkle/witness roots from all transactions
        let txids: Vec<[u8; 32]> = all_txs.iter().map(|tx| tx.txid()).collect();
        let wtxids: Vec<[u8; 32]> = all_txs.iter().map(|tx| tx.wtxid()).collect();
        let merkle_root = bitquan_types::merkle_root_from_txids(&txids)?;
        let witness_root = bitquan_types::merkle_root_from_txids(&wtxids)?;

        // Determine prev_block
        let mut prev = [0u8; 32];
        {
            let s = store
                .lock()
                .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
            if let Ok(Some(tip)) = s.tip() {
                prev = header_hash(&tip);
            }
        }

        let mut header = bitquan_types::BlockHeader {
            version: 1,
            prev_block: prev,
            merkle_root,
            pqc_agg_hint: witness_root,
            uncles_hash: [0u8; 32],
            time,
            bits,
            nonce: 0,
            algo_id: 0,
        };

        // Mining loop with real-time progress
        found.store(false, Ordering::Relaxed);
        let start_time = std::time::Instant::now();
        let mut last_update = std::time::Instant::now();
        let update_interval = std::time::Duration::from_millis(100); // Update every 100ms

        // Initial display
        print!("\r\x1b[36mMining Block #{} | Target: 0x{:08x} | Reward: {} qbits | Hashes: 0 | H/s: 0.00\x1b[0m",
        height + 1, bits, subsidy);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Hybrid mining path
        #[allow(unused_variables)]
        let (_mined_header, algo_used) = if let Some(ref hybrid_miner) = hybrid_miner {
            // Select algorithm based on iteration
            let algo = hybrid_miner.select_algorithm(height);

            // Update display for hybrid mining
            print!("\r\x1b[36mMining Block #{} | Target: 0x{:08x} | Reward: {} qbits | Algo: {} | Hashes: 0 | H/s: 0.00\x1b[0m",
          height + 1, bits, subsidy, algo.name());
            let _ = std::io::Write::flush(&mut std::io::stdout());

            match hybrid_miner.mine_block_attempt(header.clone(), max_nonce, algo)? {
                Some(h) => (Some(h), Some(algo)),
                None => {
                    println!(
                        "\r\x1b[31m✗ No solution found in {} attempts with {}\x1b[0m",
                        max_nonce,
                        algo.name()
                    );
                    continue;
                }
            }
        } else {
            (None, None)
        };

        // Standard SHA-256d mining path
        let (mined_header, _algo_used): (
            Option<bitquan_types::BlockHeader>,
            Option<bitquan_consensus::pow::PowAlgo>,
        ) = (None, None);

        let (header, n) = if let Some(h) = mined_header {
            let nonce = h.nonce;
            (h, nonce)
        } else {
            // Standard sequential mining
            let mut solution_found = false;
            let mut final_nonce = 0;

            for n in 0..max_nonce {
                header.nonce = n;
                let pow_valid = if allow_mock {
                    // Only allow mock if bits are very easy (for testing only)
                    header.bits >= DEVNET_MAX_BITS
                } else {
                    check_header_pow(&header)
                        .map_err(|e| Error::Invalid(format!("pow verification failed: {e}")))?
                };

                // Update progress display every 100ms
                if last_update.elapsed() >= update_interval {
                    let elapsed = start_time.elapsed();
                    let hashrate = (n as f64) / elapsed.as_secs_f64();
                    print!("\r\x1b[36mMining Block #{} | Target: 0x{:08x} | Reward: {} qbits | Hashes: {} | H/s: {:.2}\x1b[0m",
              height + 1, bits, subsidy, n, hashrate);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    last_update = std::time::Instant::now();
                }

                if pow_valid {
                    solution_found = true;
                    final_nonce = n;
                    break;
                }
            }

            if !solution_found {
                println!(
                    "\r\x1b[31m✗ No solution found in {} attempts\x1b[0m",
                    max_nonce
                );
                continue;
            }

            (header, final_nonce)
        };

        let id = header_hash(&header);
        let elapsed = start_time.elapsed();
        let hashrate = (n as f64) / elapsed.as_secs_f64();

        // Clear the mining line and show result with color based on hashrate
        let hash_str = hex::encode(id);
        let elapsed_str = format!("{:.2}", elapsed.as_secs_f64());
        let hashrate_str = format!("{:.2}", hashrate);

        // Determine color based on hashrate (real mining vs mock/easy)
        let color_code = if hashrate > 0.0 {
            "\x1b[32m" // Green for real mining
        } else {
            "\x1b[37m" // White/gray for mock/easy blocks
        };

        // Calculate padding to align "Total" at consistent position
        // Target position: around column 120 (adjust as needed)
        let base_line_length = 100; // Approximate length of base info
        let current_length = format!(
            "✓ FOUND Block #{} | Nonce: {} | Hash: {} | {}s | {} H/s",
            height + 1,
            n,
            hash_str,
            elapsed_str,
            hashrate_str
        )
        .len();
        let padding_needed = if current_length < base_line_length {
            base_line_length - current_length
        } else {
            5 // Minimum padding
        };
        let padding = " ".repeat(padding_needed);

        #[cfg(feature = "randomx")]
        if let Some(algo) = algo_used {
            print!(
                "\r{}✓ FOUND Block #{} | Algo: {} | Nonce: {} | Hash: {} | {}s | {} H/s{}\x1b[0m",
                color_code,
                height + 1,
                algo.name(),
                n,
                hash_str,
                elapsed_str,
                hashrate_str,
                padding
            );
        } else {
            print!(
                "\r{}✓ FOUND Block #{} | Nonce: {} | Hash: {} | {}s | {} H/s{}\x1b[0m",
                color_code,
                height + 1,
                n,
                hash_str,
                elapsed_str,
                hashrate_str,
                padding
            );
        }
        #[cfg(not(feature = "randomx"))]
        print!(
            "\r{}✓ FOUND Block #{} | Nonce: {} | Hash: {} | {}s | {} H/s{}\x1b[0m",
            color_code,
            height + 1,
            n,
            hash_str,
            elapsed_str,
            hashrate_str,
            padding
        );

        let block = Block {
            header: header.clone(),
            uncles: vec![],
            transactions: all_txs,
        };

        {
            let mut s = store
                .lock()
                .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
            s.insert_block(block.clone())
                .map_err(|e| Error::Invalid(format!("failed to insert block: {e}")))?;
        }

        // Cleanup: remove included transactions from pending file
        cleanup();

        // Broadcast block to connected peers
        if let Some(ref pm) = peer_manager {
            // ready_peer_count() is async, use block_on in sync context
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                let ready_peers = handle.block_on(pm.ready_peer_count());
                if ready_peers > 0 {
                    print!(" | Broadcasting to {} peer(s)...", ready_peers);

                    // Create block message for broadcasting
                    let msg = Message::Block {
                        block: block.clone(),
                    };
                    // broadcast() is async, use block_on
                    match handle.block_on(pm.broadcast(msg)) {
                        Ok(_count) => {
                            print!(" [OK]");
                        }
                        Err(e) => {
                            print!(" [WARN] Broadcast: {}", e);
                        }
                    }
                }
            }
        }

        let block_height = height + 1;
        let block_time = header.time as i64;
        let block_bits = header.bits;
        let _block_target = compact_to_target(block_bits); // Kept for other uses, not stored in BlockLog

        if let Some(prev_ts) = last_timestamp {
            let interval = (block_time - prev_ts).max(0) as u64;
            total_intervals = total_intervals.saturating_add(interval);
            interval_count = interval_count
                .checked_add(1)
                .ok_or(Error::Overflow("interval count overflow"))?;
        }
        last_timestamp = Some(block_time);

        history.push_back(BlockLog {
            height: block_height,
            timestamp: block_time,
            bits: block_bits, // Correct: Store exact bits, not converted target
        });
        if history.len() > window + 1 {
            history.pop_front();
        }

        let anchor = if block_height as usize > window && history.len() > window {
            history[history.len() - 1 - window]
        } else {
            // Get anchor from history - handle empty case gracefully
            *history.front().ok_or_else(|| {
                Error::Invalid("empty block history - cannot determine anchor block".to_string())
            })?
        };

        let height_delta = block_height as i64 - anchor.height as i64;
        let time_delta = block_time - anchor.timestamp;

        // Deterministic fixed-point arithmetic for guard trigger (NO f64 in consensus!)
        // Follow the same pattern as asert.rs::calculate_burst_guard_trigger_fp
        let guard_triggered =
            if height_delta as u64 >= params.difficulty.burst_guard_window && time_delta > 0 {
                let height_delta_abs = height_delta.max(1) as u128;
                let expected_time_fp = height_delta_abs
                    * params.difficulty.target_block_time as u128
                    * bitquan_consensus::FP_SCALE as u128;
                let floor_threshold_fp = (expected_time_fp
                    * params.difficulty.burst_guard_floor_ratio_fp as u128)
                    / bitquan_consensus::FP_SCALE as u128;
                let actual_time_fp = time_delta as u128 * bitquan_consensus::FP_SCALE as u128;
                actual_time_fp < floor_threshold_fp
            } else {
                false
            };
        if guard_triggered {
            guard_total = guard_total
                .checked_add(1)
                .ok_or(Error::Overflow("guard count overflow"))?;
        }

        // Use config difficulty for early blocks (before ASERT kicks in)
        // This ensures network starts with the intended difficulty
        const DIFFICULTY_ADJUSTMENT_START: u64 = 144; // ~1 day of blocks

        if block_height < DIFFICULTY_ADJUSTMENT_START {
            // Keep using config difficulty for first blocks
            let config_bits = load_difficulty_from_config(network)?;
            bits = config_bits;
        } else {
            // Use ASERT difficulty adjustment after sufficient history
            // Convert anchor.bits to target for ASERT calculation
            let anchor_target = compact_to_target(anchor.bits);
            let next_target =
                asert_next_target(anchor_target, height_delta, time_delta, &params, None);
            let mut next_bits = target_to_compact(&next_target);
            if next_bits == 0 {
                next_bits = block_bits;
            }
            next_bits = clamp_bits_within_bounds(next_bits);
            bits = next_bits;
        }

        let total = blocks_mined.fetch_add(1, Ordering::Relaxed) + 1;
        println!(" | Total: {}", total);
        found.store(true, Ordering::Relaxed);

        if let Some(limit) = limit_blocks {
            if total >= limit {
                print_session_summary(interval_count, total_intervals, guard_total);
                println!("Reached block limit ({}). Session complete.", limit);
                return Ok(());
            }
        }

        if !found.load(Ordering::Relaxed) {
            print!(
                "\r\x1b[33mNo valid nonce in {} tries, adjusting difficulty...\x1b[0m\n",
                max_nonce
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
            bits = (bits & 0x00ff_ffff) | ((((bits >> 24) + 1) & 0xff) << 24);
            bits = clamp_bits_within_bounds(bits);
        }
    }
}

#[cfg(not(feature = "rocksdb-backend"))]
pub fn mine_continuous(_options: MiningOptions) -> Result<()> {
    eprintln!("ERROR: Continuous mining requires 'rocksdb-backend' feature");
    eprintln!("Rebuild with: cargo build --release --features rocksdb-backend");
    Ok(())
}

/// Print mining session summary statistics
///
/// Displays interval count, average times, and guard activation statistics
/// for a mining session.
pub fn print_session_summary(interval_count: u64, total_intervals: u64, guard_total: u64) {
    if interval_count == 0 {
        println!("Session summary -> insufficient interval data to compute averages.");
        return;
    }
    // Integer arithmetic for average (whole seconds)
    let average = total_intervals / interval_count;
    // Integer arithmetic for guard rate (percentage * 100)
    let guard_rate = guard_total * 10000 / interval_count;
    // Display as XX.XX%
    println!(
        "Session summary -> avg {}s across {} intervals | guard {} activations ({}.{:02}/100)",
        average,
        interval_count,
        guard_total,
        guard_rate / 100,
        guard_rate % 100
    );
}

/// Run Stratum mining server.
pub fn run_stratum_server(
    bind_addr: String,
    allow_list: Vec<String>,
    default_difficulty: f64,
    network: NetworkId,
) -> Result<()> {
    use crate::stratum_server::{StratumConfig, StratumServer};

    let config = StratumConfig {
        bind_addr: bind_addr.clone(),
        allow_list,
        default_difficulty,
        network,
        enable_vardiff: true,
        vardiff_target_time: 15.0,
        vardiff_adjust_rate: 0.05,
        require_auth: false,
        max_connections_per_ip: 10,
        max_share_rate: 10.0,
        connection_timeout: 300,
        max_connections: 1000,
        enable_rate_limiting: true,
    };

    println!("Starting BitQuan Stratum Mining Server");
    println!(" Bind address: {}", bind_addr);
    println!(" Network: {:?}", network);
    println!(" Default difficulty: {}", default_difficulty);
    println!();

    // Create runtime for async server
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| Error::Invalid(format!("failed to create tokio runtime: {}", e)))?;

    runtime.block_on(async {
        let mut server = StratumServer::new(config);

        // Start server (blocks until shutdown)
        server.start().await
    })
}

/// Parse hybrid weights from CLI string format "sha256d:1,randomx:2".
#[allow(unexpected_cfgs)]
pub fn parse_hybrid_weights(s: &str) -> Result<Vec<(bitquan_consensus::pow::PowAlgo, f32)>> {
    use bitquan_consensus::pow::PowAlgo;

    let mut weights = Vec::new();
    for part in s.split(',') {
        let (key, value) = part.split_once(':').ok_or_else(|| {
            Error::Invalid(format!(
                "invalid weight format: '{}', expected 'algo:weight'",
                part
            ))
        })?;

        let algo = match key.trim() {
            "sha256d" | "sha256" => PowAlgo::Sha256d,
            #[cfg(feature = "randomx")]
            "randomx" => PowAlgo::RandomX,
            // ethash/hybrid features planned for future
            #[cfg(feature = "ethash")]
            #[allow(unexpected_cfgs)]
            "ethash" => PowAlgo::Ethash,
            #[cfg(feature = "hybrid")]
            #[allow(unexpected_cfgs)]
            "hybrid" => PowAlgo::Hybrid,
            _ => {
                return Err(Error::Invalid(format!(
                    "unknown algorithm: '{}'",
                    key.trim()
                )))
            }
        };

        let weight = value
            .trim()
            .parse::<f32>()
            .map_err(|e| Error::Invalid(format!("invalid weight '{}': {}", value, e)))?;

        if weight <= 0.0 {
            return Err(Error::Invalid(format!(
                "weight must be positive: {}",
                weight
            )));
        }

        weights.push((algo, weight));
    }

    if weights.is_empty() {
        return Err(Error::Invalid("no valid weights specified".to_string()));
    }

    Ok(weights)
}
