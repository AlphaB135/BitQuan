//! BitQuan reference node entrypoint.

mod wallet;

use anyhow::Result;
use bitquan_consensus::{check_header_pow, header_hash, ConsensusEngine, ConsensusParams, DifficultyState};
use bitquan_storage::{ChainStore, InMemoryChainStore};
#[cfg(feature = "rocksdb-backend")]
use bitquan_storage::rocksdb_store::RocksDBStore;
use bitquan_types::{Block, Transaction, TxIn, TxOut, SigAlgorithm};
use bq_crypto::{
    rng::{RandomSource, RngService},
    CryptoRegistry,
};
use bitquan_network::protocol::{Message, MessageEnvelope, PROTOCOL_VERSION};
use bitquan_network::io::{recv_envelope, send_envelope};
use clap::{Parser, Subcommand};
use hex::encode as hex_encode;
use std::net::{TcpListener, TcpStream, SocketAddr};
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
    },
    /// Import/show wallet address from keypair file
    WalletAddress {
        /// Path to keypair file
        #[arg(long)]
        keypair: String,
    },
    /// Sign a message with wallet keypair
    WalletSign {
        /// Path to keypair file
        #[arg(long)]
        keypair: String,
        /// Message to sign (hex-encoded)
        #[arg(long)]
        message: String,
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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => run_node(&config),
        Commands::CheckBlock { path } => check_block(&path),
        Commands::Rng { label, length } => rng_demo(&label, length),
        Commands::MineOnce { max_tries, payout_script_hex, bits } => mine_once(max_tries, &payout_script_hex, bits),
        Commands::Mine { datadir, payout_script_hex, bits, max_nonce, threads } => {
            mine_continuous(&datadir, &payout_script_hex, bits, max_nonce, threads)
        },
        Commands::WalletGen { algo, output } => wallet_gen(&algo, output.as_deref()),
        Commands::WalletAddress { keypair } => wallet_address(&keypair),
        Commands::WalletSign { keypair, message } => wallet_sign(&keypair, &message),
        Commands::WalletVerify { pubkey, message, signature } => wallet_verify(&pubkey, &message, &signature),
        Commands::BuildTx { prev_txid, prev_vout, value, to_script_hex } => build_tx(&prev_txid, prev_vout, value, &to_script_hex),
        Commands::P2PDemo { addr } => p2p_demo(&addr),
        Commands::P2PServer { listen, max_peers, datadir } => p2p_server(&listen, max_peers, &datadir),
        Commands::P2PConnect { peer, height } => p2p_connect(&peer, height),
        Commands::Balance { datadir, script_hex } => check_balance(&datadir, script_hex.as_deref()),
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
                &MessageEnvelope::new(Message::Reject { message: "expected version".into(), code: bitquan_network::protocol::RejectCode::Malformed, reason: "handshake".into() })
            )?;
            return Ok(());
        }
    }

    // Minimal message loop: respond to Ping with Pong
    loop {
        let msg = read_envelope(&stream)?;
        match msg.message {
            Message::Ping { nonce } => write_envelope(&stream, &MessageEnvelope::new(Message::Pong { nonce }))?,
            Message::GetAddr => write_envelope(&stream, &MessageEnvelope::new(Message::Addr { addrs: vec![] }))?,
            _ => {}
        }
    }
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
    let subsidy = bitquan_consensus::ConsensusParams::phase3_defaults().reward_schedule.subsidy_at_height(store.height());
    let coinbase = Transaction {
        version: 2,
        lock_time: 0,
        inputs: vec![coinbase_in],
        outputs: vec![TxOut { value: subsidy, script_pubkey: payout_script }],
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
        let (anchor_bits, anchor_time) = if let Ok(Some(tip)) = store.tip() { (tip.bits, tip.time as u64) } else { (0x207fffff, now as u64) };
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
            let block = Block { header: header.clone(), transactions: vec![coinbase] };
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
fn mine_continuous(datadir: &str, payout_script_hex: &str, mut bits: u32, max_nonce: u64, threads: usize) -> Result<()> {
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    
    println!("BitQuan Continuous Miner");
    println!("Data directory: {}", datadir);
    println!("Threads: {}", if threads == 0 { num_cpus::get() } else { threads });
    
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
        
        let subsidy = ConsensusParams::phase3_defaults().reward_schedule.subsidy_at_height(height);
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
                println!("Time: {:.2}s | Hashrate: {:.2} H/s", elapsed.as_secs_f64(), hashrate);
                
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
                println!("... tried {} nonces ({:.2} H/s), latest hash={}", 
                    n, hashrate, hex::encode(current_hash));
            }
        }
        
        if !found.load(Ordering::Relaxed) {
            println!("\nNo valid nonce in {} tries, adjusting difficulty...", max_nonce);
            bits = (bits & 0x00ffffff) | ((((bits >> 24) + 1) & 0xff) << 24); // Easier
        }
    }
}

#[cfg(not(feature = "rocksdb-backend"))]
fn mine_continuous(_datadir: &str, _payout_script_hex: &str, _bits: u32, _max_nonce: u64, _threads: usize) -> Result<()> {
    eprintln!("ERROR: Continuous mining requires 'rocksdb-backend' feature");
    eprintln!("Rebuild with: cargo build --release --features rocksdb-backend");
    Ok(())
}

/// Generate a wallet keypair
fn wallet_gen(algo: &str, output_path: Option<&str>) -> Result<()> {
    use wallet::{WalletKeypair, address};

    println!("BitQuan Wallet Generator");
    println!("Algorithm: {}", algo);

    if algo != "dilithium3" {
        anyhow::bail!("Only 'dilithium3' is supported currently");
    }

    println!("\n⏳ Generating keypair...");
    let keypair = WalletKeypair::generate_dilithium3()?;

    let pubkey_hash = keypair.public_key_hash();
    let address_str = address::encode(&pubkey_hash);

    println!("\n✅ Keypair generated successfully!");
    println!("\n📍 Address: {}", address_str);
    println!("📏 Public key size: {} bytes", keypair.public_key.len());
    println!("📏 Secret key size: {} bytes", keypair.secret_key.len());
    println!("\n🔑 Public key hash: {}", hex::encode(pubkey_hash));
    println!("🔑 Public key: {}", hex::encode(&keypair.public_key[..64.min(keypair.public_key.len())]));
    println!("   ... (truncated, full: {} bytes)", keypair.public_key.len());

    if let Some(path) = output_path {
        use std::path::Path;
        keypair.save_to_file(Path::new(path))?;
        println!("\n💾 Keypair saved to: {}", path);
    } else {
        println!("\n💡 Tip: Use --output <file> to save keypair");
    }

    println!("\n⚠️  Keep your secret key safe! Anyone with it can spend your coins.");

    Ok(())
}

/// Show wallet address from keypair file
fn wallet_address(keypair_path: &str) -> Result<()> {
    use wallet::{WalletKeypair, address};
    use std::path::Path;

    println!("BitQuan Wallet Address");
    println!("Loading keypair from: {}", keypair_path);

    let keypair = WalletKeypair::load_from_file(Path::new(keypair_path))?;
    let pubkey_hash = keypair.public_key_hash();
    let addr = address::encode(&pubkey_hash);

    println!("\n📍 Address: {}", addr);
    println!("🔑 Public key hash: {}", hex::encode(pubkey_hash));
    println!("📏 Public key: {} bytes", keypair.public_key.len());

    Ok(())
}

/// Sign a message with wallet keypair
fn wallet_sign(keypair_path: &str, message_hex: &str) -> Result<()> {
    use wallet::WalletKeypair;
    use std::path::Path;

    println!("BitQuan Wallet Sign");
    println!("Keypair: {}", keypair_path);

    let message = hex::decode(message_hex)?;
    println!("Message: {} ({} bytes)", message_hex, message.len());

    let keypair = WalletKeypair::load_from_file(Path::new(keypair_path))?;
    
    println!("\n⏳ Signing...");
    let signature = keypair.sign(&message)?;

    println!("✅ Signature generated!");
    println!("📏 Signature size: {} bytes", signature.len());
    println!("📝 Signature: {}", hex::encode(&signature));

    // Verify immediately
    if keypair.verify(&message, &signature) {
        println!("✅ Signature verified successfully!");
    } else {
        println!("❌ Signature verification failed!");
    }

    Ok(())
}

/// Verify a signature
fn wallet_verify(pubkey_hex: &str, message_hex: &str, signature_hex: &str) -> Result<()> {
    use wallet::{WalletPublicKey, WalletAlgorithm};

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

    println!("\n⏳ Verifying...");
    if public_key.verify(&message, &signature) {
        println!("✅ Signature is VALID!");
        Ok(())
    } else {
        println!("❌ Signature is INVALID!");
        anyhow::bail!("Signature verification failed")
    }
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

    let input = TxIn { prev_txid: prev, prev_vout, sequence: u32::MAX, script_sig: Vec::new() };
    let output = TxOut { value, script_pubkey };
    let tx = Transaction { version: 2, lock_time: 0, inputs: vec![input], outputs: vec![output], sig_algo: SigAlgorithm::Dilithium3, witnesses: vec![] };

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
            match env.message {
                Message::Version { .. } => {
                    // Reply VerAck
                    write_envelope(&stream, &MessageEnvelope::new(Message::VerAck))?;
                    // Expect Ping then reply Pong
                    let ping = read_envelope(&stream)?;
                    if let Message::Ping { nonce } = ping.message {
                        write_envelope(&stream, &MessageEnvelope::new(Message::Pong { nonce }))?;
                    }
                }
                _ => {}
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
    if let Message::Pong { nonce: n } = pong.message { println!("P2P demo OK (nonce={n})"); } else { println!("P2P demo failed"); }

    // Wait server
    let _ = server.join().unwrap_or(Ok(()));
    Ok(())
}

/// P2P Server that accepts incoming connections
fn p2p_server(listen: &str, max_peers: usize, datadir: &str) -> Result<()> {
    use bitquan_network::{PeerManager, P2PListener};
    use std::sync::Arc;

    println!("BitQuan P2P Server");
    println!("Listen: {}", listen);
    println!("Max peers: {}", max_peers);
    println!("Data dir: {}", datadir);
    
    // Load current height from storage
    #[cfg(feature = "rocksdb-backend")]
    let height = {
        use bitquan_storage::rocksdb_store::RocksDBStore;
        let store = RocksDBStore::open(datadir)?;
        store.height().unwrap_or(0)
    };
    
    #[cfg(not(feature = "rocksdb-backend"))]
    let height = 0u64;
    
    println!("Current height: {}", height);

    let peer_manager = Arc::new(PeerManager::new(max_peers));
    peer_manager.update_height(height);
    
    let listener = P2PListener::bind(listen, peer_manager.clone())?;
    println!("✅ Server started at {}", listener.local_addr()?);
    println!("Waiting for connections...");

    loop {
        match listener.accept_one() {
            Ok(()) => {
                let count = peer_manager.peer_count();
                let ready = peer_manager.ready_peer_count();
                println!("✅ Peer connected! Total: {}, Ready: {}", count, ready);
            }
            Err(e) => {
                eprintln!("❌ Accept error: {}", e);
            }
        }
        
        // Cleanup dead peers
        peer_manager.cleanup_peers();
        
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
fn check_balance(datadir: &str, script_hex: Option<&str>) -> Result<()> {
    use bitquan_storage::rocksdb_store::RocksDBStore;
    
    let store = RocksDBStore::open(datadir)?;
    let height = store.height()?;
    
    println!("\n=== BitQuan Balance ===");
    println!("Chain height: {}", height);
    
    if let Some(script) = script_hex {
        let target_script = hex::decode(script)?;
        
        println!("Script: {}", script);
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
                            println!("  Block #{} TX {} vout={} amount={}", 
                                h, hex::encode(tx.txid()), vout, output.value);
                        }
                    }
                }
            }
        }
        
        println!("\nUTXO count: {}", utxo_count);
        println!("Balance: {} qbits", balance);
        println!("Balance: {:.8} BQ", balance as f64 / 100_000_000.0);
    } else {
        // Show total supply
        println!("\nTotal supply calculation:");
        
        let mut total_supply: u64 = 0;
        
        for h in 0..=height {
            if let Ok(Some(block)) = store.get_block_by_height(h) {
                for tx in &block.transactions {
                    for output in &tx.outputs {
                        total_supply += output.value;
                    }
                }
            }
        }
        
        println!("Total coins mined: {} qbits", total_supply);
        println!("Total coins mined: {:.8} BQ", total_supply as f64 / 100_000_000.0);
        println!("\nBlocks mined: {}", height + 1);
    }
    
    Ok(())
}

#[cfg(not(feature = "rocksdb-backend"))]
fn check_balance(_datadir: &str, _script_hex: Option<&str>) -> Result<()> {
    eprintln!("ERROR: Balance checking requires 'rocksdb-backend' feature");
    eprintln!("Rebuild with: cargo build --release --features rocksdb-backend");
    Ok(())
}
