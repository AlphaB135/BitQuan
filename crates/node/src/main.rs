//! BitQuan reference node entrypoint.
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(missing_docs)]
#![allow(dead_code)] // Allow utility functions/constants for future use

mod address;
mod block_submit;
mod chainstate;
mod keystore;
mod metrics;
mod miner;
mod mnemonic;
mod pool_template;
mod reward_engine;
#[cfg(feature = "rocksdb-backend")]
mod rpc;
mod stratum_server;
mod sync_task;
mod tx_builder;
mod utxo;
mod vardiff;
mod wallet;
mod worker;
mod ws_dashboard;

// CLI utilities
mod cli;

// Command modules
pub mod commands;

// Import moved command functions
// Note: MiningOptions and PowMode are defined locally in main.rs
use commands::mining::load_pending_transactions;
use commands::node::{
    build_tx, check_balance, genesis_verify, script_from_address, verify_database,
};
use commands::p2p::RpcServerOptions;
use commands::rpc::{
    generate_self_signed_cert_cli, hash_password_cli, jwt_user_add, jwt_user_list, jwt_user_remove,
};
use commands::wallet::{
    multisig_info, tx_combine_signatures, tx_sign_partial, wallet_address, wallet_backup,
    wallet_from_mnemonic, wallet_gen, wallet_gen_mnemonic, wallet_gen_multisig, wallet_restore,
    wallet_send, wallet_sign, wallet_verify,
};
// Import for address validation (moved to commands/node)
use bitquan_consensus::{
    asert_next_target, check_header_pow, clamp_bits_within_bounds, compact_to_target, header_hash,
    target_to_compact_u64, ConsensusEngine, ConsensusParams, DifficultyState, DEVNET_MAX_BITS,
};
use bitquan_network::protocol::Message;
#[cfg(feature = "rocksdb-backend")]
use bitquan_storage::rocksdb_store::RocksDBStore;
use bitquan_storage::{ChainStore, InMemoryChainStore};
use bitquan_types::error::{Error, Result};
use bitquan_types::{
    genesis::GENESIS_HASH_BYTES, Block, NetworkId, SigAlgorithm, Transaction, TxIn, TxOut,
};
use bq_crypto::{
    rng::{RandomSource, RngService},
    CryptoRegistry,
};
use clap::{Parser, Subcommand};
use commands::node::address_validate;
use hex::encode as hex_encode;
use log::error;
use std::collections::VecDeque;
use std::net::SocketAddr;

/// 1 BQ = 10^18 qbits (like wei to ETH)
#[allow(dead_code)]
const QBITS_PER_BQ: u128 = 1_000_000_000_000_000_000;

/// Format qbits as BQ using pure integer arithmetic.
/// SECURITY: Never use f64 for money! Floating point causes precision loss.
/// Example: 1_500_000_000_000_000_000 -> "1.500000000000000000"
#[allow(dead_code)]
fn format_bq(qbits: u128) -> String {
    let whole = qbits / QBITS_PER_BQ;
    let frac = qbits % QBITS_PER_BQ;
    format!("{}.{:018}", whole, frac)
}

/// Proof-of-Work algorithm mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowMode {
    /// Standard SHA-256d hashcash (Bitcoin-style)
    Hashcash,
    /// Mock mode for testing (debug builds only)
    #[allow(dead_code)]
    Mock,
    /// RandomX algorithm (memory-hard)
    #[cfg(feature = "randomx")]
    RandomX,
    /// Hybrid mode combining multiple algorithms
    Hybrid,
    /// Ethash algorithm (Ethereum-style)
    Ethash,
}

impl PowMode {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "hashcash" | "sha256d" | "real" => Ok(PowMode::Hashcash),
            "mock" | "dev-fast-pow" => {
                #[cfg(feature = "testing")]
                return Ok(PowMode::Mock);

                #[cfg(not(feature = "testing"))]
                return crate::cli::invalid(
                    "Mock PoW is only available with '--features testing'. \
                     Use 'hashcash' for real proof-of-work mining.",
                );
            }
            #[cfg(feature = "randomx")]
            "randomx" => Ok(PowMode::RandomX),
            #[cfg(feature = "randomx")]
            "hybrid" => Ok(PowMode::Hybrid),
            #[cfg(not(feature = "randomx"))]
            "hybrid" => Ok(PowMode::Hybrid),
            "ethash" => Ok(PowMode::Ethash),
            other => crate::cli::invalid(format!("unknown pow engine '{}'", other)),
        }
    }
}

fn parse_network_id(value: &str) -> Result<NetworkId> {
    match value.to_ascii_lowercase().as_str() {
        "mainnet" => Ok(NetworkId::Mainnet),
        "testnet" => Ok(NetworkId::Testnet),
        "devnet" => Ok(NetworkId::Devnet),
        "regtest" => Ok(NetworkId::Regtest),
        other => crate::cli::invalid(format!("unknown network '{}'", other)),
    }
}

fn ensure_pow_allowed(pow_mode: PowMode, network: NetworkId) -> Result<()> {
    if matches!(pow_mode, PowMode::Mock) && matches!(network, NetworkId::Mainnet) {
        return crate::cli::invalid("mock PoW is disabled on mainnet");
    }
    #[cfg(feature = "randomx")]
    {
        // Allow hybrid mining on mainnet for multi-algorithm support
        if matches!(pow_mode, PowMode::RandomX) && matches!(network, NetworkId::Mainnet) {
            return crate::cli::invalid("RandomX only mode is disabled on mainnet (use hybrid)");
        }
    }
    Ok(())
}

/// Load difficulty_bits from network config file
fn load_difficulty_from_config(network: NetworkId) -> Result<u32> {
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

/// Parse hybrid weights from CLI string format "sha256d:1,randomx:2".
fn parse_hybrid_weights(s: &str) -> Result<Vec<(bitquan_consensus::pow::PowAlgo, f32)>> {
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
            "sha256d" => PowAlgo::Sha256d,
            "ethash" => PowAlgo::Ethash,
            #[cfg(feature = "randomx")]
            "randomx" => PowAlgo::RandomX,
            other => return crate::cli::invalid(format!("unknown algorithm: '{}'", other)),
        };

        let weight = value
            .trim()
            .parse::<f32>()
            .map_err(|e| Error::Invalid(format!("invalid weight value '{}': {}", value, e)))?;

        if weight <= 0.0 {
            return crate::cli::invalid(format!("weight must be positive for {}", key));
        }

        weights.push((algo, weight));
    }

    if weights.is_empty() {
        return crate::cli::invalid("at least one algorithm weight required");
    }

    Ok(weights)
}

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

/// Custom parser for u128 values in CLI arguments
/// Clap doesn't have built-in u128 support, so we use string parsing
fn parse_u128(s: &str) -> std::result::Result<u128, String> {
    s.parse::<u128>()
        .map_err(|e| format!("Invalid u128 amount: {}", e))
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a placeholder node loop.
    Run {
        /// Path to the node configuration file.
        #[arg(long, default_value = "config/bitquan.toml")]
        config: String,

        /// Override RPC bind address (e.g., "0.0.0.0:18332")
        #[arg(long)]
        rpc_bind: Option<String>,

        /// Override P2P listen address (e.g., "0.0.0.0:18444")
        #[arg(long)]
        p2p_bind: Option<String>,
    },
    /// Mine the genesis block for BitQuan blockchain
    MineGenesis {
        /// Maximum nonce attempts
        #[arg(long, default_value_t = 100_000_000u64)]
        max_tries: u64,
        /// Output file for mined genesis block
        #[arg(long, default_value = "genesis.json")]
        output: String,
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
        /// Compact bits target (e.g., 0x1c00ffff for mainnet difficulty).
        #[arg(long, default_value_t = 0x1c00ffff)]
        bits: u32,
        /// Network to target (mainnet|testnet|devnet|regtest).
        #[arg(long, value_name = "NETWORK", default_value = "mainnet")]
        network: String,
        /// Proof-of-Work engine (hashcash|mock).
        #[arg(long, value_name = "POW", default_value = "hashcash")]
        pow: String,
    },
    /// Continuous mining mode with persistent storage
    Mine {
        /// Data directory for blockchain storage
        #[arg(long, default_value = "./data/chainstate")]
        datadir: String,
        /// Hex-encoded script_pubkey for coinbase payout.
        #[arg(long, default_value = "76a9140088ac")]
        payout_script_hex: String,
        /// Compact bits target (0 = auto-adjust from chain)
        #[arg(long, default_value_t = 0)]
        bits: u32,
        /// Maximum nonce per block attempt
        #[arg(long, default_value_t = 100_000_000u64)]
        max_nonce: u64,
        /// Network to target (mainnet|testnet|devnet|regtest).
        #[arg(long, value_name = "NETWORK", default_value = "mainnet")]
        network: String,
        /// Proof-of-Work engine (hashcash|mock|randomx|ethash|kawpow|hybrid).
        #[arg(long, value_name = "POW", default_value = "hashcash")]
        pow: String,
        /// Number of threads for mining (0 = CPU count)
        #[arg(long, default_value_t = 1)]
        threads: usize,
        /// Optional limit on number of blocks to mine in this session.
        #[arg(long)]
        limit_blocks: Option<u64>,
        /// Hybrid algorithm weights (e.g., "sha256d:1,ethash:2,kawpow:1").
        #[arg(long)]
        hybrid_weights: Option<String>,
        /// RandomX cache mode (fast|full).
        #[cfg(feature = "randomx")]
        #[arg(long, default_value = "fast")]
        randomx_mode: String,
        /// RandomX seed as hex (uses genesis hash if not provided).
        #[cfg(feature = "randomx")]
        #[arg(long)]
        randomx_seed: Option<String>,
        /// Peer addresses to connect to (e.g., 149.56.132.54:18444). Can be specified multiple times.
        #[arg(long)]
        peers: Vec<String>,
    },
    /// Generates a post-quantum keypair for wallet
    WalletGen {
        /// Algorithm (dilithium5, falcon512, sphincs)
        #[arg(long, default_value = "dilithium5")]
        algo: String,
        /// Network to target (mainnet|testnet|devnet|regtest)
        #[arg(long, value_name = "NETWORK", default_value = "mainnet")]
        network: String,
        /// Output file for keypair (optional)
        #[arg(long)]
        output: Option<String>,
        /// Password to encrypt keystore (interactive prompt if not provided)
        #[arg(long)]
        password: Option<String>,
    },
    /// Generate wallet from BIP39 mnemonic phrase
    WalletGenMnemonic {
        /// Number of words (12 or 24)
        #[arg(long, default_value_t = 12)]
        words: usize,
        /// Output file for keypair (optional)
        #[arg(long)]
        output: Option<String>,
        /// Password to encrypt the keystore (interactive prompt if not provided)
        #[arg(long)]
        password: Option<String>,
        /// Show mnemonic phrase (WARNING: insecure if logged)
        #[arg(long, default_value_t = true)]
        show_mnemonic: bool,
    },
    /// Recover wallet from BIP39 mnemonic phrase
    WalletFromMnemonic {
        /// Mnemonic phrase (will prompt if not provided)
        #[arg(long)]
        mnemonic: Option<String>,
        /// Optional passphrase for additional security
        #[arg(long)]
        passphrase: Option<String>,
        /// Output file for keypair (optional)
        #[arg(long)]
        output: Option<String>,
        /// Password to encrypt the keystore (interactive prompt if not provided)
        #[arg(long)]
        password: Option<String>,
    },
    /// Create encrypted backup of wallet keystore
    WalletBackup {
        /// Path to wallet keystore file
        #[arg(long)]
        keystore: String,
        /// Output backup file path
        #[arg(long)]
        output: String,
        /// Backup password (separate from wallet password, will prompt if not provided)
        #[arg(long)]
        backup_password: Option<String>,
        /// Network: mainnet, testnet, or devnet
        #[arg(long, default_value = "mainnet")]
        network: String,
        /// Optional backup label
        #[arg(long)]
        label: Option<String>,
    },
    /// Restore wallet from encrypted backup
    WalletRestore {
        /// Path to backup file
        #[arg(long)]
        backup: String,
        /// Output keystore path
        #[arg(long)]
        output: String,
        /// Backup password (will prompt if not provided)
        #[arg(long)]
        backup_password: Option<String>,
    },
    /// Generate multi-signature wallet address
    WalletGenMultisig {
        /// Required number of signatures (M in M-of-N)
        #[arg(long)]
        threshold: usize,
        /// Paths to keystore files for all signers
        #[arg(long, value_delimiter = ',')]
        keystores: Vec<String>,
        /// Optional labels for signers (comma-separated)
        #[arg(long, value_delimiter = ',')]
        labels: Vec<String>,
        /// Output file for multisig config
        #[arg(long, default_value = "multisig.json")]
        output: String,
    },
    /// Show multi-signature wallet information
    MultisigInfo {
        /// Path to multisig config file
        #[arg(long)]
        config: String,
    },
    /// Sign transaction with partial signature for multisig
    TxSignPartial {
        /// Path to transaction file
        #[arg(long)]
        tx: String,
        /// Path to signer's keystore
        #[arg(long)]
        keystore: String,
        /// Path to multisig config
        #[arg(long)]
        multisig_config: String,
        /// Output file for partial signature
        #[arg(long)]
        output: String,
        /// Password to decrypt keystore
        #[arg(long)]
        password: Option<String>,
    },
    /// Combine partial signatures into final transaction
    TxCombineSignatures {
        /// Path to transaction file
        #[arg(long)]
        tx: String,
        /// Paths to partial signature files (comma-separated)
        #[arg(long, value_delimiter = ',')]
        signatures: Vec<String>,
        /// Path to multisig config
        #[arg(long)]
        multisig_config: String,
        /// Output file for signed transaction
        #[arg(long)]
        output: String,
    },
    /// Import/show wallet address from keypair file
    WalletAddress {
        /// Path to keystore file
        #[arg(long)]
        keystore: String,
        /// Password to decrypt the keystore
        #[arg(long)]
        password: Option<String>,
    },
    /// Convert a Bech32m address to script hex (stdout emits only the hex value).
    #[command(alias = "address-to-script")]
    ScriptFromAddress {
        /// Bech32m address (e.g., q1...)
        #[arg(long)]
        address: String,
    },
    /// Validate a Bech32m address and display decoded metadata.
    #[command(name = "validateaddress", alias = "address-validate")]
    ValidateAddress {
        /// Bech32m address (e.g., q1...)
        #[arg(long)]
        address: String,
    },
    /// Sign a message with wallet keypair
    WalletSign {
        /// Path to keystore file
        #[arg(long)]
        keystore: String,
        /// Message to sign (hex-encoded)
        #[arg(long)]
        message: String,
        /// Password to decrypt the keystore
        #[arg(long)]
        password: Option<String>,
    },
    /// Verify a signature
    WalletVerify {
        /// Public key (hex-encoded)
        #[arg(long)]
        pubkey: String,
        /// Message (hex-encoded)
        #[arg(long)]
        message: String,
        /// Signature (hex-encoded)
        #[arg(long)]
        signature: String,
    },
    /// Send transaction from wallet
    WalletSend {
        /// Path to keystore file
        #[arg(long)]
        keystore: String,
        /// Recipient address
        #[arg(long)]
        to: String,
        /// Amount to send (in qbits)
        #[arg(long, value_parser = parse_u128)]
        amount: u128,
        /// Fee rate (qbits per weight unit)
        #[arg(long, default_value_t = 1)]
        fee_rate: u64,
        /// Password to decrypt the keystore
        #[arg(long)]
        password: Option<String>,
        /// Data directory for blockchain storage
        #[arg(long, default_value = "data/chainstate")]
        datadir: String,
    },
    /// Builds a simple unsigned transaction (1-in, 1-out) and prints JSON.
    BuildTx {
        /// Previous txid (hex, 32 bytes big-endian)
        #[arg(long)]
        prev_txid: String,
        /// Previous output index
        #[arg(long)]
        prev_vout: u32,
        /// Output value in qbits (1 BQ = 10^18 qbits)
        #[arg(long)]
        value: u128,
        /// Hex-encoded script_pubkey for recipient
        #[arg(long)]
        to_script_hex: String,
    },
    /// Run a local P2P handshake demo (server+client) on a TCP address.
    P2PDemo {
        /// Address to bind/connect (e.g., 127.0.0.1:18444)
        #[arg(long, default_value = "127.0.0.1:18444")]
        addr: String,
    },
    /// Start a P2P server that accepts peer connections
    P2PServer {
        /// Address to bind (e.g., 0.0.0.0:8333)
        #[arg(long, default_value = "127.0.0.1:8333")]
        listen: String,
        /// Maximum number of peers
        #[arg(long, default_value_t = 125)]
        max_peers: usize,
        /// Data directory for blockchain storage
        #[arg(long, default_value = "./data/chainstate")]
        datadir: String,
        /// Network to target (mainnet|testnet|devnet|regtest).
        #[arg(long, value_name = "NETWORK", default_value = "mainnet")]
        network: String,
        /// Optional RPC bind address (e.g., 127.0.0.1:8332)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        rpc_listen: Option<String>,
        /// RPC username (required if RPC server enabled)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        rpc_username: Option<String>,
        /// RPC password (required if RPC server enabled)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        rpc_password: Option<String>,
        /// Maximum RPC request body size (bytes)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = 1_048_576)]
        rpc_max_body: usize,
        /// RPC rate-limit burst (tokens per IP)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = 20)]
        rpc_rl_burst: u32,
        /// RPC rate-limit refill per second (tokens)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = 10)]
        rpc_rl_refill_per_sec: u32,
        /// RPC per-connection cooldown in milliseconds
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = 10)]
        rpc_conn_cooldown_ms: u64,
        /// RPC header size limit (bytes)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = 8_192)]
        rpc_max_header: usize,
        /// RPC header read timeout (milliseconds)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = 1_000)]
        rpc_header_timeout_ms: u64,
        /// Trust proxy X-Forwarded-For header for client IP
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = false)]
        rpc_trust_proxy: bool,
        /// Comma-separated CIDR list of trusted proxies
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, value_delimiter = ',')]
        rpc_trusted_cidr: Vec<String>,
        /// Path to PEM-encoded TLS certificate for RPC server
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        rpc_tls_cert: Option<String>,
        /// Path to PEM-encoded TLS private key for RPC server
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        rpc_tls_key: Option<String>,
        /// Allow running RPC without TLS (development only)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long, default_value_t = false)]
        rpc_allow_insecure: bool,
        /// Path to JWT configuration file (enables JWT authentication)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        jwt_config: Option<String>,
        /// JWT secret key (alternative to jwt_config file)
        #[cfg(feature = "rocksdb-backend")]
        #[arg(long)]
        jwt_secret: Option<String>,
        /// Peer addresses to connect to on startup (e.g., "127.0.0.1:18444")
        #[arg(long, value_delimiter = ',')]
        connect: Vec<String>,
    },
    /// Generate a self-signed TLS certificate for RPC (development use)
    #[cfg(feature = "rocksdb-backend")]
    GenerateCert {
        /// Output directory to place cert.pem/key.pem
        #[arg(long, default_value = ".")]
        output: String,
    },
    /// Hash a password using Argon2id for JWT configuration
    HashPassword {
        /// Password to hash (will prompt if not provided)
        password: Option<String>,
    },
    /// Add a user to JWT configuration file
    JwtUserAdd {
        /// Path to JWT configuration file
        #[arg(long, default_value = "jwt.toml")]
        config: String,
        /// Username
        #[arg(long)]
        username: String,
        /// Role (admin, miner, readonly)
        #[arg(long, default_value = "readonly")]
        role: String,
        /// Password (will prompt if not provided)
        #[arg(long)]
        password: Option<String>,
    },
    /// Remove a user from JWT configuration file
    JwtUserRemove {
        /// Path to JWT configuration file
        #[arg(long, default_value = "jwt.toml")]
        config: String,
        /// Username to remove
        #[arg(long)]
        username: String,
    },
    /// List users in JWT configuration file
    JwtUserList {
        /// Path to JWT configuration file
        #[arg(long, default_value = "jwt.toml")]
        config: String,
    },
    /// Verify database integrity and optionally create backup
    #[cfg(feature = "rocksdb-backend")]
    VerifyDb {
        /// Path to database directory
        #[arg(long, default_value = "data/chaindata")]
        path: String,
        /// Create backup before verification
        #[arg(long)]
        backup: bool,
        /// Backup directory path
        #[arg(long)]
        backup_path: Option<String>,
        /// Rebuild indices if corrupted
        #[arg(long)]
        rebuild: bool,
    },
    /// Connect to a peer as a client
    P2PConnect {
        /// Peer address to connect to (e.g., 127.0.0.1:8333)
        #[arg(long)]
        peer: String,
        /// Our current block height
        #[arg(long, default_value_t = 0)]
        height: u64,
        /// Network to target (mainnet|testnet|devnet|regtest)
        #[arg(long, default_value = "devnet")]
        network: String,
    },
    /// Start Stratum mining server
    StratumServer {
        /// Bind address for Stratum server
        #[arg(long, default_value = "0.0.0.0:3333")]
        stratum_bind: String,
        /// Allowed client IPs (comma-separated)
        #[arg(long, default_value = "127.0.0.1")]
        stratum_allow: String,
        /// Default difficulty for miners
        #[arg(long, default_value_t = 1.0)]
        stratum_diff: f64,
        /// Network to target
        #[arg(long, value_name = "NETWORK", default_value = "devnet")]
        network: String,
    },
    /// Check balance for a given script/address
    Balance {
        /// Data directory for blockchain storage
        #[arg(long, default_value = "./data/chainstate")]
        datadir: String,
        /// Hex-encoded script_pubkey to check balance for
        #[arg(long)]
        script_hex: Option<String>,
        /// Bech32m address to check balance for (alternative to script-hex)
        #[arg(long)]
        address: Option<String>,
    },
    /// Verify genesis block hash and configuration
    GenesisVerify {
        /// Path to genesis JSON file
        #[arg(long, default_value = "genesis/mainnet.json")]
        genesis_file: String,
        /// Network to verify against (mainnet|testnet|devnet)
        #[arg(long, default_value = "mainnet")]
        network: String,
    },
}

/// Install panic hook for better crash reporting
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        error!("\n=== PANIC ===");
        if let Some(location) = panic_info.location() {
            error!(
                "Location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            error!("Message: {}", s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            error!("Message: {}", s);
        }
        error!("==============\n");
    }));
}

#[allow(clippy::too_many_arguments)]
#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook for better crash reporting
    install_panic_hook();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            config,
            rpc_bind,
            p2p_bind,
        } => {
            let network = load_network_from_config(&config)?;
            run_node(&config, rpc_bind.as_deref(), p2p_bind.as_deref(), network).await
        }
        Commands::MineGenesis { max_tries, output } => mine_genesis(max_tries, &output),
        Commands::CheckBlock { path } => check_block(&path),
        Commands::Rng { label, length } => rng_demo(&label, length),
        Commands::MineOnce {
            max_tries,
            payout_script_hex,
            bits,
            network,
            pow,
        } => {
            let network_id = parse_network_id(&network)?;
            let pow_mode = PowMode::parse(&pow)?;
            ensure_pow_allowed(pow_mode, network_id)?;
            mine_once(max_tries, &payout_script_hex, bits, network_id, pow_mode)
        }
        Commands::Mine {
            datadir,
            payout_script_hex,
            bits,
            max_nonce,
            network,
            pow,
            threads,
            limit_blocks,
            hybrid_weights,
            #[cfg(feature = "randomx")]
                randomx_mode: _randomx_mode,
            #[cfg(feature = "randomx")]
                randomx_seed: _randomx_seed,
            peers,
        } => {
            let network_id = parse_network_id(&network)?;
            let pow_mode = PowMode::parse(&pow)?;
            ensure_pow_allowed(pow_mode, network_id)?;

            let weights = if matches!(pow_mode, PowMode::Hybrid) {
                Some(parse_hybrid_weights(
                    hybrid_weights.as_deref().unwrap_or("sha256d:1,ethash:2"),
                )?)
            } else {
                None
            };

            let mining_handle = tokio::task::spawn_blocking(move || {
                mine_continuous(MiningOptions {
                    datadir,
                    payout_script_hex,
                    bits_override: bits,
                    max_nonce,
                    threads,
                    limit_blocks,
                    network: network_id,
                    pow_mode,
                    hybrid_weights: weights,
                    peers,
                })
            });
            mining_handle
                .await
                .map_err(|e| Error::Invalid(format!("mining task failed: {e}")))?
        }
        Commands::WalletGen {
            algo,
            network,
            output,
            password,
        } => wallet_gen(&algo, &network, output.as_deref(), password.as_deref()),
        Commands::WalletGenMnemonic {
            words,
            output,
            password,
            show_mnemonic,
        } => wallet_gen_mnemonic(words, output.as_deref(), password.as_deref(), show_mnemonic),
        Commands::WalletFromMnemonic {
            mnemonic,
            passphrase,
            output,
            password,
        } => wallet_from_mnemonic(
            mnemonic.as_deref(),
            passphrase.as_deref(),
            output.as_deref(),
            password.as_deref(),
        ),
        Commands::WalletBackup {
            keystore,
            output,
            backup_password,
            network,
            label,
        } => wallet_backup(
            &keystore,
            &output,
            backup_password.as_deref(),
            &network,
            label.clone(),
        ),
        Commands::WalletRestore {
            backup,
            output,
            backup_password,
        } => wallet_restore(&backup, &output, backup_password.as_deref()),
        Commands::WalletGenMultisig {
            threshold,
            keystores,
            labels,
            output,
        } => wallet_gen_multisig(threshold, &keystores, &labels, &output),
        Commands::MultisigInfo { config } => multisig_info(&config),
        Commands::TxSignPartial {
            tx,
            keystore,
            multisig_config,
            output,
            password,
        } => tx_sign_partial(
            &tx,
            &keystore,
            &multisig_config,
            &output,
            password.as_deref(),
        ),
        Commands::TxCombineSignatures {
            tx,
            signatures,
            multisig_config,
            output,
        } => tx_combine_signatures(&tx, &signatures, &multisig_config, &output),
        Commands::WalletAddress { keystore, password } => {
            wallet_address(&keystore, password.as_deref())
        }
        Commands::ScriptFromAddress { address } => script_from_address(&address),
        Commands::ValidateAddress { address } => address_validate(&address),
        Commands::WalletSign {
            keystore,
            message,
            password,
        } => wallet_sign(&keystore, &message, password.as_deref()),
        Commands::WalletVerify {
            pubkey,
            message,
            signature,
        } => wallet_verify(&pubkey, &message, &signature),
        Commands::WalletSend {
            keystore,
            to,
            amount,
            fee_rate,
            password,
            datadir,
        } => {
            wallet_send(
                &keystore,
                &to,
                amount,
                fee_rate,
                password.as_deref(),
                &datadir,
            )
            .await
        }
        Commands::BuildTx {
            prev_txid,
            prev_vout,
            value,
            to_script_hex,
        } => build_tx(&prev_txid, prev_vout, value, &to_script_hex),
        Commands::P2PDemo { addr } => commands::p2p::p2p_demo(&addr),
        Commands::P2PServer {
            listen,
            max_peers,
            datadir,
            network,
            #[cfg(feature = "rocksdb-backend")]
            rpc_listen,
            #[cfg(feature = "rocksdb-backend")]
            rpc_username,
            #[cfg(feature = "rocksdb-backend")]
            rpc_password,
            #[cfg(feature = "rocksdb-backend")]
            rpc_max_body,
            #[cfg(feature = "rocksdb-backend")]
            rpc_rl_burst,
            #[cfg(feature = "rocksdb-backend")]
            rpc_rl_refill_per_sec,
            #[cfg(feature = "rocksdb-backend")]
            rpc_conn_cooldown_ms,
            #[cfg(feature = "rocksdb-backend")]
            rpc_max_header,
            #[cfg(feature = "rocksdb-backend")]
            rpc_header_timeout_ms,
            #[cfg(feature = "rocksdb-backend")]
            rpc_trust_proxy,
            #[cfg(feature = "rocksdb-backend")]
            rpc_trusted_cidr,
            #[cfg(feature = "rocksdb-backend")]
            rpc_tls_cert,
            #[cfg(feature = "rocksdb-backend")]
            rpc_tls_key,
            #[cfg(feature = "rocksdb-backend")]
            rpc_allow_insecure,
            #[cfg(feature = "rocksdb-backend")]
            jwt_config,
            #[cfg(feature = "rocksdb-backend")]
            jwt_secret,
            connect,
        } => {
            #[cfg(feature = "rocksdb-backend")]
            {
                let network_id = parse_network_id(&network)?;
                commands::p2p::p2p_server(
                    &listen,
                    max_peers,
                    &datadir,
                    RpcServerOptions {
                        listen: rpc_listen.as_deref(),
                        username: rpc_username.as_deref(),
                        password: rpc_password.as_deref(),
                        max_body_bytes: rpc_max_body,
                        rl_burst: rpc_rl_burst,
                        rl_refill_per_sec: rpc_rl_refill_per_sec,
                        conn_cooldown_ms: rpc_conn_cooldown_ms,
                        max_header_bytes: rpc_max_header,
                        header_timeout_ms: rpc_header_timeout_ms,
                        trust_proxy: rpc_trust_proxy,
                        trusted_cidr: rpc_trusted_cidr,
                        tls_cert: rpc_tls_cert.as_deref(),
                        tls_key: rpc_tls_key.as_deref(),
                        allow_insecure: rpc_allow_insecure,
                        jwt_config_path: jwt_config.as_deref(),
                        jwt_secret: jwt_secret.as_deref(),
                    },
                    network_id,
                    Some(connect), // bootstrap_peers: connect to specified peers
                )
                .await
            }
            #[cfg(not(feature = "rocksdb-backend"))]
            {
                let network_id = parse_network_id(&network)?;
                let _ = (&listen, max_peers, &datadir);
                commands::p2p::p2p_server(
                    &listen,
                    max_peers,
                    &datadir,
                    RpcServerOptions {
                        listen: None,
                        username: None,
                        password: None,
                    },
                    network_id,
                    Some(connect), // bootstrap_peers: connect to specified peers
                )
                .await
            }
        }
        Commands::P2PConnect {
            peer,
            height,
            network,
        } => {
            let network_id = parse_network_id(&network)?;
            commands::p2p::p2p_connect(&peer, height, network_id).await
        }
        Commands::StratumServer {
            stratum_bind,
            stratum_allow,
            stratum_diff,
            network,
        } => {
            let network_id = parse_network_id(&network)?;
            let allow_list = stratum_allow
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            run_stratum_server(stratum_bind, allow_list, stratum_diff, network_id)
        }
        Commands::Balance {
            datadir,
            script_hex,
            address,
        } => check_balance(&datadir, script_hex.as_deref(), address.as_deref()),
        #[cfg(feature = "rocksdb-backend")]
        Commands::GenerateCert { output } => generate_self_signed_cert_cli(&output),
        Commands::HashPassword { password } => hash_password_cli(password.as_deref()),
        Commands::JwtUserAdd {
            config,
            username,
            role,
            password,
        } => jwt_user_add(&config, &username, &role, password.as_deref()),
        Commands::JwtUserRemove { config, username } => jwt_user_remove(&config, &username),
        Commands::JwtUserList { config } => jwt_user_list(&config),
        #[cfg(feature = "rocksdb-backend")]
        Commands::VerifyDb {
            path,
            backup,
            backup_path,
            rebuild,
        } => verify_database(&path, backup, backup_path.as_deref(), rebuild),
        Commands::GenesisVerify {
            genesis_file,
            network,
        } => genesis_verify(&genesis_file, &network),
    }
}

fn load_network_from_config(path: &str) -> Result<NetworkId> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("id") {
            if let Some((_, value)) = line.split_once('=') {
                let val = value.trim().trim_matches('"').trim();
                return parse_network_id(val);
            }
        }
    }
    Ok(NetworkId::Mainnet)
}

async fn run_node(
    config_path: &str,
    rpc_bind: Option<&str>,
    p2p_bind: Option<&str>,
    network: NetworkId,
) -> Result<()> {
    // Parse config file for settings
    let config_content = std::fs::read_to_string(config_path).unwrap_or_default();

    // Extract db_path from config (default to ./data/chainstate)
    let datadir = extract_config_value(&config_content, "db_path")
        .unwrap_or_else(|| "./data/chainstate".to_string());

    // Extract p2p_port from config for deriving metrics port
    let config_p2p_port: u16 = extract_config_value(&config_content, "p2p_port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(18444);

    // Use CLI override or config value for P2P address
    let p2p_addr = p2p_bind
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("0.0.0.0:{}", config_p2p_port));

    let _rpc_addr = rpc_bind.unwrap_or("0.0.0.0:18332"); // Currently unused

    log::info!(
    "Starting BitQuan node with configuration: {config_path}\nP2P listening on {p2p_addr}\nData directory: {datadir}"
  );

    // Create data directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&datadir) {
        error!(
            "Warning: Failed to create data directory {}: {}",
            datadir, e
        );
    }

    // Metrics server will be started by commands::p2p::p2p_server() with proper port derivation

    // Extract bootstrap_nodes from config
    let bootstrap_peers = extract_config_array(&config_content, "bootstrap_nodes");
    let bootstrap_peers_opt = if bootstrap_peers.is_empty() {
        None
    } else {
        Some(bootstrap_peers)
    };

    commands::p2p::p2p_server(
        &p2p_addr,
        50, // max_peers
        &datadir,
        RpcServerOptions {
            listen: rpc_bind, // FIX: Use the CLI argument!
            username: Some("admin"),
            password: Some("admin"),
            #[cfg(feature = "rocksdb-backend")]
            jwt_config_path: None,
            #[cfg(feature = "rocksdb-backend")]
            jwt_secret: None,
            #[cfg(feature = "rocksdb-backend")]
            max_body_bytes: 1_000_000,
            #[cfg(feature = "rocksdb-backend")]
            rl_burst: 10,
            #[cfg(feature = "rocksdb-backend")]
            rl_refill_per_sec: 1,
            #[cfg(feature = "rocksdb-backend")]
            conn_cooldown_ms: 1000,
            #[cfg(feature = "rocksdb-backend")]
            max_header_bytes: 8192,
            #[cfg(feature = "rocksdb-backend")]
            header_timeout_ms: 5000,
            #[cfg(feature = "rocksdb-backend")]
            trust_proxy: false,
            #[cfg(feature = "rocksdb-backend")]
            trusted_cidr: vec![],
            #[cfg(feature = "rocksdb-backend")]
            tls_cert: None,
            #[cfg(feature = "rocksdb-backend")]
            tls_key: None,
            #[cfg(feature = "rocksdb-backend")]
            allow_insecure: true, // FIX: Allow insecure for development
        },
        network,
        bootstrap_peers_opt,
    )
    .await
}

/// Extract a simple key = "value" or key = value from TOML content
fn extract_config_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some((_, value)) = line.split_once('=') {
                let val = value.trim().trim_matches('"').trim();
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Extract an array value like bootstrap_nodes = ["addr1", "addr2"]
fn extract_config_array(content: &str, key: &str) -> Vec<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some((_, value)) = line.split_once('=') {
                let val = value.trim();
                // Parse simple array: ["a", "b"]
                if val.starts_with('[') && val.ends_with(']') {
                    let inner = &val[1..val.len() - 1];
                    return inner
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

/// Mine the genesis block
fn mine_genesis(max_tries: u64, output: &str) -> Result<()> {
    use bitquan_types::{create_genesis_block, is_valid_genesis, GENESIS_BITS, GENESIS_TIME};
    use std::fs;
    use std::time::Instant;

    log::info!("╔══════════════════════════════════════════════════╗");
    log::info!("║   BitQuan Genesis Block Miner        ║");
    log::info!("╚══════════════════════════════════════════════════╝");
    log::info!("");
    log::info!("Parameters:");
    log::info!(" Time:    {}", GENESIS_TIME);
    log::info!(" Bits:    0x{:08x}", GENESIS_BITS);
    log::info!(" Max tries: {}", max_tries);
    log::info!(" Output:   {}", output);
    log::info!("");

    // Create genesis block template
    let mut genesis = create_genesis_block();

    log::info!("Genesis Message:");
    let msg = &genesis.transactions[0].inputs[0].script_sig;
    log::info!(" {}", String::from_utf8_lossy(msg));
    log::info!("");

    log::info!("🔨 Mining genesis block...");
    log::info!("");

    let start_time = Instant::now();
    let mut found = false;

    for nonce in 0..max_tries {
        genesis.header.nonce = nonce;

        if let Ok(true) = check_header_pow(&genesis.header) {
            let hash = header_hash(&genesis.header);
            let elapsed = start_time.elapsed();
            let hashrate = (nonce as f64) / elapsed.as_secs_f64();

            log::info!("GENESIS BLOCK FOUND!");
            log::info!("");
            log::info!("Nonce:   {}", nonce);
            log::info!("Hash:    {}", hex_encode(hash));
            log::info!("Time:    {:.2}s", elapsed.as_secs_f64());
            log::info!("Hashrate:  {:.2} H/s", hashrate);
            log::info!("");

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

            log::info!("Genesis block saved to: {}", output);
            log::info!("");
            log::info!("Next steps:");
            log::info!(" 1. Update GENESIS_HASH in crates/types/src/genesis.rs");
            log::info!(" 2. Commit genesis block to repository");
            log::info!(" 3. Use this block to initialize blockchain");
            log::info!("");

            found = true;
            break;
        }

        if nonce % 100_000 == 0 && nonce > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let hashrate = (nonce as f64) / elapsed;
            let hash = header_hash(&genesis.header);
            log::info!(
                " ... {} attempts ({:.2} H/s) | Hash: {}",
                nonce,
                hashrate,
                &hex_encode(hash)[..16]
            );
        }
    }

    if !found {
        log::info!(
            "Failed to find valid genesis block in {} attempts",
            max_tries
        );
        log::info!("Try increasing --max-tries or adjusting difficulty");
    }

    Ok(())
}

fn check_block(path: &str) -> Result<()> {
    log::info!(
        "Block validation placeholder invoked for file: {path}. \
     Actual parsing logic will be implemented in Phase 4."
    );

    let params = ConsensusParams::phase3_defaults();
    let registry = CryptoRegistry::default();
    let mut engine = ConsensusEngine::new(params, registry);
    let block = load_block_placeholder()?;

    match engine.validate_block(&block, 0, 0) {
        Ok(report) => {
            log::info!("Block validation successful!");
            log::info!("  Weight: {} WU", report.block_weight);
            log::info!("  Signatures: {}", report.signature_count);
            log::info!("  Subsidy: {} qbits", report.block_subsidy);
        }
        Err(e) => {
            return crate::cli::invalid(format!("Block validation failed: {}", e));
        }
    }

    Ok(())
}

fn rng_demo(label: &str, length: usize) -> Result<()> {
    if length == 0 {
        log::info!("Length must be greater than zero.");
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

    log::info!(
        "Master stream sample ({length} bytes): {}",
        hex_encode(master_bytes)
    );
    log::info!(
        "Derived stream `{label}` ({length} bytes): {}",
        hex_encode(derived_bytes)
    );

    Ok(())
}

fn load_block_placeholder() -> Result<Block> {
    // Load block from disk or create test block
    let block = Block {
        header: bitquan_types::BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 0,
            bits: 0,
            nonce: 0,
            algo_id: 0,
        },
        transactions: Vec::new(),
    };
    Ok(block)
}

fn mine_once(
    max_tries: u64,
    payout_script_hex: &str,
    mut bits: u32,
    network: NetworkId,
    pow_mode: PowMode,
) -> Result<()> {
    use bitquan_types::{
        genesis::GENESIS_HASH_BYTES, Block, BlockHeader, SigAlgorithm, Transaction, TxOut,
    };
    let mut store = InMemoryChainStore::new();

    let allow_mock = matches!(pow_mode, PowMode::Mock);

    // Determine timestamp safely with bounds checking
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    if now == 0 {
        error!("Error: System time is before UNIX epoch");
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

    // Merkle/witness roots for block (support multi-tx in future)
    let merkle_root = bitquan_types::merkle_root_from_txids(&[coinbase.txid()])?;
    let witness_root = bitquan_types::merkle_root_from_txids(&[coinbase.wtxid()])?;

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
        time,
        bits,
        nonce: 0,
        algo_id: 0,
    };

    if allow_mock {
        log::info!(
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
            log::info!("FOUND nonce={n} hash={}", hex::encode(id));
            let block = Block {
                header: header.clone(),
                transactions: vec![coinbase],
            };
            let _ = store.insert_block(block);
            log::info!("Inserted block tip={}", hex::encode(id));
            return Ok(());
        }
        if n % 100_000 == 0 {
            let h = header_hash(&header);
            log::info!("... tried {n} nonces, latest hash={} ", hex::encode(h));
        }
    }
    log::info!("No valid nonce found within {max_tries} tries.");
    Ok(())
}

struct MiningOptions {
    datadir: String,
    payout_script_hex: String,
    bits_override: u32,
    max_nonce: u64,
    threads: usize,
    limit_blocks: Option<u64>,
    network: NetworkId,
    pow_mode: PowMode,
    hybrid_weights: Option<Vec<(bitquan_consensus::pow::PowAlgo, f32)>>,
    peers: Vec<String>,
}

/// Continuous mining with persistent RocksDB storage
#[cfg(feature = "rocksdb-backend")]
fn mine_continuous(options: MiningOptions) -> Result<()> {
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
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

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
        use std::path::PathBuf;

        // Path for peers.json persistence
        let peers_json = PathBuf::from("peers.json");
        let peers_file_exists = peers_json.exists();

        log::info!("\n=== P2P Network Configuration ===");

        // Generate Noise Protocol keypair for P2P encryption
        let noise_config = Arc::new(
            NoiseConfig::generate()
                .map_err(|e| Error::Invalid(format!("failed to generate noise config: {e}")))?,
        );
        log::info!(
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
                        log::info!("Loaded {} peers from peers.json", count);
                    }
                }
                Err(e) => {
                    log::info!("Failed to load peers.json: {}, starting fresh", e);
                }
            }
        }

        // Determine bootstrap peers: CLI args > cached peers > TESTNET_SEEDS
        let bootstrap_peers: Vec<String> = if !peers.is_empty() {
            // CLI-provided peers take priority
            log::info!("Connecting to {} peer(s) from CLI...", peers.len());
            peers.clone()
        } else {
            // No CLI peers: check if we have cached peers
            let known_count = pm.known_peers_count().unwrap_or(0);
            if known_count > 0 {
                log::info!("Using {} cached peers from address book", known_count);
                pm.get_known_peers()
                    .unwrap_or_default()
                    .into_iter()
                    .take(10) // Connect to top 10 cached peers
                    .map(|addr| format!("{}:{}", addr.ip, addr.port))
                    .collect()
            } else {
                // No cached peers: use TESTNET_SEEDS
                log::info!("No cached peers, using TESTNET_SEEDS...");
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
                    error!("Invalid peer address '{}': {}", peer_addr, e);
                    continue;
                }
            };

            print!(" Connecting to {}... ", peer_addr);
            // connect_peer() is async, use block_on in sync context
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                match handle.block_on(pm.connect_peer(addr)) {
                    Ok(()) => {
                        log::info!("Connected");
                        connected_count += 1;
                    }
                    Err(e) => {
                        error!("Failed: {}", e);
                    }
                }
            }
        }

        if connected_count > 0 {
            log::info!(
                "\nConnected to {}/{} peers",
                connected_count,
                bootstrap_peers.len()
            );
            // ready_peer_count() is async and returns usize directly
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                let ready = handle.block_on(pm.ready_peer_count());
                log::info!("Ready peers: {}", ready);
            }
            log::info!("================================\n");
            Some(pm)
        } else {
            error!("Warning: Failed to connect to any peers. Mining will continue without network connectivity.\n");
            Some(pm) // Return pm anyway for future peer discovery
        }
    };

    let mut history: VecDeque<BlockLog> = VecDeque::with_capacity(window + 2);
    let mut last_timestamp: Option<i64> = None;
    let mut bits = bits_override;
    let allow_mock = matches!(pow_mode, PowMode::Mock);

    // Load difficulty from config file if not overridden
    if bits == 0 {
        bits = load_difficulty_from_config(network)?;
        log::info!(
            "Loaded difficulty from config: 0x{:08x} for {:?}",
            bits, network
        );
    } else {
        log::info!("Using override difficulty: 0x{:08x}", bits);
    }

    log::info!("BitQuan Continuous Miner");
    log::info!("Data directory: {}", datadir);
    log::info!(
        "Threads: {}",
        if threads == 0 {
            num_cpus::get()
        } else {
            threads
        }
    );
    log::info!("Network: {:?}", network);
    log::info!("PoW mode: {:?}", pow_mode);
    if allow_mock {
        log::info!(
            "[mock-pow] enabled: nonce=0 or bits>=0x{:08x} will satisfy difficulty",
            DEVNET_MAX_BITS
        );
    }

    // Initialize hybrid miner if applicable
    let hybrid_miner = if matches!(pow_mode, PowMode::Hybrid) {
        use bitquan_consensus::pow::PowAlgo;
        let weights = if let Some(w) = hybrid_weights {
            w
        } else {
            vec![(PowAlgo::Sha256d, 1.0), (PowAlgo::Ethash, 2.0)]
        };

        log::info!("\n=== Hybrid Mining Enabled ===");
        log::info!("Algorithms:");
        for (algo, weight) in &weights {
            log::info!(" - {} (weight: {:.1})", algo.name(), weight);
        }
        log::info!("=============================\n");

        let miner = miner::HybridMiner::new(&weights, threads, network)?;
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
            error!("ERROR: System time is before UNIX epoch");
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

        // Load pending transactions for inclusion in this block
        let (pending_txs, _included_txids, cleanup_fn) = load_pending_transactions();

        // Build complete transaction list: coinbase + pending transactions
        let mut all_txs = vec![coinbase.clone()];
        all_txs.extend(pending_txs);

        // Merkle/witness roots for block (include all transactions)
        let all_txids: Vec<[u8; 32]> = all_txs.iter().map(|tx| tx.txid()).collect();
        let all_wtxids: Vec<[u8; 32]> = all_txs.iter().map(|tx| tx.wtxid()).collect();
        let merkle_root = bitquan_types::merkle_root_from_txids(&all_txids)?;
        let witness_root = bitquan_types::merkle_root_from_txids(&all_wtxids)?;

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
                    log::info!(
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
                log::info!(
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
            transactions: all_txs,
        };

        {
            let mut s = store
                .lock()
                .map_err(|e| Error::Invalid(format!("store lock poisoned: {e}")))?;
            s.insert_block(block.clone())
                .map_err(|e| Error::Invalid(format!("failed to insert block: {e}")))?;
        }

        // Cleanup pending transactions that were included in this block
        cleanup_fn();

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
            let mut next_bits = target_to_compact_u64(next_target);
            if next_bits == 0 {
                next_bits = block_bits;
            }
            next_bits = clamp_bits_within_bounds(next_bits);
            bits = next_bits;
        }

        let total = blocks_mined.fetch_add(1, Ordering::Relaxed) + 1;
        log::info!(" | Total: {}", total);
        found.store(true, Ordering::Relaxed);

        if let Some(limit) = limit_blocks {
            if total >= limit {
                print_session_summary(interval_count, total_intervals, guard_total);
                log::info!("Reached block limit ({limit}). Session complete.");
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

fn print_session_summary(interval_count: u64, total_intervals: u64, guard_total: u64) {
    if interval_count == 0 {
        log::info!("Session summary -> insufficient interval data to compute averages.");
        return;
    }
    // Integer arithmetic for average (whole seconds)
    let average = total_intervals / interval_count;
    // Integer arithmetic for guard rate (percentage * 100)
    let guard_rate = guard_total * 10000 / interval_count;
    // Display as XX.XX%
    log::info!(
        "Session summary -> avg {}s across {} intervals | guard {} activations ({}.{:02}/100)",
        average,
        interval_count,
        guard_total,
        guard_rate / 100,
        guard_rate % 100
    );
}

#[cfg(not(feature = "rocksdb-backend"))]
fn mine_continuous(_options: MiningOptions) -> Result<()> {
    error!("ERROR: Continuous mining requires 'rocksdb-backend' feature");
    error!("Rebuild with: cargo build --release --features rocksdb-backend");
    Ok(())
}

/// Generate a wallet keypair with encrypted storage
/// Show wallet address from encrypted keystore
#[allow(dead_code)]
fn address_network_label(network: address::AddressNetwork) -> &'static str {
    match network {
        address::AddressNetwork::Mainnet => "mainnet",
        address::AddressNetwork::Testnet => "testnet",
        address::AddressNetwork::LegacyMainnet => "mainnet (legacy q1)",
    }
}

/// Convert Bech32m address to script hex for mining/balance checks.
/// Validate a Bech32m address and display decoded metadata.
/// Sign a message with encrypted wallet keypair
/// Helper to read password from stdin securely (no echo)
#[allow(dead_code)]
fn read_password_from_stdin() -> Result<String> {
    // SECURITY: Use rpassword to prompt and hide input (no terminal echo)
    // prompt_password handles flushing stdout automatically
    let password = rpassword::prompt_password("Password: ")
        .map_err(|e| Error::Invalid(format!("Failed to read password: {e}")))?;

    if password.is_empty() {
        return Err(Error::Invalid("Password cannot be empty".into()));
    }

    Ok(password)
}

/// Run Stratum mining server.
fn run_stratum_server(
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
    };

    log::info!("Starting BitQuan Stratum Mining Server");
    log::info!(" Bind address: {}", bind_addr);
    log::info!(" Network: {:?}", network);
    log::info!(" Default difficulty: {}", default_difficulty);
    log::info!("");

    // Create runtime for async server
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| Error::Invalid(format!("failed to create tokio runtime: {}", e)))?;

    runtime.block_on(async {
        let mut server = StratumServer::new(config);

        // Start server (blocks until shutdown)
        server.start().await
    })
}
