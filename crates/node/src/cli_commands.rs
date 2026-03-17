//! CLI command definitions for BitQuan node

use crate::cli::parse_u128;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
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
