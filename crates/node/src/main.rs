//! BitQuan reference node entrypoint.

mod address;
mod keystore;
mod mnemonic;
mod tx_builder;
mod utxo;
mod wallet;

use anyhow::Result;
use bitquan_consensus::{
    check_header_pow, header_hash, ConsensusEngine, ConsensusParams, DifficultyState,
};
use bitquan_network::io::{recv_envelope, send_envelope};
use bitquan_network::protocol::{Message, MessageEnvelope, PROTOCOL_VERSION};
#[cfg(feature = "rocksdb-backend")]
use bitquan_storage::rocksdb_store::RocksDBStore;
use bitquan_storage::{ChainStore, InMemoryChainStore};
use bitquan_types::{Block, SigAlgorithm, Transaction, TxIn, TxOut};
use bq_crypto::{
    rng::{RandomSource, RngService},
    CryptoRegistry,
};
use clap::{Parser, Subcommand};
use hex::encode as hex_encode;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

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
        /// Number of threads for mining (0 = CPU count)
        #[arg(long, default_value_t = 1)]
        threads: usize,
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
    /// Import/show wallet address from keypair file
    WalletAddress {
        /// Path to keystore file
        #[arg(long)]
        keystore: String,
        /// Password to decrypt the keystore
        #[arg(long)]
        password: Option<String>,
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
        } => mine_once(max_tries, &payout_script_hex, bits),
        Commands::Mine {
            datadir,
            payout_script_hex,
            bits,
            max_nonce,
            threads,
        } => mine_continuous(&datadir, &payout_script_hex, bits, max_nonce, threads),
        Commands::WalletGen {
            algo,
            output,
            password,
        } => wallet_gen(&algo, output.as_deref(), password.as_deref()),
        Commands::WalletAddress { keystore, password } => {
            wallet_address(&keystore, password.as_deref())
        }
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
        } => p2p_server(&listen, max_peers, &datadir),
        Commands::P2PConnect { peer, height } => p2p_connect(&peer, height),
        Commands::Balance {
            datadir,
            script_hex,
            address,
        } => check_balance(&datadir, script_hex.as_deref(), address.as_deref()),
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

        if check_header_pow(&genesis.header) {
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
    use bitquan_types::{Block, BlockHeader, SigAlgorithm, Transaction, TxOut};
    let mut store = InMemoryChainStore::new();

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
    let subsidy = bitquan_consensus::ConsensusParams::phase3_defaults()
        .reward_schedule
        .subsidy_at_height(store.height());
    let coinbase = Transaction {
        version: 2,
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
        let mut state = DifficultyState::new(0, anchor_time, anchor_bits);
        bits = state.update(1, time as u64, &params);
    }

    let mut header = BlockHeader {
        version: 1,
        prev_block: prev,
        merkle_root,
        pqc_agg_hint: witness_root,
        time,
        bits,
        nonce: 0,
    };

    for n in 0..max_tries {
        header.nonce = n;
        if check_header_pow(&header) {
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

/// Continuous mining with persistent RocksDB storage
#[cfg(feature = "rocksdb-backend")]
fn mine_continuous(
    datadir: &str,
    payout_script_hex: &str,
    mut bits: u32,
    max_nonce: u64,
    threads: usize,
) -> Result<()> {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

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

    // Open or create RocksDB store
    let store = RocksDBStore::open(datadir)?;
    let store = Arc::new(Mutex::new(store));

    let payout_script = hex::decode(payout_script_hex)?;
    let found = Arc::new(AtomicBool::new(false));
    let blocks_mined = Arc::new(AtomicU64::new(0));

    loop {
        let height = {
            let s = store.lock().unwrap();
            s.height()?
        };

        println!("\n[Block #{}] Mining...", height + 1);

        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);

        if now == 0 {
            eprintln!("ERROR: System time is before UNIX epoch");
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

        let subsidy = ConsensusParams::phase3_defaults()
            .reward_schedule
            .subsidy_at_height(height);
        let coinbase = Transaction {
            version: 2,
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

        // Auto-calc bits if zero
        if bits == 0 {
            let params = ConsensusParams::phase3_defaults();
            let s = store.lock().unwrap();
            let (anchor_bits, anchor_time) = if let Ok(Some(tip)) = s.tip() {
                (tip.bits, tip.time as u64)
            } else {
                (0x207fffff, now as u64)
            };
            drop(s);
            let mut state = DifficultyState::new(0, anchor_time, anchor_bits);
            bits = state.update(1, time as u64, &params);
        }

        let mut header = bitquan_types::BlockHeader {
            version: 1,
            prev_block: prev,
            merkle_root,
            pqc_agg_hint: witness_root,
            time,
            bits,
            nonce: 0,
        };

        println!("Mining block #{} ...", height + 1);
        println!("Target bits: 0x{:08x}", bits);
        println!("Block reward: {} qbits", subsidy);

        // Mining loop
        found.store(false, Ordering::Relaxed);
        let start_time = std::time::Instant::now();

        for n in 0..max_nonce {
            header.nonce = n;
            if check_header_pow(&header) {
                let id = header_hash(&header);
                let elapsed = start_time.elapsed();
                let hashrate = (n as f64) / elapsed.as_secs_f64();

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
                    s.insert_block(block)?;
                }

                blocks_mined.fetch_add(1, Ordering::Relaxed);
                let total = blocks_mined.load(Ordering::Relaxed);
                println!("Saved to DB | Session total: {}", total);
                found.store(true, Ordering::Relaxed);
                break;
            }

            if n % 100_000 == 0 && n > 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let hashrate = (n as f64) / elapsed;
                let current_hash = header_hash(&header);
                println!(
                    "... tried {} nonces ({:.2} H/s), latest hash={}",
                    n,
                    hashrate,
                    hex::encode(current_hash)
                );
            }
        }

        if !found.load(Ordering::Relaxed) {
            println!(
                "\nNo valid nonce in {} tries, adjusting difficulty...",
                max_nonce
            );
            bits = (bits & 0x00ffffff) | ((((bits >> 24) + 1) & 0xff) << 24); // Easier
        }
    }
}

#[cfg(not(feature = "rocksdb-backend"))]
fn mine_continuous(
    _datadir: &str,
    _payout_script_hex: &str,
    _bits: u32,
    _max_nonce: u64,
    _threads: usize,
) -> Result<()> {
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
        anyhow::bail!("Only 'dilithium3' is supported currently");
    }

    println!("\n⏳ Generating keypair...");
    let keypair = WalletKeypair::generate_dilithium3()?;

    let pubkey_hash = keypair.public_key_hash();
    let address_str = address::encode(&pubkey_hash);

    use pqc_dilithium::{PUBLICKEYBYTES, SECRETKEYBYTES};

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
        anyhow::bail!("Password must be at least 8 characters");
    }

    // Serialize keypair metadata for encryption
    let serializable = keypair.to_serializable();
    let json = serde_json::to_string_pretty(&serializable)?;

    // Encrypt and save
    let keystore_file = keystore::encrypt_keypair(&json, &password)?;

    let path = output_path.unwrap_or("wallet.keystore");
    keystore::save_keystore(&keystore_file, Path::new(path))?;

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
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))?;

    // Get password
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("\n🔒 Enter password:");
            read_password_from_stdin()?
        }
    };

    // Decrypt
    let json = keystore::decrypt_keypair(&keystore_file, &password)?;
    let data: wallet::SerializableKeypair = serde_json::from_str(&json)?;

    println!("\n📍 Address: {}", data.address);
    println!("🔑 Public key hash: {}", data.public_key_hash);
    println!("📏 Metadata only (full keys require session keypair)");

    Ok(())
}

/// Sign a message with encrypted wallet keypair
fn wallet_sign(keystore_path: &str, message_hex: &str, password: Option<&str>) -> Result<()> {
    use std::path::Path;

    println!("BitQuan Wallet Sign");
    println!("Keystore: {}", keystore_path);

    let message = hex::decode(message_hex)?;
    println!("Message: {} ({} bytes)", message_hex, message.len());

    // Load keystore
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))?;

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
    let json = keystore::decrypt_keypair(&keystore_file, &password)?;
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

    let pubkey_bytes = hex::decode(pubkey_hex)?;
    let message = hex::decode(message_hex)?;
    let signature = hex::decode(signature_hex)?;

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
        anyhow::bail!("Signature verification failed")
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
    let keystore_file = keystore::load_keystore(Path::new(keystore_path))?;

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
    let json = keystore::decrypt_keypair(&keystore_file, &password)?;
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
    let prev_vec = hex::decode(prev_txid_hex)?;
    if prev_vec.len() != 32 {
        println!("prev_txid must be 32 bytes hex");
        return Ok(());
    }
    prev.copy_from_slice(&prev_vec);
    let script_pubkey = hex::decode(to_script_hex)?;

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
    send_envelope(&mut stream, env).map_err(|e| anyhow::anyhow!(e.to_string()))
}

fn read_envelope(mut stream: &TcpStream) -> Result<MessageEnvelope> {
    recv_envelope(&mut stream).map_err(|e| anyhow::anyhow!(e.to_string()))
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
fn p2p_server(listen: &str, max_peers: usize, datadir: &str) -> Result<()> {
    use bitquan_network::{P2PListener, PeerManager};
    use std::sync::Arc;
    use std::sync::Mutex;

    println!("BitQuan P2P Server");
    println!("Listen: {}", listen);
    println!("Max peers: {}", max_peers);
    println!("Data dir: {}", datadir);

    // Load current height from storage
    #[cfg(feature = "rocksdb-backend")]
    let (height, store) = {
        use bitquan_storage::rocksdb_store::RocksDBStore;
        let store = RocksDBStore::open(datadir)?;
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

    // Create relay manager
    use bitquan_network::RelayManager;
    let relay_manager = Arc::new(RelayManager::new(10000));

    let peer_manager = Arc::new(PeerManager::with_relay(max_peers, relay_manager.clone()));
    peer_manager.update_height(height);

    let listener = P2PListener::bind(listen, peer_manager.clone())?;
    println!("Server started at {}", listener.local_addr()?);
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

    let addr: SocketAddr = peer.parse()?;

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
            Err(e.into())
        }
    }
}

/// Check balance for a script
#[cfg(feature = "rocksdb-backend")]
fn check_balance(datadir: &str, script_hex: Option<&str>, address: Option<&str>) -> Result<()> {
    use bitquan_storage::rocksdb_store::RocksDBStore;

    let store = RocksDBStore::open(datadir)?;
    let height = store.height()?;

    println!("\n=== BitQuan Balance ===");
    println!("Chain height: {}", height);

    // Determine script_pubkey from either script_hex or address
    let target_script = if let Some(script) = script_hex {
        hex::decode(script)?
    } else if let Some(addr) = address {
        // Decode bech32m address to pubkey hash
        let pubkey_hash = address::decode_bech32m(addr)
            .map_err(|e| anyhow::anyhow!("Failed to decode address: {}", e))?;
        
        // Create P2PKH script: OP_DUP OP_HASH256 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
        // For simplicity, we'll use a direct format (adjust based on your script format)
        // Standard P2PKH: 76 a9 14 <20-byte-hash> 88 ac
        // But for Dilithium with 32-byte hash: 76 a9 20 <32-byte-hash> 88 ac
        let mut script = Vec::with_capacity(35);
        script.push(0x76); // OP_DUP
        script.push(0xa9); // OP_HASH256
        script.push(0x20); // Push 32 bytes
        script.extend_from_slice(&pubkey_hash);
        script.push(0x88); // OP_EQUALVERIFY
        script.push(0xac); // OP_CHECKSIG
        
        println!("Decoded address: {}", addr);
        println!("Pubkey hash: {}", hex::encode(pubkey_hash));
        
        script
    } else {
        anyhow::bail!("Either --script-hex or --address must be provided");
    };

    println!("Script: {}", hex::encode(&target_script));
    println!("\nScanning blockchain for UTXOs...");

    let mut balance: u64 = 0;
    let mut utxo_count = 0;

    // Scan all blocks (simple implementation)
    for h in 0..=height {
        if let Ok(Some(block)) = store.get_block_by_height(h) {
            for tx in &block.transactions {
                for (vout, output) in tx.outputs.iter().enumerate() {
                    if output.script_pubkey == target_script {
                        // Check if spent (simplified - should check UTXO set)
                        balance += output.value;
                        utxo_count += 1;
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
