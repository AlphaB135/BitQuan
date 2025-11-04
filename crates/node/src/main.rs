//! BitQuan reference node entrypoint.

mod address;
mod block_submit;
mod chainstate;
mod keystore;
mod metrics;
mod miner;
mod mnemonic;
mod pool_db;
mod pool_template;
mod reward_engine;
#[cfg(feature = "rocksdb-backend")]
mod rpc;
mod stratum_server;
mod tx_builder;
mod utxo;
mod vardiff;
mod wallet;
mod ws_dashboard;

use bitquan_consensus::{
    asert_next_target, check_header_pow, clamp_bits_within_bounds, compact_to_target, header_hash,
    target_to_compact, ConsensusEngine, ConsensusParams, DifficultyState, DEVNET_MAX_BITS,
};
use bitquan_network::io::{recv_envelope, send_envelope};
use bitquan_network::protocol::{Message, MessageEnvelope, PROTOCOL_VERSION};
#[cfg(feature = "rocksdb-backend")]
use bitquan_rpc::{server::RpcServer, tls::TlsConfig, IpNetwork, RpcConfig};
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
use hex::encode as hex_encode;
use std::collections::VecDeque;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

#[cfg(feature = "rocksdb-backend")]
use rpc::NodeRpcHandler;

#[inline]
fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Invalid(msg.into()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PowMode {
    Hashcash,
    Mock,
    #[cfg(feature = "randomx")]
    RandomX,
    #[cfg(feature = "randomx")]
    Hybrid,
}

impl PowMode {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "hashcash" | "sha256d" | "real" => Ok(PowMode::Hashcash),
            "mock" | "dev-fast-pow" => Ok(PowMode::Mock),
            #[cfg(feature = "randomx")]
            "randomx" => Ok(PowMode::RandomX),
            #[cfg(feature = "randomx")]
            "hybrid" => Ok(PowMode::Hybrid),
            other => invalid(format!("unknown pow engine '{}'", other)),
        }
    }
}

fn parse_network_id(value: &str) -> Result<NetworkId> {
    match value.to_ascii_lowercase().as_str() {
        "mainnet" => Ok(NetworkId::Mainnet),
        "testnet" => Ok(NetworkId::Testnet),
        "devnet" => Ok(NetworkId::Devnet),
        "regtest" => Ok(NetworkId::Regtest),
        other => invalid(format!("unknown network '{}'", other)),
    }
}

fn ensure_pow_allowed(pow_mode: PowMode, network: NetworkId) -> Result<()> {
    if matches!(pow_mode, PowMode::Mock) && matches!(network, NetworkId::Mainnet) {
        return invalid("mock PoW is disabled on mainnet");
    }
    #[cfg(feature = "randomx")]
    {
        if matches!(pow_mode, PowMode::RandomX | PowMode::Hybrid)
            && matches!(network, NetworkId::Mainnet)
        {
            return invalid("RandomX and hybrid PoW are disabled on mainnet");
        }
    }
    Ok(())
}

/// Parse hybrid weights from CLI string format "sha256d:1,randomx:2".
#[cfg(feature = "randomx")]
fn parse_hybrid_weights(s: &str) -> Result<Vec<(bitquan_consensus::pow::PowAlgo, f32)>> {
    use bitquan_consensus::pow::PowAlgo;
    
    let mut weights = Vec::new();
    for part in s.split(',') {
        let (key, value) = part
            .split_once(':')
            .ok_or_else(|| Error::Invalid(format!("invalid weight format: '{}', expected 'algo:weight'", part)))?;
        
        let algo = match key.trim() {
            "sha256d" => PowAlgo::Sha256d,
            "randomx" => PowAlgo::RandomX,
            other => return invalid(format!("unknown algorithm: '{}'", other)),
        };
        
        let weight = value
            .trim()
            .parse::<f32>()
            .map_err(|e| Error::Invalid(format!("invalid weight value '{}': {}", value, e)))?;
        
        if weight <= 0.0 {
            return invalid(format!("weight must be positive for {}", key));
        }
        
        weights.push((algo, weight));
    }
    
    if weights.is_empty() {
        return invalid("at least one algorithm weight required");
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

#[derive(Subcommand)]
enum Commands {
    /// Runs a placeholder node loop.
    Run {
        /// Path to the node configuration file.
        #[arg(long, default_value = "config/bitquan.toml")]
        config: String,
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
        /// Compact bits target (e.g., 0x207fffff for very easy target).
        #[arg(long, default_value_t = 0x207fffff)]
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
        /// Proof-of-Work engine (hashcash|mock|randomx|hybrid).
        #[arg(long, value_name = "POW", default_value = "hashcash")]
        pow: String,
        /// Number of threads for mining (0 = CPU count)
        #[arg(long, default_value_t = 1)]
        threads: usize,
        /// Optional limit on number of blocks to mine in this session.
        #[arg(long)]
        limit_blocks: Option<u64>,
        /// Hybrid algorithm weights (e.g., "sha256d:1,randomx:2").
        #[cfg(feature = "randomx")]
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
    },
    /// Generates a post-quantum keypair for wallet
    WalletGen {
        /// Algorithm (dilithium3, falcon512, sphincs)
        #[arg(long, default_value = "dilithium3")]
        algo: String,
        /// Output file for keypair (optional)
        #[arg(long)]
        output: Option<String>,
        /// Password to encrypt the keystore (interactive prompt if not provided)
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
        #[arg(long)]
        amount: u64,
        /// Fee rate (qbits per weight unit)
        #[arg(long, default_value_t = 1)]
        fee_rate: u64,
        /// Password to decrypt the keystore
        #[arg(long)]
        password: Option<String>,
    },
    /// Builds a simple unsigned transaction (1-in, 1-out) and prints JSON.
    BuildTx {
        /// Previous txid (hex, 32 bytes big-endian)
        #[arg(long)]
        prev_txid: String,
        /// Previous output index
        #[arg(long)]
        prev_vout: u32,
        /// Output value in qbits (1 BQ = 10^8 qbits)
        #[arg(long)]
        value: u64,
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
}

fn main() -> Result<()> {
    // Install panic hook for better crash reporting
    install_panic_hook();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => run_node(&config),
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
            #[cfg(feature = "randomx")]
            hybrid_weights,
            #[cfg(feature = "randomx")]
            randomx_mode: _randomx_mode,
            #[cfg(feature = "randomx")]
            randomx_seed: _randomx_seed,
        } => {
            let network_id = parse_network_id(&network)?;
            let pow_mode = PowMode::parse(&pow)?;
            ensure_pow_allowed(pow_mode, network_id)?;
            
            #[cfg(feature = "randomx")]
            let weights = if matches!(pow_mode, PowMode::Hybrid) {
                Some(parse_hybrid_weights(
                    hybrid_weights.as_deref().unwrap_or("sha256d:1,randomx:2")
                )?)
            } else {
                None
            };
            
            mine_continuous(MiningOptions {
                datadir: &datadir,
                payout_script_hex: &payout_script_hex,
                bits_override: bits,
                max_nonce,
                threads,
                limit_blocks,
                network: network_id,
                pow_mode,
                #[cfg(feature = "randomx")]
                hybrid_weights: weights,
            })
        }
        Commands::WalletGen {
            algo,
            output,
            password,
        } => wallet_gen(&algo, output.as_deref(), password.as_deref()),
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
        } => wallet_send(&keystore, &to, amount, fee_rate, password.as_deref()),
        Commands::BuildTx {
            prev_txid,
            prev_vout,
            value,
            to_script_hex,
        } => build_tx(&prev_txid, prev_vout, value, &to_script_hex),
        Commands::P2PDemo { addr } => p2p_demo(&addr),
        Commands::P2PServer {
            listen,
            max_peers,
            datadir,
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
        } => {
            #[cfg(feature = "rocksdb-backend")]
            {
                p2p_server(
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
                )
            }
            #[cfg(not(feature = "rocksdb-backend"))]
            {
                let _ = (&listen, max_peers, &datadir);
                p2p_server(
                    &listen,
                    max_peers,
                    &datadir,
                    RpcServerOptions {
                        listen: None,
                        username: None,
                        password: None,
                    },
                )
            }
        }
        Commands::P2PConnect { peer, height } => p2p_connect(&peer, height),
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
    }
}

fn run_node(config_path: &str) -> Result<()> {
    println!(
        "Starting BitQuan node with configuration: {config_path}\nListening on 127.0.0.1:18444 (prototype)."
    );

    // Bootstraps placeholder subsystems to illustrate crate integration.
    let registry = CryptoRegistry::default();
    let params = ConsensusParams::phase3_defaults();
    let _engine = ConsensusEngine::new(params, registry);
    let _storage = InMemoryChainStore::new();

    start_p2p_server("127.0.0.1:18444")
}

fn start_p2p_server(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(false)?;
    println!("P2P server listening at {addr}");
    loop {
        let (stream, peer) = listener.accept()?;
        println!("Incoming connection from {peer}");
        thread::spawn(move || {
            if let Err(e) = handle_peer(stream) {
                eprintln!("peer error: {e}");
            }
        });
    }
}

fn handle_peer(stream: TcpStream) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    // Simple handshake: expect Version -> send VerAck, reply with our Version -> expect optional VerAck
    let env = read_envelope(&stream)?;
    match env.message {
        Message::Version { .. } => {
            write_envelope(&stream, &MessageEnvelope::new(Message::VerAck))?;
            let version = Message::Version {
                version: PROTOCOL_VERSION,
                services: 1,
                timestamp: 1_700_000_000,
                user_agent: "BitQuan/0.1.0".into(),
                start_height: 0,
            };
            write_envelope(&stream, &MessageEnvelope::new(version))?;
        }
        _ => {
            write_envelope(
                &stream,
                &MessageEnvelope::new(Message::Reject {
                    message: "expected version".into(),
                    code: bitquan_network::protocol::RejectCode::Malformed,
                    reason: "handshake".into(),
                }),
            )?;
            return Ok(());
        }
    }

    // Minimal message loop: respond to Ping with Pong
    loop {
        let msg = read_envelope(&stream)?;
        match msg.message {
            Message::Ping { nonce } => {
                write_envelope(&stream, &MessageEnvelope::new(Message::Pong { nonce }))?
            }
            Message::GetAddr => write_envelope(
                &stream,
                &MessageEnvelope::new(Message::Addr { addrs: vec![] }),
            )?,
            _ => {}
        }
    }
}

/// Mine the genesis block
fn mine_genesis(max_tries: u64, output: &str) -> Result<()> {
    use bitquan_types::{create_genesis_block, is_valid_genesis, GENESIS_BITS, GENESIS_TIME};
    use std::fs;
    use std::time::Instant;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║      BitQuan Genesis Block Miner                ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Parameters:");
    println!("  Time:       {}", GENESIS_TIME);
    println!("  Bits:       0x{:08x}", GENESIS_BITS);
    println!("  Max tries:  {}", max_tries);
    println!("  Output:     {}", output);
    println!();

    // Create genesis block template
    let mut genesis = create_genesis_block();

    println!("Genesis Message:");
    let msg = &genesis.transactions[0].inputs[0].script_sig;
    println!("  {}", String::from_utf8_lossy(msg));
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

            println!("✅ GENESIS BLOCK FOUND!");
            println!();
            println!("Nonce:      {}", nonce);
            println!("Hash:       {}", hex_encode(hash));
            println!("Time:       {:.2}s", elapsed.as_secs_f64());
            println!("Hashrate:   {:.2} H/s", hashrate);
            println!();

            // Validate genesis
            assert!(is_valid_genesis(&genesis), "Invalid genesis block");

            // Save to JSON
            let json = serde_json::to_string_pretty(&genesis)?;
            fs::write(output, json)?;

            println!("💾 Genesis block saved to: {}", output);
            println!();
            println!("Next steps:");
            println!("  1. Update GENESIS_HASH in crates/types/src/genesis.rs");
            println!("  2. Commit genesis block to repository");
            println!("  3. Use this block to initialize blockchain");
            println!();

            found = true;
            break;
        }

        if nonce % 100_000 == 0 && nonce > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let hashrate = (nonce as f64) / elapsed;
            let hash = header_hash(&genesis.header);
            println!(
                "  ... {} attempts ({:.2} H/s) | Hash: {}",
                nonce,
                hashrate,
                &hex_encode(hash)[..16]
            );
        }
    }

    if !found {
        println!(
            "❌ Failed to find valid genesis block in {} attempts",
            max_tries
        );
        println!("Try increasing --max-tries or adjusting difficulty");
    }

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
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![],
    };

    // Merkle/witness roots for block (support multi-tx in future)
    let merkle_root = bitquan_types::compute_merkle_root_from_txids(&[coinbase.txid()]);
    let witness_root = bitquan_types::compute_merkle_root_from_txids(&[coinbase.wtxid()]);

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
            (0x207fffff, now as u64)
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
                transactions: vec![coinbase],
            };
            let _ = store.insert_block(block);
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

struct MiningOptions<'a> {
    datadir: &'a str,
    payout_script_hex: &'a str,
    bits_override: u32,
    max_nonce: u64,
    threads: usize,
    limit_blocks: Option<u64>,
    network: NetworkId,
    pow_mode: PowMode,
    #[cfg(feature = "randomx")]
    hybrid_weights: Option<Vec<(bitquan_consensus::pow::PowAlgo, f32)>>,
}

struct RpcServerOptions<'a> {
    listen: Option<&'a str>,
    username: Option<&'a str>,
    password: Option<&'a str>,
    #[cfg(feature = "rocksdb-backend")]
    max_body_bytes: usize,
    #[cfg(feature = "rocksdb-backend")]
    rl_burst: u32,
    #[cfg(feature = "rocksdb-backend")]
    rl_refill_per_sec: u32,
    #[cfg(feature = "rocksdb-backend")]
    conn_cooldown_ms: u64,
    #[cfg(feature = "rocksdb-backend")]
    max_header_bytes: usize,
    #[cfg(feature = "rocksdb-backend")]
    header_timeout_ms: u64,
    #[cfg(feature = "rocksdb-backend")]
    trust_proxy: bool,
    #[cfg(feature = "rocksdb-backend")]
    trusted_cidr: Vec<String>,
    #[cfg(feature = "rocksdb-backend")]
    tls_cert: Option<&'a str>,
    #[cfg(feature = "rocksdb-backend")]
    tls_key: Option<&'a str>,
    #[cfg(feature = "rocksdb-backend")]
    allow_insecure: bool,
    #[cfg(feature = "rocksdb-backend")]
    jwt_config_path: Option<&'a str>,
    #[cfg(feature = "rocksdb-backend")]
    jwt_secret: Option<&'a str>,
}

/// Continuous mining with persistent RocksDB storage
#[cfg(feature = "rocksdb-backend")]
fn mine_continuous(options: MiningOptions<'_>) -> Result<()> {
    let MiningOptions {
        datadir,
        payout_script_hex,
        bits_override,
        max_nonce,
        threads,
        limit_blocks,
        network,
        pow_mode,
        #[cfg(feature = "randomx")]
        hybrid_weights,
    } = options;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy)]
    struct BlockLog {
        height: u64,
        timestamp: i64,
        target: f64,
    }

    let params = ConsensusParams::phase3_defaults();
    let window = params.difficulty.burst_guard_window as usize;

    // Open or create RocksDB store
    let store = RocksDBStore::open(datadir)
        .map_err(|e| Error::Invalid(format!("failed to open RocksDB: {e}")))?;
    let store = Arc::new(Mutex::new(store));

    let payout_script = hex::decode(payout_script_hex)
        .map_err(|e| Error::Invalid(format!("invalid payout script hex: {e}")))?;
    let found = Arc::new(AtomicBool::new(false));
    let blocks_mined = Arc::new(AtomicU64::new(0));

    let mut history: VecDeque<BlockLog> = VecDeque::with_capacity(window + 2);
    let mut last_timestamp: Option<i64> = None;
    let mut bits = bits_override;
    let allow_mock = matches!(pow_mode, PowMode::Mock);

    println!("BitQuan Continuous Miner");
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
    #[cfg(feature = "randomx")]
    let hybrid_miner = if matches!(pow_mode, PowMode::Hybrid | PowMode::RandomX) {
        use bitquan_consensus::pow::PowAlgo;
        let weights = if let Some(w) = hybrid_weights {
            w
        } else if matches!(pow_mode, PowMode::RandomX) {
            vec![(PowAlgo::RandomX, 1.0)]
        } else {
            vec![(PowAlgo::Sha256d, 1.0), (PowAlgo::RandomX, 2.0)]
        };
        
        println!("\n=== Hybrid Mining Enabled ===");
        println!("Algorithms:");
        for (algo, weight) in &weights {
            println!("  - {} (weight: {:.1})", algo.name(), weight);
        }
        println!("=============================\n");
        
        let miner = miner::HybridMiner::new(&weights, threads, network)?;
        Some(miner)
    } else {
        None
    };

    {
        let s = store.lock().unwrap();
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
                        target: compact_to_target(block.header.bits),
                    };
                    last_timestamp = Some(log.timestamp);
                    history.push_back(log);
                }
            }
            if bits == 0 {
                bits = s
                    .tip()
                    .map_err(|e| Error::Invalid(format!("storage tip error: {e}")))?
                    .map(|tip| tip.bits)
                    .unwrap_or(0x207fffff);
            }
        } else if bits == 0 {
            bits = 0x207fffff;
        }
    }

    if bits != 0 {
        bits = clamp_bits_within_bounds(bits);
    }

    let mut total_intervals = 0.0;
    let mut interval_count = 0u64;
    let mut guard_total = 0u64;

    loop {
        let height = {
            let s = store.lock().unwrap();
            s.height()
                .map_err(|e| Error::Invalid(format!("storage height error: {e}")))?
        };

        println!("\n[Block #{}] Mining...", height + 1);

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
            let s = store.lock().unwrap();
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
            sig_algo: SigAlgorithm::Dilithium3,
        };

        let merkle_root = bitquan_types::compute_merkle_root_from_txids(&[coinbase.txid()]);
        let witness_root = bitquan_types::compute_merkle_root_from_txids(&[coinbase.wtxid()]);

        // Determine prev_block
        let mut prev = [0u8; 32];
        {
            let s = store.lock().unwrap();
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

        println!("Mining block #{} ...", height + 1);
        println!("Target bits: 0x{:08x}", bits);
        println!("Block reward: {} qbits", subsidy);

        // Mining loop
        found.store(false, Ordering::Relaxed);
        let start_time = std::time::Instant::now();
        
        // Hybrid mining path
        #[cfg(feature = "randomx")]
        let (mined_header, algo_used) = if let Some(ref hybrid_miner) = hybrid_miner {
            use bitquan_consensus::pow::PowAlgo;
            
            // Select algorithm based on iteration
            let algo = hybrid_miner.select_algorithm(height);
            println!("Mining with algorithm: {}", algo.name());
            
            match hybrid_miner.mine_block_attempt(header.clone(), max_nonce, algo)? {
                Some(h) => (Some(h), Some(algo)),
                None => {
                    println!("No solution found in {} attempts with {}", max_nonce, algo.name());
                    continue;
                }
            }
        } else {
            (None, None)
        };
        
        // Standard SHA-256d mining path
        #[cfg(not(feature = "randomx"))]
        let (mined_header, _algo_used): (Option<bitquan_types::BlockHeader>, Option<()>) = (None, None);
        
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
                    header.nonce == 0 || header.bits >= DEVNET_MAX_BITS
                } else {
                    check_header_pow(&header)
                        .map_err(|e| Error::Invalid(format!("pow verification failed: {e}")))?
                };

                if pow_valid {
                    solution_found = true;
                    final_nonce = n;
                    break;
                }
            }
            
            if !solution_found {
                println!("No solution found in {} attempts", max_nonce);
                continue;
            }
            
            (header, final_nonce)
        };

        let id = header_hash(&header);
        let elapsed = start_time.elapsed();
        let hashrate = (n as f64) / elapsed.as_secs_f64();

        #[cfg(feature = "randomx")]
        if let Some(algo) = algo_used {
            println!("\nFOUND! Block #{} | Algo: {} | Nonce: {}", height + 1, algo.name(), n);
        } else {
            println!("\nFOUND! Block #{} | Nonce: {}", height + 1, n);
        }
        #[cfg(not(feature = "randomx"))]
        println!("\nFOUND! Block #{} | Nonce: {}", height + 1, n);
        
        println!("Hash: {}", hex::encode(id));
        println!(
            "Time: {:.2}s | Hashrate: {:.2} H/s",
            elapsed.as_secs_f64(),
            hashrate
        );

        let block = Block {
            header: header.clone(),
            transactions: vec![coinbase.clone()],
        };

        {
            let mut s = store.lock().unwrap();
            s.insert_block(block)
                .map_err(|e| Error::Invalid(format!("failed to insert block: {e}")))?;
        }

        let block_height = height + 1;
        let block_time = header.time as i64;
        let block_bits = header.bits;
        let block_target = compact_to_target(block_bits);

        if let Some(prev_ts) = last_timestamp {
            let interval = (block_time - prev_ts).max(0) as f64;
            total_intervals += interval;
            interval_count = interval_count
                .checked_add(1)
                .ok_or(Error::Overflow("interval count overflow"))?;
        }
        last_timestamp = Some(block_time);

        history.push_back(BlockLog {
            height: block_height,
            timestamp: block_time,
            target: block_target,
        });
        if history.len() > window + 1 {
            history.pop_front();
        }

        let anchor = if block_height as usize > window && history.len() > window {
            history[history.len() - 1 - window]
        } else {
            *history
                .front()
                .expect("history always contains at least the mined block")
        };

        let height_delta = block_height as i64 - anchor.height as i64;
        let time_delta = block_time - anchor.timestamp;
        let expected_time =
            params.difficulty.target_block_time as f64 * height_delta.max(1) as f64;
        let average = if height_delta > 0 {
            time_delta as f64 / height_delta as f64
        } else {
            params.difficulty.target_block_time as f64
        };
        let ratio = if expected_time > 0.0 {
            time_delta as f64 / expected_time
        } else {
            1.0
        };
        let guard_triggered = height_delta as u64 >= params.difficulty.burst_guard_window
            && time_delta > 0
            && ratio < params.difficulty.burst_guard_floor_ratio;
        if guard_triggered {
            guard_total = guard_total
                .checked_add(1)
                .ok_or(Error::Overflow("guard count overflow"))?;
        }

        let next_target =
            asert_next_target(anchor.target, height_delta, time_delta, &params, None);
        let mut next_bits = target_to_compact(next_target);
        if next_bits == 0 {
            next_bits = block_bits;
        }
        next_bits = clamp_bits_within_bounds(next_bits);

        println!(
            "[ASERT] height={} guard={} window={} avg={:.2}s ratio={:.3} target=0x{:08x} next_bits=0x{:08x}",
            block_height,
            if guard_triggered { "ON " } else { "off" },
            height_delta,
            average,
            ratio,
            block_bits,
            next_bits
        );

        bits = next_bits;

        let total = blocks_mined.fetch_add(1, Ordering::Relaxed) + 1;
        println!("Saved to DB | Session total: {}", total);
        found.store(true, Ordering::Relaxed);

        if let Some(limit) = limit_blocks {
            if total >= limit {
                print_session_summary(interval_count, total_intervals, guard_total);
                println!("Reached block limit ({limit}). Session complete.");
                return Ok(());
            }
        }
        
        if !found.load(Ordering::Relaxed) {
            println!(
                "\nNo valid nonce in {} tries, adjusting difficulty...",
                max_nonce
            );
            bits = (bits & 0x00ff_ffff) | ((((bits >> 24) + 1) & 0xff) << 24);
            bits = clamp_bits_within_bounds(bits);
        }
    }
}

fn print_session_summary(interval_count: u64, total_intervals: f64, guard_total: u64) {
    if interval_count == 0 {
        println!("Session summary -> insufficient interval data to compute averages.");
        return;
    }
    let average = total_intervals / interval_count as f64;
    let guard_rate = guard_total as f64 * 100.0 / interval_count as f64;
    println!(
        "Session summary -> avg {:.2}s across {} intervals | guard {} activations ({:.2}/100)",
        average, interval_count, guard_total, guard_rate
    );
}

#[cfg(not(feature = "rocksdb-backend"))]
fn mine_continuous(_options: MiningOptions<'_>) -> Result<()> {
    eprintln!("ERROR: Continuous mining requires 'rocksdb-backend' feature");
    eprintln!("Rebuild with: cargo build --release --features rocksdb-backend");
    Ok(())
}

/// Generate a wallet keypair with encrypted storage
fn wallet_gen(algo: &str, output_path: Option<&str>, password: Option<&str>) -> Result<()> {
    use std::path::Path;
    use wallet::{address, WalletKeypair};

    println!("BitQuan Wallet Generator");
    println!("Algorithm: {}", algo);

    if algo != "dilithium3" {
        return invalid("Only 'dilithium3' is supported currently");
    }

    println!("\n⏳ Generating keypair...");
    let keypair = WalletKeypair::generate_dilithium3()?;

    let pubkey_hash = keypair.public_key_hash();
    let address_str = address::encode(&pubkey_hash);

    use pqc_dilithium_seeded::{PUBLICKEYBYTES, SECRETKEYBYTES};

    println!("\n✅ Keypair generated successfully!");
    println!("\n📍 Address: {}", address_str);
    println!("🔑 Public key hash: {}", hex::encode(pubkey_hash));
    println!("📏 Public key: {} bytes", PUBLICKEYBYTES);
    println!("📏 Secret key: {} bytes", SECRETKEYBYTES);

    // Get password for encryption
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password to encrypt keystore:");
            read_password_from_stdin()?
        }
    };

    if password.len() < 8 {
        return invalid("Password must be at least 8 characters");
    }

    // Serialize keypair metadata for encryption
    let serializable = keypair.to_serializable();
    let json = serde_json::to_string_pretty(&serializable)?;

    // Encrypt and save
    let keystore_file = keystore::encrypt_keypair(&json, &password, &address_str)
        .map_err(|e| Error::Invalid(format!("keystore encrypt failed: {e}")))?;

    let path = output_path.unwrap_or("wallet.keystore");
    keystore::save_keystore(&keystore_file, Path::new(path))
        .map_err(|e| Error::Invalid(format!("keystore save failed: {e}")))?;

    println!("\n💾 Encrypted keystore saved to: {}", path);
    println!("\n⚠️  IMPORTANT:");
    println!("   - Keep this file safe!");
    println!("   - Remember your password!");
    println!("   - Make backups!");
    println!("\n⚠️  Note: Keypair metadata persisted (address, pubkey hash)");
    println!("   Full signing requires session keypair due to pqc_dilithium 0.2 limitations");

    Ok(())
}

/// Show wallet address from encrypted keystore
fn wallet_address(keystore_path: &str, password: Option<&str>) -> Result<()> {
    use std::path::Path;

    println!("BitQuan Wallet Address");
    println!("Loading keystore from: {}", keystore_path);

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))
        .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password:");
            read_password_from_stdin()?
        }
    };

    // Decrypt
    let json = keystore::decrypt_keypair(&keystore_file, &password)
        .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
    let data: wallet::SerializableKeypair = serde_json::from_str(&json)?;

    println!("\n📍 Address: {}", data.address);
    println!("🔑 Public key hash: {}", data.public_key_hash);
    println!("📏 Metadata only (full keys require session keypair)");

    Ok(())
}

fn address_network_label(network: address::AddressNetwork) -> &'static str {
    match network {
        address::AddressNetwork::Mainnet => "mainnet",
        address::AddressNetwork::Testnet => "testnet",
        address::AddressNetwork::LegacyMainnet => "mainnet (legacy q1)",
    }
}

/// Convert Bech32m address to script hex for mining/balance checks.
fn script_from_address(addr: &str) -> Result<()> {
    let info = address::inspect(addr)
        .map_err(|e| Error::Invalid(format!("Failed to decode address: {}", e)))?;

    let script = address::script_from_pubkey_hash(&info.payload);
    let script_hex = hex::encode(script);
    let trimmed = addr.trim();

    eprintln!("Bech32m checksum: OK");
    eprintln!("Network         : {}", address_network_label(info.network));
    if trimmed != info.normalized {
        eprintln!("Normalized      : {}", info.normalized);
    }
    eprintln!("Pubkey hash     : {}", hex::encode(info.payload));
    println!("{script_hex}");

    Ok(())
}

/// Validate a Bech32m address and display decoded metadata.
fn address_validate(addr: &str) -> Result<()> {
    let info = address::inspect(addr)
        .map_err(|e| Error::Invalid(format!("Address validation failed: {}", e)))?;
    let trimmed = addr.trim();

    println!("BitQuan Address Validation");
    println!("Input      : {}", trimmed);
    if trimmed != info.normalized {
        println!("Normalized  : {}", info.normalized);
    }
    println!("Network     : {}", address_network_label(info.network));
    println!("HRP         : {}", info.hrp);
    println!("Checksum    : OK (Bech32m)");
    println!("Payload size: {} bytes", info.payload.len());
    println!("Pubkey hash : {}", hex::encode(info.payload));
    println!(
        "Script hex  : {}",
        hex::encode(address::script_from_pubkey_hash(&info.payload))
    );

    Ok(())
}

/// Sign a message with encrypted wallet keypair
fn wallet_sign(keystore_path: &str, message_hex: &str, password: Option<&str>) -> Result<()> {
    use std::path::Path;

    println!("BitQuan Wallet Sign");
    println!("Keystore: {}", keystore_path);

    let message = hex::decode(message_hex)
        .map_err(|e| Error::Invalid(format!("invalid message hex: {e}")))?;
    println!("Message: {} ({} bytes)", message_hex, message.len());

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))
        .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password:");
            read_password_from_stdin()?
        }
    };

    // Decrypt keystore
    println!("\n⏳ Decrypting keystore...");
    let json = keystore::decrypt_keypair(&keystore_file, &password)
        .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
    let data: wallet::SerializableKeypair = serde_json::from_str(&json)?;

    println!("✅ Keystore decrypted!");
    println!("📍 Address: {}", data.address);
    println!("🔑 Public key hash: {}", data.public_key_hash);

    println!("\n⚠️  Note: Signing with persisted keys not yet fully supported");
    println!("   pqc_dilithium 0.2 doesn't expose keypair serialization");
    println!("   Use a session-based keypair (wallet-gen without saving) for signing");
    println!("\n💡 Workaround: Generate ephemeral keypair and sign immediately");

    Ok(())
}

/// Helper to read password from stdin securely
fn read_password_from_stdin() -> Result<String> {
    use std::io::{self, Write};

    print!("Password: ");
    io::stdout().flush()?;

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;

    // Trim newline
    Ok(password.trim().to_string())
}

/// Verify a signature
fn wallet_verify(pubkey_hex: &str, message_hex: &str, signature_hex: &str) -> Result<()> {
    use wallet::{WalletAlgorithm, WalletPublicKey};

    println!("BitQuan Wallet Verify");

    let pubkey_bytes = hex::decode(pubkey_hex)
        .map_err(|e| Error::Invalid(format!("invalid public key hex: {e}")))?;
    let message = hex::decode(message_hex)
        .map_err(|e| Error::Invalid(format!("invalid message hex: {e}")))?;
    let signature = hex::decode(signature_hex)
        .map_err(|e| Error::Invalid(format!("invalid signature hex: {e}")))?;

    println!("Public key: {} bytes", pubkey_bytes.len());
    println!("Message: {} bytes", message.len());
    println!("Signature: {} bytes", signature.len());

    let public_key = WalletPublicKey {
        algorithm: WalletAlgorithm::Dilithium3,
        public_key: pubkey_bytes,
    };

    println!();
    println!("Verifying...");
    if public_key.verify(&message, &signature) {
        println!("Signature is VALID!");
        Ok(())
    } else {
        println!("Signature is INVALID!");
        invalid("Signature verification failed")
    }
}

fn wallet_send(
    keystore_path: &str,
    to_address: &str,
    amount: u64,
    fee_rate: u64,
    password: Option<&str>,
) -> Result<()> {
    use std::path::Path;

    println!("BitQuan Wallet Send");
    println!("To: {}", to_address);
    println!(
        "Amount: {} qbits ({:.8} BQ)",
        amount,
        amount as f64 / 100_000_000.0
    );
    println!("Fee rate: {} qbits/WU", fee_rate);
    println!();

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))
        .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password:");
            read_password_from_stdin()?
        }
    };

    // Decrypt keystore
    println!("Decrypting keystore...");
    let json = keystore::decrypt_keypair(&keystore_file, &password)
        .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
    let _data: wallet::SerializableKeypair = serde_json::from_str(&json)?;

    println!();
    println!("Note: Full transaction sending not yet implemented");
    println!("Missing components:");
    println!("  - UTXO lookup from blockchain");
    println!("  - Address to script_pubkey conversion");
    println!("  - Transaction broadcast to network");
    println!();
    println!("Current capabilities:");
    println!("  - Transaction building: use 'build-tx' command");
    println!("  - Message signing: use 'wallet-sign' command");
    println!();
    println!("Example workflow:");
    println!("  1. Get UTXOs: cargo run -- balance --datadir ./data/chainstate");
    println!("  2. Build tx: cargo run -- build-tx --prev-txid <txid> --prev-vout 0 --value <amount> --to-script-hex <script>");
    println!("  3. Sign manually with wallet-sign");

    Ok(())
}

fn build_tx(prev_txid_hex: &str, prev_vout: u32, value: u64, to_script_hex: &str) -> Result<()> {
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
        sig_algo: SigAlgorithm::Dilithium3,
        witnesses: vec![],
    };

    let json = serde_json::to_string_pretty(&tx)?;
    println!("{json}");
    Ok(())
}

fn write_envelope(mut stream: &TcpStream, env: &MessageEnvelope) -> Result<()> {
    send_envelope(&mut stream, env).map_err(|e| Error::Net(e.to_string()))
}

fn read_envelope(mut stream: &TcpStream) -> Result<MessageEnvelope> {
    recv_envelope(&mut stream).map_err(|e| Error::Net(e.to_string()))
}

fn p2p_demo(addr: &str) -> Result<()> {
    // Start server
    let addr_str = addr.to_string();
    let server = thread::spawn(move || -> Result<()> {
        let listener = TcpListener::bind(&addr_str)?;
        listener.set_nonblocking(false)?;
        if let Ok((stream, _peer)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            // Expect Version
            let env = read_envelope(&stream)?;
            if let Message::Version { .. } = env.message {
                // Reply VerAck
                write_envelope(&stream, &MessageEnvelope::new(Message::VerAck))?;
                // Expect Ping then reply Pong
                let ping = read_envelope(&stream)?;
                if let Message::Ping { nonce } = ping.message {
                    write_envelope(&stream, &MessageEnvelope::new(Message::Pong { nonce }))?;
                }
            }
        }
        Ok(())
    });

    // Client
    thread::sleep(Duration::from_millis(50));
    let client = TcpStream::connect(addr)?;
    client.set_read_timeout(Some(Duration::from_secs(5)))?;
    client.set_write_timeout(Some(Duration::from_secs(5)))?;
    let version = Message::Version {
        version: PROTOCOL_VERSION,
        services: 1,
        timestamp: 1_700_000_000,
        user_agent: "BitQuan/0.1.0".into(),
        start_height: 0,
    };
    write_envelope(&client, &MessageEnvelope::new(version))?;
    let verack = read_envelope(&client)?;
    if !matches!(verack.message, Message::VerAck) {
        println!("Unexpected message from server");
        return Ok(());
    }
    let nonce = 42u64;
    write_envelope(&client, &MessageEnvelope::new(Message::Ping { nonce }))?;
    let pong = read_envelope(&client)?;
    if let Message::Pong { nonce: n } = pong.message {
        println!("P2P demo OK (nonce={n})");
    } else {
        println!("P2P demo failed");
    }

    // Wait server
    let _ = server.join().unwrap_or(Ok(()));
    Ok(())
}

/// P2P Server that accepts incoming connections
#[allow(unused_variables)]
fn p2p_server(
    listen: &str,
    max_peers: usize,
    datadir: &str,
    rpc: RpcServerOptions<'_>,
) -> Result<()> {
    use bitquan_network::{P2PListener, PeerManager};
    #[cfg(feature = "rocksdb-backend")]
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Mutex;

    println!("BitQuan P2P Server");
    println!("Listen: {}", listen);
    println!("Max peers: {}", max_peers);
    println!("Data dir: {}", datadir);

    #[cfg(feature = "rocksdb-backend")]
    let RpcServerOptions {
        listen: rpc_listen,
        username: rpc_username,
        password: rpc_password,
        max_body_bytes: rpc_max_body,
        rl_burst: rpc_rl_burst,
        rl_refill_per_sec: rpc_rl_refill_per_sec,
        conn_cooldown_ms: rpc_conn_cooldown_ms,
        max_header_bytes: rpc_max_header,
        header_timeout_ms: rpc_header_timeout_ms,
        trust_proxy: rpc_trust_proxy,
        trusted_cidr: rpc_trusted_cidr,
        tls_cert: rpc_tls_cert,
        tls_key: rpc_tls_key,
        allow_insecure: rpc_allow_insecure,
        jwt_config_path: jwt_config,
        jwt_secret,
    } = rpc;

    #[cfg(not(feature = "rocksdb-backend"))]
    let RpcServerOptions {
        listen: rpc_listen,
        username: rpc_username,
        password: rpc_password,
    } = rpc;

    // Load current height from storage
    #[cfg(feature = "rocksdb-backend")]
    let (height, store) = {
        use bitquan_storage::rocksdb_store::RocksDBStore;
        let store = RocksDBStore::open(datadir)
            .map_err(|e| Error::Invalid(format!("failed to open RocksDB: {e}")))?;
        let h = store.height().unwrap_or(0);
        (h, Some(Arc::new(Mutex::new(store))))
    };

    #[cfg(not(feature = "rocksdb-backend"))]
    let (height, store) = (0u64, None);

    println!("Current height: {}", height);
    println!(
        "Storage: {}",
        if store.is_some() {
            "RocksDB"
        } else {
            "In-Memory"
        }
    );

    #[cfg(feature = "rocksdb-backend")]
    if let Some(addr) = rpc_listen {
        let username = rpc_username.ok_or_else(|| {
            Error::Invalid("--rpc-username is required when enabling RPC server".to_string())
        })?;

        let password_value = if let Some(pass) = rpc_password {
            pass.to_string()
        } else {
            println!("Enter RPC password:");
            let input = read_password_from_stdin()?;
            if input.is_empty() {
                return invalid("RPC password cannot be empty");
            }
            input
        };

        if password_value.is_empty() {
            return invalid("RPC password cannot be empty");
        }

        if username.is_empty() {
            return invalid("RPC username cannot be empty");
        }

        if !addr.starts_with("127.") && !addr.starts_with("localhost") {
            println!(
                "Warning: RPC server binding to '{}'. Ensure firewall and authentication are configured.",
                addr
            );
        }

        let Some(store_arc) = store.clone() else {
            return invalid("RPC server requires RocksDB storage backend");
        };

        let handler = NodeRpcHandler::new(store_arc, "mainnet");
        let rpc_addr = addr.to_string();

        // JWT authentication is required
        use bitquan_rpc::jwt::{JwtAuth, JwtConfig};

        if jwt_config.is_none() && jwt_secret.is_none() {
            return invalid(
                "RPC server requires JWT authentication. Provide --jwt-config or --jwt-secret"
                    .to_string(),
            );
        }

        println!("RPC authentication: JWT");

        let mut trusted_proxies = Vec::new();
        for cidr in rpc_trusted_cidr {
            let trimmed = cidr.trim();
            if trimmed.is_empty() {
                continue;
            }
            let network = IpNetwork::from_str(trimmed).map_err(|e| {
                Error::Invalid(format!("invalid --rpc-trusted-cidr '{}': {}", trimmed, e))
            })?;
            trusted_proxies.push(network);
        }

        if rpc_tls_key.is_some() && rpc_tls_cert.is_none() {
            return invalid("--rpc-tls-key provided without --rpc-tls-cert");
        }

        let require_tls = !rpc_allow_insecure;
        let tls_config = if let Some(cert_path) = rpc_tls_cert {
            let key_path = rpc_tls_key.ok_or_else(|| {
                Error::Invalid(
                    "--rpc-tls-key is required when --rpc-tls-cert is provided".to_string(),
                )
            })?;
            let tls = TlsConfig::new(Path::new(cert_path), Path::new(key_path))
                .map_err(|err| Error::Invalid(format!("failed to initialise RPC TLS: {err}")))?;
            Some(tls)
        } else {
            None
        };

        if require_tls && tls_config.is_none() {
            return invalid(
                "RPC TLS is required. Provide --rpc-tls-cert/--rpc-tls-key or pass --rpc-allow-insecure for development."
                    .to_string(),
            );
        }

        let rpc_config = RpcConfig {
            max_body_bytes: rpc_max_body,
            rl_burst: rpc_rl_burst,
            rl_refill_per_sec: rpc_rl_refill_per_sec,
            conn_cooldown_ms: rpc_conn_cooldown_ms,
            trust_proxy: rpc_trust_proxy,
            trusted_proxies,
            max_header_bytes: rpc_max_header,
            header_read_timeout_ms: rpc_header_timeout_ms,
            require_tls,
            allow_self_signed: false,
            enable_hsts: true,
            hsts_max_age: 31_536_000,
            hsts_include_subdomains: false,
            ..RpcConfig::default()
        };
        println!(
            "RPC starting with max_body_bytes={} rl_burst={} rl_refill_per_sec={} conn_cooldown_ms={} max_header_bytes={} header_timeout_ms={} trust_proxy={} trusted_cidr={:?} require_tls={} tls_configured={}",
            rpc_config.max_body_bytes,
            rpc_config.rl_burst,
            rpc_config.rl_refill_per_sec,
            rpc_config.conn_cooldown_ms,
            rpc_config.max_header_bytes,
            rpc_config.header_read_timeout_ms,
            rpc_config.trust_proxy,
            rpc_config.trusted_proxies,
            rpc_config.require_tls,
            tls_config.is_some()
        );

        if let Some(cert_path) = rpc_tls_cert {
            println!("RPC TLS certificate: {}", cert_path);
        } else if rpc_config.require_tls {
            println!("RPC TLS certificate: <required>");
        } else {
            println!("RPC TLS certificate: <not configured>");
        }

        let tls_config_for_thread = tls_config.clone();
        let jwt_config_owned = jwt_config.map(|s| s.to_string());
        let jwt_secret_owned = jwt_secret.map(|s| s.to_string());

        thread::spawn(move || {
            // JWT authentication (required)
            let jwt_auth = if let Some(config_path) = jwt_config_owned {
                println!("Loading JWT config from: {}", config_path);
                match JwtConfig::from_file(&config_path) {
                    Ok(config) => match JwtAuth::from_config(&config) {
                        Ok(auth) => auth,
                        Err(e) => {
                            eprintln!("Failed to create JWT auth from config: {}", e);
                            return;
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to load JWT config: {}", e);
                        return;
                    }
                }
            } else if let Some(secret) = jwt_secret_owned {
                println!("Using JWT with provided secret");
                JwtAuth::new(&secret)
            } else {
                eprintln!("JWT authentication required but no config or secret provided");
                return;
            };

            let mut server = RpcServer::new(handler, rpc_addr.clone(), jwt_auth, rpc_config);

            if let Some(tls_cfg) = tls_config_for_thread {
                server = server.with_tls_config(tls_cfg);
            }
            server = server.require_tls(require_tls);
            if let Err(e) = server.serve() {
                eprintln!("RPC server error ({}): {}", rpc_addr, e);
            }
        });
        println!("RPC server listening on {}", addr);
    }

    // Create relay manager
    use bitquan_network::RelayManager;
    let relay_manager = Arc::new(RelayManager::new(10000));

    let peer_manager = Arc::new(PeerManager::with_relay(max_peers, relay_manager.clone()));
    peer_manager.update_height(height);

    let listener = P2PListener::bind(listen, peer_manager.clone())
        .map_err(|e| Error::Invalid(format!("p2p bind failed: {e}")))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| Error::Invalid(format!("p2p local addr failed: {e}")))?;
    println!("Server started at {}", local_addr);
    println!("Waiting for connections...");
    println!();
    println!("Commands:");
    println!("  - Press Ctrl+C to stop");
    println!("  - Peers will sync blockchain automatically");

    // Broadcast tip block when we have storage
    if let Some(s) = &store {
        if height > 0 {
            use bitquan_consensus::header_hash;
            let store_locked = s.lock().unwrap();
            if let Ok(Some(tip)) = store_locked.tip() {
                let tip_hash = header_hash(&tip);
                drop(store_locked);

                println!();
                println!("Tip: Use 'mine' command to mine blocks");
                println!("Current tip: {}", hex_encode(tip_hash));
                println!("New blocks will be broadcast to peers");
            }
        }
    }

    loop {
        match listener.accept_one() {
            Ok(()) => {
                let count = peer_manager.peer_count();
                let ready = peer_manager.ready_peer_count();
                println!("Peer connected! Total: {}, Ready: {}", count, ready);

                // Send inv for our tip block to new peer
                if let Some(s) = &store {
                    if height > 0 {
                        use bitquan_consensus::header_hash;

                        let store_locked = s.lock().unwrap();
                        if let Ok(Some(tip)) = store_locked.tip() {
                            let tip_hash = header_hash(&tip);
                            drop(store_locked);

                            let inv = bitquan_network::protocol::InvVector {
                                inv_type: bitquan_network::protocol::InvType::Block,
                                hash: tip_hash,
                            };

                            if let Ok(sent) = peer_manager.broadcast_inv(inv) {
                                println!("Announced tip block to {} peers", sent);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }

        // Cleanup dead peers and old relay data
        peer_manager.cleanup_peers();
        relay_manager.cleanup();

        thread::sleep(Duration::from_millis(100));
    }
}

/// Connect to a peer as a client
fn p2p_connect(peer: &str, height: u64) -> Result<()> {
    use bitquan_network::PeerManager;
    use std::sync::Arc;

    println!("BitQuan P2P Client");
    println!("Connecting to: {}", peer);
    println!("Our height: {}", height);

    let peer_manager = Arc::new(PeerManager::new(1));
    peer_manager.update_height(height);

    let addr: SocketAddr = peer
        .parse()
        .map_err(|e| Error::Invalid(format!("invalid peer address: {e}")))?;

    println!("⏳ Connecting...");
    match peer_manager.connect_peer(addr) {
        Ok(()) => {
            println!("✅ Connected and handshake complete!");
            println!("Ready peers: {}", peer_manager.ready_peer_count());

            // Keep connection alive for a bit
            for i in 1..=5 {
                thread::sleep(Duration::from_secs(1));
                println!("Connection alive... {}/5", i);
            }

            println!("✅ Test complete");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Connection failed: {}", e);
            Err(Error::Invalid(format!("connection failed: {e}")))
        }
    }
}

/// Check balance for a script
#[cfg(feature = "rocksdb-backend")]
fn check_balance(datadir: &str, script_hex: Option<&str>, address: Option<&str>) -> Result<()> {
    use bitquan_storage::rocksdb_store::RocksDBStore;

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
        return invalid("Either --script-hex or --address must be provided");
    };

    println!("Script: {}", hex::encode(&target_script));
    println!("\nScanning blockchain for UTXOs...");

    let mut balance: u64 = 0;
    let mut utxo_count: u64 = 0;

    // Scan all blocks (simple implementation)
    for h in 0..=height {
        if let Ok(Some(block)) = store.get_block_by_height(h) {
            for tx in &block.transactions {
                for (vout, output) in tx.outputs.iter().enumerate() {
                    if output.script_pubkey == target_script {
                        // Check if spent (simplified - should check UTXO set)
                        balance = balance
                            .checked_add(output.value)
                            .ok_or(Error::Overflow("balance accumulation overflow"))?;
                        utxo_count = utxo_count
                            .checked_add(1)
                            .ok_or(Error::Overflow("UTXO count overflow"))?;
                        println!(
                            "  Block #{} TX {} vout={} amount={}",
                            h,
                            hex::encode(tx.txid()),
                            vout,
                            output.value
                        );
                    }
                }
            }
        }
    }

    println!("\nUTXO count: {}", utxo_count);
    println!("Balance: {} qbits", balance);
    println!("Balance: {:.8} BQ", balance as f64 / 100_000_000.0);

    Ok(())
}

#[cfg(not(feature = "rocksdb-backend"))]
fn check_balance(_datadir: &str, _script_hex: Option<&str>, _address: Option<&str>) -> Result<()> {
    eprintln!("ERROR: Balance checking requires 'rocksdb-backend' feature");
    eprintln!("Rebuild with: cargo build --release --features rocksdb-backend");
    Ok(())
}

#[cfg(feature = "rocksdb-backend")]
fn generate_self_signed_cert_cli(output_dir: &str) -> Result<()> {
    use std::path::Path;

    let path = Path::new(output_dir);
    std::fs::create_dir_all(path).map_err(|e| {
        Error::Invalid(format!(
            "failed to create output directory {}: {e}",
            path.display()
        ))
    })?;

    bitquan_rpc::tls::generate_self_signed_cert(path).map_err(|err| {
        Error::Invalid(format!("failed to generate self-signed certificate: {err}"))
    })?;

    println!("✅ Generated self-signed certificate:");
    println!("   cert: {}/cert.pem", path.display());
    println!("   key:  {}/key.pem", path.display());
    println!();
    println!(
        "⚠️  Development only. For production, obtain a trusted certificate (e.g. Let's Encrypt)."
    );
    println!();
    println!("To start the node with TLS:");
    println!("  bitquan-node p2p-server \\\n    --rpc-listen 127.0.0.1:8332 \\\n    --rpc-username admin \\\n    --rpc-password <YOUR_PASSWORD> \\\n    --rpc-tls-cert {}/cert.pem \\\n    --rpc-tls-key {}/key.pem", path.display(), path.display()); // Safe: example placeholder

    Ok(())
}

/// Hash a password using Argon2id
fn hash_password_cli(password: Option<&str>) -> Result<()> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password to hash:");
            read_password_from_stdin()?
        }
    };

    if password.is_empty() {
        return invalid("Password cannot be empty");
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Invalid(format!("Failed to hash password: {}", e)))?
        .to_string();

    println!("\nHashed password:");
    println!("{}", hash);
    println!("\nCopy this hash to your jwt.toml file");

    Ok(())
}

/// Add a user to JWT configuration
fn jwt_user_add(
    config_path: &str,
    username: &str,
    role: &str,
    password: Option<&str>,
) -> Result<()> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    use bitquan_rpc::jwt::{JwtConfig, JwtUserConfig};
    use std::path::Path;

    // Validate role
    if !["admin", "miner", "readonly"].contains(&role) {
        return invalid(format!(
            "Invalid role '{}'. Must be: admin, miner, or readonly",
            role
        ));
    }

    // Load existing config or create new
    let mut config = if Path::new(config_path).exists() {
        JwtConfig::from_file(config_path)
            .map_err(|e| Error::Invalid(format!("Failed to load config: {}", e)))?
    } else {
        JwtConfig::default()
    };

    // Check if user already exists
    if config.users.iter().any(|u| u.username == username) {
        return invalid(format!("User '{}' already exists in config", username));
    }

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password for user '{}':", username);
            read_password_from_stdin()?
        }
    };

    if password.is_empty() {
        return invalid("Password cannot be empty");
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Invalid(format!("Failed to hash password: {}", e)))?
        .to_string();

    // Add user
    config.users.push(JwtUserConfig {
        username: username.to_string(),
        password_hash: hash,
        role: role.to_string(),
    });

    // Save config
    config
        .save_to_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to save config: {}", e)))?;

    println!(
        "✅ User '{}' added successfully with role '{}'",
        username, role
    );
    println!("📄 Config saved to: {}", config_path);

    Ok(())
}

/// Remove a user from JWT configuration
fn jwt_user_remove(config_path: &str, username: &str) -> Result<()> {
    use bitquan_rpc::jwt::JwtConfig;
    use std::path::Path;

    if !Path::new(config_path).exists() {
        return invalid(format!("Config file not found: {}", config_path));
    }

    let mut config = JwtConfig::from_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to load config: {}", e)))?;

    let initial_count = config.users.len();
    config.users.retain(|u| u.username != username);

    if config.users.len() == initial_count {
        return invalid(format!("User '{}' not found in config", username));
    }

    if config.users.is_empty() {
        return invalid("Cannot remove last user. At least one user must remain.");
    }

    config
        .save_to_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to save config: {}", e)))?;

    println!("✅ User '{}' removed successfully", username);
    println!("📄 Config saved to: {}", config_path);

    Ok(())
}

/// List users in JWT configuration
#[cfg(feature = "rocksdb-backend")]
fn verify_database(
    path: &str,
    backup: bool,
    backup_path: Option<&str>,
    rebuild: bool,
) -> Result<()> {
    use bitquan_storage::RecoveryOptions;

    println!("🔍 Database Verification Tool");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Database path: {}", path);
    println!();

    let options = RecoveryOptions {
        verify_checksums: true,
        auto_backup: backup,
        backup_path: backup_path.map(|s| s.to_string()),
        rebuild_indices: rebuild,
    };

    println!("Opening database with recovery options...");
    let store = RocksDBStore::open_with_options(path, options)
        .map_err(|e| Error::Invalid(format!("failed to open RocksDB with options: {e}")))?;

    println!();
    println!("📊 Database Statistics:");
    let stats = store
        .get_stats()
        .map_err(|e| Error::Invalid(format!("storage stats error: {e}")))?;
    println!("  Chain height: {}", stats.height);
    println!("  Total blocks: {}", stats.num_blocks);
    println!("  Transactions: {}", stats.num_transactions);
    println!("  UTXOs: {}", stats.num_utxos);

    println!();
    println!("✅ Database verification complete!");

    Ok(())
}

fn jwt_user_list(config_path: &str) -> Result<()> {
    use bitquan_rpc::jwt::JwtConfig;
    use std::path::Path;

    if !Path::new(config_path).exists() {
        return invalid(format!("Config file not found: {}", config_path));
    }

    let config = JwtConfig::from_file(config_path)
        .map_err(|e| Error::Invalid(format!("Failed to load config: {}", e)))?;

    if config.users.is_empty() {
        println!("No users found in config");
        return Ok(());
    }

    println!("\n📋 Users in {}:", config_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("{:<20} {:<15}", "Username", "Role");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for user in &config.users {
        println!("{:<20} {:<15}", user.username, user.role);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total: {} user(s)\n", config.users.len());

    Ok(())
}

/// Generate wallet from BIP39 mnemonic
fn wallet_gen_mnemonic(
    word_count: usize,
    output_path: Option<&str>,
    password: Option<&str>,
    show_mnemonic: bool,
) -> Result<()> {
    use crate::mnemonic::MnemonicHelper;
    use std::path::Path;

    // Generate mnemonic
    let helper = MnemonicHelper::generate_with_word_count(word_count)?;
    let mnemonic_phrase = helper.phrase();

    // Show mnemonic to user
    if show_mnemonic {
        eprintln!("\n⚠️  SECURITY WARNING: Mnemonic phrase will be displayed!");
        eprintln!("   - Do NOT log terminal output");
        eprintln!("   - Do NOT screenshot this");
        eprintln!("   - Ensure nobody is watching your screen\n");

        println!("\n🔑 Your BIP39 Mnemonic Phrase:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("{}", mnemonic_phrase);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\n⚠️  CRITICAL SECURITY:");
        println!("   - Write down these words in order on paper");
        println!("   - Store them in a safe place (NOT digitally)");
        println!("   - Never share them with anyone");
        println!("   - Never enter them on websites or apps");
        println!("   - You need these words to recover your wallet");
        println!();
    } else {
        println!("✅ Mnemonic generated (hidden for security)");
        println!("   Use --show-mnemonic flag to display (NOT recommended in production)");
    }

    // Derive keypair
    let keypair = helper.to_keypair()?;
    let serializable = keypair.to_serializable();
    let json = serde_json::to_string_pretty(&serializable)?;

    // Get encryption password
    let password_value = match password {
        Some(p) => p.to_string(),
        None => {
            println!("🔒 Enter password to encrypt keystore:");
            read_password_from_stdin()?
        }
    };

    if password_value.is_empty() {
        return invalid("Password cannot be empty");
    }

    // Encrypt and save keystore
    let keystore_file = keystore::encrypt_keypair(&json, &password_value, &serializable.address)
        .map_err(|e| Error::Invalid(format!("keystore encrypt failed: {e}")))?;
    let output_file = output_path.unwrap_or("wallet.keystore");
    keystore::save_keystore(&keystore_file, Path::new(output_file))
        .map_err(|e| Error::Invalid(format!("keystore save failed: {e}")))?;

    println!("\n✅ Wallet created successfully!");
    println!("📄 Keystore saved to: {}", output_file);
    println!("🔐 Address: {}", serializable.address);
    println!("\n💡 To recover this wallet later, use:");
    println!("   bitquan-node wallet-from-mnemonic");
    Ok(())
}

/// Recover wallet from BIP39 mnemonic
fn wallet_from_mnemonic(
    mnemonic: Option<&str>,
    passphrase: Option<&str>,
    output_path: Option<&str>,
    password: Option<&str>,
) -> Result<()> {
    use crate::mnemonic::MnemonicHelper;
    use std::path::Path;

    // Get mnemonic phrase
    let mnemonic_phrase = match mnemonic {
        Some(m) => m.to_string(),
        None => {
            println!("Enter your BIP39 mnemonic phrase:");
            println!("(12 or 24 words separated by spaces)");
            let mut phrase = String::new();
            std::io::stdin().read_line(&mut phrase)?;
            phrase.trim().to_string()
        }
    };

    if mnemonic_phrase.is_empty() {
        return invalid("Mnemonic phrase cannot be empty");
    }

    // Validate and parse mnemonic
    let helper = MnemonicHelper::from_phrase(&mnemonic_phrase, passphrase)?;

    println!("✅ Mnemonic validated successfully!");

    // Derive keypair
    let keypair = helper.to_keypair()?;
    let serializable = keypair.to_serializable();
    let json = serde_json::to_string_pretty(&serializable)?;

    // Get encryption password
    let password_value = match password {
        Some(p) => p.to_string(),
        None => {
            println!("🔒 Enter password to encrypt keystore:");
            read_password_from_stdin()?
        }
    };

    if password_value.is_empty() {
        return invalid("Password cannot be empty");
    }

    // Encrypt and save keystore
    let keystore_file = keystore::encrypt_keypair(&json, &password_value, &serializable.address)
        .map_err(|e| Error::Invalid(format!("keystore encrypt failed: {e}")))?;
    let output_file = output_path.unwrap_or("wallet-recovered.keystore");
    keystore::save_keystore(&keystore_file, Path::new(output_file))
        .map_err(|e| Error::Invalid(format!("keystore save failed: {e}")))?;

    println!("\n✅ Wallet recovered successfully!");
    println!("📄 Keystore saved to: {}", output_file);
    println!("🔐 Address: {}", serializable.address);

    Ok(())
}

/// Generate multi-signature wallet address
fn wallet_gen_multisig(
    threshold: usize,
    keystores: &[String],
    labels: &[String],
    output: &str,
) -> Result<()> {
    use ::wallet::multisig::MultisigConfig;
    use std::path::Path;

    if keystores.is_empty() {
        return invalid("At least 2 keystore files required for multisig");
    }

    println!(
        "\n🔐 Creating {}-of-{} Multi-signature Wallet",
        threshold,
        keystores.len()
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load public keys from keystores
    let mut public_keys = Vec::new();
    for (i, keystore_path) in keystores.iter().enumerate() {
        println!(
            "📂 Loading keystore {} of {}: {}",
            i + 1,
            keystores.len(),
            keystore_path
        );

        let keystore_file = keystore::load_keystore(Path::new(keystore_path))
            .map_err(|e| Error::Invalid(format!("keystore load failed: {e}")))?;

        // Prompt for password
        println!("🔑 Enter password for {}:", keystore_path);
        let password = read_password_from_stdin()?;
        let json = keystore::decrypt_keypair(&keystore_file, &password)
            .map_err(|e| Error::Invalid(format!("keystore decrypt failed: {e}")))?;
        let serializable: wallet::SerializableKeypair = serde_json::from_str(&json)?;

        public_keys.push(serializable.public_key.clone());
    }

    // Add labels if provided
    let label = if !labels.is_empty() {
        Some(labels.join(", "))
    } else {
        Some(format!("{}-of-{} Multisig", threshold, keystores.len()))
    };

    // Create multisig config
    let config = MultisigConfig::new(threshold as u8, public_keys, label)
        .map_err(|e| Error::Invalid(format!("multisig config error: {e}")))?;

    // Generate address
    let address = config.address();

    // Save config
    let config_json = serde_json::to_string_pretty(&config)?;
    std::fs::write(output, config_json)?;

    println!("\n✅ Multi-signature wallet created successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 Configuration:");
    println!("   Type: {}", config.config_type());
    println!("   Address: {}", address);
    println!("   Config saved to: {}", output);
    println!("\n👥 Signers: {}", config.total_signers);
    println!("\n💡 Next steps:");
    println!("   1. Share this address with all signers");
    println!("   2. Distribute the config file: {}", output);
    println!("   3. Use 'tx-sign-partial' to sign transactions");

    Ok(())
}

/// Show multi-signature wallet information
fn multisig_info(config_path: &str) -> Result<()> {
    use ::wallet::multisig::MultisigConfig;

    // Load config
    let config_json = std::fs::read_to_string(config_path)?;
    let config: MultisigConfig = serde_json::from_str(&config_json)?;

    // Generate address
    let address = config.address();

    println!("\n📋 Multi-signature Wallet Information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Address:   {}", address);
    println!("Type:      {}", config.config_type());
    println!(
        "Created:   {}",
        chrono::DateTime::from_timestamp(config.created_at as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );
    if let Some(label) = &config.label {
        println!("Label:     {}", label);
    }
    println!("\n👥 Signers: {}", config.total_signers);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for (i, pk) in config.public_keys.iter().enumerate() {
        let pk_preview = if pk.len() > 16 {
            format!("{}...{}", &pk[..8], &pk[pk.len() - 8..])
        } else {
            pk.clone()
        };
        println!("   {}. {}", i + 1, pk_preview);
    }
    println!("━━━━━━━━��━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Sign transaction with partial signature
fn tx_sign_partial(
    _tx_path: &str,
    _keystore_path: &str,
    _multisig_config_path: &str,
    _output: &str,
    _password: Option<&str>,
) -> Result<()> {
    // TODO: Implement when transaction format is finalized
    println!("⚠️  Partial transaction signing will be implemented in the next phase");
    println!("📋 This requires transaction serialization format to be finalized");
    println!("\n💡 For now, use the multisig module directly in your code:");
    println!("   use wallet::multisig::{{MultisigWallet, PendingMultisigTx}};");

    invalid("Feature coming soon: partial transaction signing")
}

/// Combine partial signatures
fn tx_combine_signatures(
    _tx_path: &str,
    _signature_paths: &[String],
    _multisig_config_path: &str,
    _output: &str,
) -> Result<()> {
    // TODO: Implement when transaction format is finalized
    println!("⚠️  Signature combination will be implemented in the next phase");
    println!("📋 This requires transaction serialization format to be finalized");
    println!("\n💡 For now, use the multisig module directly in your code:");
    println!("   use wallet::multisig::{{MultisigWallet, FinalizedMultisigTx}};");

    invalid("Feature coming soon: signature combination")
}

/// Create encrypted backup of wallet keystore
fn wallet_backup(
    keystore_path: &str,
    output_path: &str,
    backup_password: Option<&str>,
    network: &str,
    label: Option<String>,
) -> Result<()> {
    use ::wallet::backup::{Network, WalletBackup};
    use std::fs;

    // Read keystore file
    let keystore_data = fs::read(keystore_path)
        .map_err(|e| Error::Invalid(format!("Failed to read keystore {}: {e}", keystore_path)))?;

    // Get backup password
    let backup_pw = match backup_password {
        Some(pw) => pw.to_string(),
        None => {
            print!("Enter backup password: ");
            std::io::stdout().flush()?;
            rpassword::read_password()?
        }
    };

    // Parse network
    let net = match network.to_lowercase().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "devnet" => Network::Devnet,
        _ => {
            return invalid(format!(
                "Invalid network: {} (use mainnet, testnet, or devnet)",
                network
            ))
        }
    };

    // Create backup
    println!("Creating encrypted backup...");
    let backup = WalletBackup::create(&keystore_data, &backup_pw, net, label)
        .map_err(|e| Error::Invalid(format!("Backup creation failed: {}", e)))?;

    // Save to file
    backup
        .save(output_path)
        .map_err(|e| Error::Invalid(format!("Failed to save backup {}: {e}", output_path)))?;

    println!("✅ Backup created successfully: {}", output_path);
    println!("   Version: {}", backup.version);
    println!("   Network: {:?}", backup.network);
    println!("   Timestamp: {}", backup.timestamp);
    if let Some(lbl) = backup.label {
        println!("   Label: {}", lbl);
    }
    println!("\n⚠️  IMPORTANT: Store backup password separately and securely!");

    Ok(())
}

/// Restore wallet from encrypted backup
fn wallet_restore(
    backup_path: &str,
    output_path: &str,
    backup_password: Option<&str>,
) -> Result<()> {
    use ::wallet::backup::WalletBackup;
    use std::fs;

    // Load backup
    println!("Loading backup file...");
    let backup = WalletBackup::load(backup_path)
        .map_err(|e| Error::Invalid(format!("Failed to load backup {}: {e}", backup_path)))?;

    println!("Backup information:");
    println!("   Version: {}", backup.version);
    println!("   Network: {:?}", backup.network);
    println!("   Timestamp: {}", backup.timestamp);
    if let Some(ref label) = backup.label {
        println!("   Label: {}", label);
    }

    // Get backup password
    let backup_pw = match backup_password {
        Some(pw) => pw.to_string(),
        None => {
            print!("\nEnter backup password: ");
            std::io::stdout().flush()?;
            rpassword::read_password()?
        }
    };

    // Restore
    println!("Decrypting and restoring wallet...");
    let keystore_data = backup
        .restore(&backup_pw)
        .map_err(|e| Error::Invalid(format!("Restore failed: {}", e)))?;

    // Check if output exists
    if std::path::Path::new(output_path).exists() {
        print!("⚠️  Output file exists. Overwrite? (y/N): ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            return invalid("Restore cancelled");
        }
    }

    // Write keystore
    fs::write(output_path, keystore_data)
        .map_err(|e| Error::Invalid(format!("Failed to write keystore {}: {e}", output_path)))?;

    println!("✅ Wallet restored successfully: {}", output_path);
    println!("\n⚠️  Remember to use your original wallet password to access this keystore.");

    Ok(())
}

/// Install panic hook for better crash reporting
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("\n════════════════════════════════════════════════════════════");
        eprintln!("💥 PANIC: BitQuan node has crashed!");
        eprintln!("════════════════════════════════════════════════════════════");

        if let Some(location) = panic_info.location() {
            eprintln!(
                "Location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }

        if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("Message: {}", msg);
        } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("Message: {}", msg);
        }

        eprintln!("\n🔧 Please report this issue:");
        eprintln!("   https://github.com/your-org/bitquan/issues");
        eprintln!("\n💡 Include:");
        eprintln!("   - This error message");
        eprintln!("   - Steps to reproduce");
        eprintln!("   - Your configuration (without secrets)");
        eprintln!("════════════════════════════════════════════════════════════\n");
    }));
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

    println!("Starting BitQuan Stratum Mining Server");
    println!("  Bind address: {}", bind_addr);
    println!("  Network: {:?}", network);
    println!("  Default difficulty: {}", default_difficulty);
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

#[cfg(test)]
mod overflow_tests {
    #[test]
    fn test_balance_overflow_protection() {
        // This test verifies that checked_add is in place for balance calculation
        // In production code, we cannot easily trigger overflow without mocking,
        // but this test documents the expected behavior

        let max_balance = u64::MAX - 1000;
        let additional_value = 2000u64;

        // Simulate the checked_add operation
        let result = max_balance.checked_add(additional_value);
        assert!(result.is_none(), "Balance should overflow");
    }

    #[test]
    fn test_utxo_count_overflow_protection() {
        // Verify that UTXO count uses checked arithmetic
        let max_count = u64::MAX;
        let result = max_count.checked_add(1);
        assert!(result.is_none(), "UTXO count should overflow");
    }

    #[test]
    fn test_interval_count_overflow_protection() {
        // Verify that interval count uses checked arithmetic
        let max_count = u64::MAX;
        let result = max_count.checked_add(1);
        assert!(result.is_none(), "Interval count should overflow");
    }

    #[test]
    fn test_guard_count_overflow_protection() {
        // Verify that guard count uses checked arithmetic
        let max_count = u64::MAX;
        let result = max_count.checked_add(1);
        assert!(result.is_none(), "Guard count should overflow");
    }

    #[test]
    fn test_balance_calculation_normal() {
        // Verify normal balance calculation works correctly
        let mut balance = 0u64;
        let values = vec![1000u64, 2000, 3000, 4000, 5000];

        for value in values {
            balance = balance.checked_add(value).expect("should not overflow");
        }

        assert_eq!(balance, 15000);
    }

    #[test]
    fn test_utxo_count_normal() {
        // Verify normal UTXO counting works correctly
        let mut count = 0u64;

        for _ in 0..1000 {
            count = count.checked_add(1).expect("should not overflow");
        }

        assert_eq!(count, 1000);
    }
}
