//! P2P network commands for BitQuan CLI
//!
//! This module contains all P2P-related commands:
//! - p2p_server, p2p_connect, p2p_demo
//! - setup_p2p_network, setup_storage

use bitquan_consensus::{ConsensusEngine, ConsensusParams};
use bitquan_network::io::{recv_envelope, send_envelope};
use bitquan_network::protocol::{network_magic, Message, MessageEnvelope, PROTOCOL_VERSION};
use bitquan_types::error::{Error, Result};
use bitquan_types::genesis::GENESIS_HASH_BYTES;
use bitquan_types::NetworkId;
use log::{debug, error, info, warn};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[cfg(feature = "rocksdb-backend")]
use bitquan_rpc::{tls::TlsConfig, IpNetwork};
#[cfg(feature = "rocksdb-backend")]
use std::str::FromStr;

// Need to import from main for some dependencies
use crate::commands::rpc::run_rpc_server;

/// Write message envelope to TCP stream
pub fn write_envelope(mut stream: &TcpStream, env: &MessageEnvelope) -> Result<()> {
    send_envelope(&mut stream, env).map_err(|e| Error::Net(e.to_string()))
}

/// Read message envelope from TCP stream
pub fn read_envelope(mut stream: &TcpStream, magic: [u8; 4]) -> Result<MessageEnvelope> {
    recv_envelope(&mut stream, magic).map_err(|e| Error::Net(e.to_string()))
}

/// P2P demo - creates server and client for testing
pub fn p2p_demo(addr: &str) -> Result<()> {
    // Start server
    let addr_str = addr.to_string();
    let server = thread::spawn(move || -> Result<()> {
        let listener = TcpListener::bind(&addr_str)?;
        listener.set_nonblocking(false)?;
        let magic = network_magic(NetworkId::Mainnet);
        if let Ok((stream, _peer)) = listener.accept() {
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            // Expect Version
            let env = read_envelope(&stream, magic)?;
            if let Message::Version { .. } = env.message {
                // Reply VerAck
                write_envelope(&stream, &MessageEnvelope::new(magic, Message::VerAck))?;
                // Expect Ping then reply Pong
                let ping = read_envelope(&stream, magic)?;
                if let Message::Ping { nonce } = ping.message {
                    write_envelope(
                        &stream,
                        &MessageEnvelope::new(magic, Message::Pong { nonce }),
                    )?;
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
    let magic = network_magic(NetworkId::Mainnet);
    let version = Message::Version {
        version: PROTOCOL_VERSION,
        services: 1,
        timestamp: 1_700_000_000,
        user_agent: "BitQuan/0.1.0".into(),
        start_height: 0,
    };
    write_envelope(&client, &MessageEnvelope::new(magic, version))?;
    let verack = read_envelope(&client, magic)?;
    if !matches!(verack.message, Message::VerAck) {
        warn!("Unexpected message from server");
        return Ok(());
    }
    let nonce = 42u64;
    write_envelope(
        &client,
        &MessageEnvelope::new(magic, Message::Ping { nonce }),
    )?;
    let pong = read_envelope(&client, magic)?;
    if let Message::Pong { nonce: n } = pong.message {
        info!("P2P demo OK (nonce={n})");
    } else {
        error!("P2P demo failed");
    }

    // Wait server
    let _ = server.join().unwrap_or(Ok(()));
    Ok(())
}

/// Setup and verify storage backend (RocksDB/Memory)
#[cfg(feature = "rocksdb-backend")]
pub fn setup_storage(
    datadir: &str,
) -> Result<(
    u64,
    std::sync::Arc<
        bitquan_storage::async_store::AsyncStoreWrapper<
            bitquan_storage::rocksdb_store::RocksDBStore,
        >,
    >,
)> {
    use bitquan_storage::async_store::AsyncStoreWrapper;
    use bitquan_storage::rocksdb_store::RocksDBStore;

    info!("Initializing storage at: {}", datadir);
    let rocksdb_store = RocksDBStore::open(datadir)
        .map_err(|e| Error::Invalid(format!("failed to open RocksDB: {e}")))?;

    // Sync check
    let height = rocksdb_store.height().unwrap_or(0);
    info!("Current chain height: {}", height);

    let async_store = std::sync::Arc::new(AsyncStoreWrapper::new(rocksdb_store));
    Ok((height, async_store))
}

#[cfg(not(feature = "rocksdb-backend"))]
pub fn setup_storage(
    _datadir: &str,
) -> Result<(
    u64,
    std::sync::Arc<
        bitquan_storage::async_store::AsyncStoreWrapper<bitquan_storage::InMemoryChainStore>,
    >,
)> {
    let height = 0u64;
    let store = std::sync::Arc::new(bitquan_storage::async_store::AsyncStoreWrapper::new(
        bitquan_storage::InMemoryChainStore::new(),
    ));
    Ok((height, store))
}

/// Generate or load a secure JWT secret for RPC authentication.
///
/// SECURITY: Never use hardcoded secrets in production!
/// This function generates a cryptographically secure 32-byte secret.
pub fn get_or_create_jwt_secret(datadir: &str) -> Result<String> {
    use rand::Rng;

    let path = std::path::Path::new(datadir).join("jwt.hex");

    // Try to load existing secret
    if path.exists() {
        info!("Loading JWT secret from {:?}", path);
        return std::fs::read_to_string(&path)
            .map(|s| s.trim().to_string())
            .map_err(|e| Error::Invalid(format!("failed to load JWT secret: {e}")));
    }

    // Generate new cryptographically secure secret
    warn!("No JWT secret found. Generating a new secure one...");
    let secret: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64) // 64 hex chars = 32 bytes
        .map(char::from)
        .collect();

    // Save to disk for persistence
    std::fs::write(&path, &secret)
        .map_err(|e| Error::Invalid(format!("failed to save JWT secret: {e}")))?;

    info!("New JWT secret saved to {:?}", path);
    warn!("IMPORTANT: Keep this file secure! Anyone with this secret can access your RPC.");

    Ok(secret)
}

/// Setup P2P Networking (PeerManager, Noise, Relay)
pub async fn setup_p2p_network(
    max_peers: usize,
    network: NetworkId,
    height: u64,
) -> Result<(
    std::sync::Arc<bitquan_network::PeerManager>,
    std::net::SocketAddr,
    std::sync::Arc<bitquan_network::noise::NoiseConfig>,
)> {
    use bitquan_network::{NoiseConfig, PeerManager, RelayManager};

    // 1. Setup Noise keys
    let noise_config = std::sync::Arc::new(
        NoiseConfig::generate()
            .map_err(|e| Error::Invalid(format!("failed to generate noise config: {e}")))?,
    );
    info!("P2P Identity: {}", noise_config.public_key_hex());

    // 2. Setup Relay Manager
    let relay_manager = std::sync::Arc::new(RelayManager::new(10000));

    // 3. Setup Peer Manager
    let peer_manager = std::sync::Arc::new(PeerManager::with_relay(
        max_peers,
        relay_manager,
        network,
        noise_config.clone(),
    ));

    // Initial state sync
    peer_manager.update_height(height).await;

    // 4. Load persistent peers
    let peers_json_path = std::path::PathBuf::from("peers.json");
    if peers_json_path.exists() {
        if let Err(e) = peer_manager.load_address_book(&peers_json_path) {
            warn!("Failed to load peers.json: {}", e);
        } else {
            let count = peer_manager.known_peers_count().unwrap_or(0);
            info!("Loaded {} peers from disk", count);
        }
    }

    Ok((
        peer_manager,
        std::net::SocketAddr::from(([0, 0, 0, 0], 0)),
        noise_config,
    ))
}

/// P2P Server that accepts incoming connections
pub async fn p2p_server(
    listen: &str,
    max_peers: usize,
    datadir: &str,
    rpc: RpcServerOptions<'_>,
    network: NetworkId,
    bootstrap_peers: Option<Vec<String>>,
) -> Result<()> {
    use bitquan_mempool::Mempool;

    info!("BitQuan P2P Server");
    info!("Listen: {}", listen);
    info!("Max peers: {}", max_peers);
    info!("Data dir: {}", datadir);

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

    // Load current height from storage (using helper)
    let (height, store) = setup_storage(datadir)?;

    info!("Current height: {}", height);
    debug!("Storage: In-Memory");

    // Initialize mempool (moved up for global scope)
    // Create mempool for transaction relay
    let mempool =
        Arc::new(tokio::sync::Mutex::new(Mempool::new().map_err(|e| {
            Error::Invalid(format!("failed to create mempool: {e}"))
        })?));
    info!("Mempool initialized (max 300 MB)");

    if let Some(addr) = rpc_listen {
        let username = rpc_username.ok_or_else(|| {
            Error::Invalid("--rpc-username is required when enabling RPC server".to_string())
        })?;

        let password_value = if let Some(pass) = rpc_password {
            pass.to_string()
        } else {
            info!("Enter RPC password:");
            let input = crate::cli::read_password_from_stdin()?;
            if input.is_empty() {
                return crate::cli::invalid("RPC password cannot be empty");
            }
            input
        };

        if password_value.is_empty() {
            return crate::cli::invalid("RPC password cannot be empty");
        }

        if username.is_empty() {
            return crate::cli::invalid("RPC username cannot be empty");
        }

        if !addr.starts_with("127.") && !addr.starts_with("localhost") {
            warn!(
                "RPC server binding to '{}'. Ensure firewall and authentication are configured.",
                addr
            );
        }

        let store_arc = store.clone();

        // Generate noise config with proper error handling
        let noise_config = Arc::new(
            bitquan_network::noise::NoiseConfig::generate()
                .map_err(|e| Error::Invalid(format!("Failed to generate Noise config: {}", e)))?,
        );

        // Initialize RPC handler directly instead of using sync_task
        let sync_mgr = Arc::new(
            bitquan_network::async_sync::AsyncSyncManager::new_with_components(
                height,
                Arc::new(bitquan_network::PeerManager::new(1, network, noise_config)),
                Arc::new(std::sync::Mutex::new(
                    bitquan_network::discovery::PeerBook::new(),
                )),
                network,
                store_arc.clone(),
            ),
        );
        let handler = crate::rpc::NodeRpcHandler::with_components(
            store_arc,
            network.name(),
            sync_mgr,
            Some(mempool.clone()),
        );

        let rpc_addr = addr.to_string();

        // JWT authentication is required
        use bitquan_rpc::RpcConfig;

        debug!("RPC authentication: JWT");

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
            return crate::cli::invalid("--rpc-tls-key provided without --rpc-tls-cert");
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
            return crate::cli::invalid(
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
        debug!(
      "RPC config: max_body_bytes={} rl_burst={} rl_refill_per_sec={} conn_cooldown_ms={} max_header_bytes={} header_timeout_ms={} trust_proxy={} require_tls={} tls_configured={}",
      rpc_config.max_body_bytes,
      rpc_config.rl_burst,
      rpc_config.rl_refill_per_sec,
      rpc_config.conn_cooldown_ms,
      rpc_config.max_header_bytes,
      rpc_config.header_read_timeout_ms,
      rpc_config.trust_proxy,
      rpc_config.require_tls,
      tls_config.is_some()
    );

        if let Some(cert_path) = rpc_tls_cert {
            info!("RPC TLS certificate: {}", cert_path);
        } else if rpc_config.require_tls {
            warn!("RPC TLS certificate: <required>");
        } else {
            debug!("RPC TLS certificate: <not configured>");
        }

        let tls_config_for_thread = tls_config.clone();
        let jwt_config_owned = jwt_config.map(|s| s.to_string());
        let jwt_secret_owned = jwt_secret.map(|s| s.to_string());
        let username_owned = username.to_string();
        let password_owned = password_value.clone();

        let rpc_config_owned = rpc_config.clone();
        let datadir_owned = datadir.to_string(); // For thread safety

        thread::spawn(move || {
            run_rpc_server(
                handler,
                rpc_addr,
                jwt_config_owned,
                jwt_secret_owned,
                rpc_config_owned,
                tls_config_for_thread,
                username_owned,
                password_owned,
                require_tls,
                datadir_owned, // For JWT secret generation
            );
        });
        info!("RPC server listening on {}", addr);
    }

    // === P2P SERVER SETUP ===
    // Setup P2P networking using helper (Noise, PeerManager, Relay, Peers.json)
    let (peer_manager, _listen_addr, noise_config) =
        setup_p2p_network(max_peers, network, height).await?;

    // Create peer book for sync manager
    let peer_book = Arc::new(std::sync::Mutex::new(
        bitquan_network::discovery::PeerBook::new(),
    ));

    // Create sync manager for IBD (Initial Block Download)
    let sync_manager = Arc::new(
        bitquan_network::async_sync::AsyncSyncManager::new_with_components(
            height,
            peer_manager.clone(),
            peer_book.clone(),
            network,
            store.clone(),
        ),
    );
    info!("Sync manager initialized (local height: {})", height);

    // Mempool initialized earlier for RPC handler dependency

    // Create fork choice manager for chain reorganization
    let fork_choice = Arc::new(tokio::sync::Mutex::new(
        bitquan_consensus::fork::ForkChoice::new(),
    ));
    debug!("ForkChoice initialized");

    // Create consensus engine for block validation
    let consensus_params = ConsensusParams::phase3_defaults();
    let consensus = Arc::new(tokio::sync::Mutex::new(ConsensusEngine::new(
        consensus_params,
        bq_crypto::CryptoRegistry::default(),
    )));
    debug!("Consensus engine initialized");

    // Create ban manager for peer misconduct
    let ban_config = bitquan_network::ban_manager::BanConfig::default();
    let ban_manager = Arc::new(tokio::sync::Mutex::new(
        bitquan_network::ban_manager::BanManager::new(ban_config),
    ));
    debug!("Ban manager initialized");

    // Create worker context for peer handlers
    let worker_ctx = Arc::new(crate::worker::WorkerContext::new(
        peer_manager.clone(),
        store.clone(),
        mempool.clone(),
        consensus.clone(),
        fork_choice.clone(),
        ban_manager.clone(),
        network,
        GENESIS_HASH_BYTES,
    ));
    debug!("Worker context initialized");

    // Set initial block height metric
    crate::metrics::update_block_height(height);

    // Spawn periodic metrics update task
    // Updates connected_peers and mempool_size every 10 seconds
    let peer_manager_for_metrics = peer_manager.clone();
    let mempool_for_metrics = mempool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;

            // Update connected peers metric
            let count = peer_manager_for_metrics.peer_count().await;
            crate::metrics::update_connected_peers(count);

            // Update mempool size metric
            let mempool_lock = mempool_for_metrics.lock().await;
            crate::metrics::update_mempool_size(mempool_lock.len());
            drop(mempool_lock);
        }
    });

    // Spawn peer discovery loop
    let peer_manager_for_discovery = peer_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            // Check if we have connected peers
            let peer_count = peer_manager_for_discovery.peer_count().await;

            if peer_count == 0 {
                log::debug!("No peers connected, skipping discovery");
                continue;
            }

            // Broadcast GetAddr to all ready peers for peer discovery
            if let Err(e) = peer_manager_for_discovery
                .broadcast(bitquan_network::protocol::Message::GetAddr)
                .await
            {
                log::warn!("Failed to broadcast GetAddr for discovery: {}", e);
            } else {
                log::debug!("🔍 Broadcast GetAddr to {} peers for discovery", peer_count);
            }
        }
    });

    // Spawn peer address book persistence loop
    let peer_manager_for_save = peer_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;

            let peers_path = std::path::PathBuf::from("peers.json");
            if let Err(e) = peer_manager_for_save.save_address_book(&peers_path) {
                log::warn!("Failed to save peers.json: {}", e);
            } else {
                log::debug!("Saved peer address book to peers.json");
            }
        }
    });

    // Spawn sync progress monitoring loop
    // Periodically checks if we're behind and need to sync blocks
    let sync_manager_for_monitoring = sync_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;

            match sync_manager_for_monitoring.get_sync_progress().await {
                Ok(progress) => {
                    if progress.blocks_behind > 0 {
                        log::info!(
                            "Sync: {} blocks behind ({}% complete)",
                            progress.blocks_behind,
                            progress.progress
                        );
                        // TODO Phase 3: Request blocks from peers here
                    }
                }
                Err(e) => log::warn!("Failed to get sync progress: {}", e),
            }
        }
    });

    // ==========================================
    // 🔴 THE FIX: BIND LISTENER (ASYNC WAY)
    // ==========================================
    use tokio::net::TcpListener;

    info!("Binding P2P Listener on {}...", listen);
    let listener = TcpListener::bind(listen)
        .await
        .map_err(|e| Error::Invalid(format!("p2p bind failed: {e}")))?;

    let local_addr = listener
        .local_addr()
        .map_err(|e| Error::Invalid(format!("failed to get local addr: {e}")))?;
    info!("P2P Server listening on {}", local_addr);

    // Bootstrap peer connections
    if let Some(peers) = bootstrap_peers {
        if peers.is_empty() {
            log::warn!(
                "No bootstrap peers configured. Node will wait for incoming connections only."
            );
        } else {
            log::info!("Bootstrapping to {} peer(s)...", peers.len());
            let peer_manager_for_bootstrap = peer_manager.clone();

            for peer_addr in peers {
                let addr: std::net::SocketAddr = match peer_addr.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        log::warn!("Invalid bootstrap peer address '{}': {}", peer_addr, e);
                        continue;
                    }
                };

                let pm = peer_manager_for_bootstrap.clone();
                tokio::spawn(async move {
                    // Timeout bootstrap connection after 30 seconds
                    let timeout_result = tokio::time::timeout(
                        tokio::time::Duration::from_secs(30),
                        pm.connect_peer(addr),
                    )
                    .await;

                    match timeout_result {
                        Ok(Ok(())) => {
                            log::info!("Successfully connected to bootstrap peer: {}", addr);
                        }
                        Ok(Err(e)) => {
                            log::warn!("Failed to connect to bootstrap peer {}: {}", addr, e);
                        }
                        Err(_) => {
                            log::warn!("Bootstrap connection to {} timed out after 30s", addr);
                        }
                    }
                });
            }
        }
    } else {
        log::warn!("No bootstrap peers configured. Node will wait for incoming connections only.");
    }

    // === PEER ACCEPT LOOP ===
    let worker_ctx_for_accept = worker_ctx.clone();
    let noise_config_for_accept = noise_config.clone();

    info!("Accepting peer connections on {}...", local_addr);

    loop {
        if let Ok((stream, peer_addr)) = listener.accept().await {
            let noise_config = noise_config_for_accept.clone();
            let ctx = worker_ctx_for_accept.clone();

            tokio::spawn(async move {
                // Convert tokio TcpStream to std TcpStream for Peer::new_inbound
                let std_stream = match stream.into_std() {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!(
                            "Failed to convert tokio stream to std stream for {}: {}",
                            peer_addr,
                            e
                        );
                        return;
                    }
                };

                // Create Peer from inbound stream using Noise handshake
                let magic = bitquan_network::protocol::network_magic(ctx.network_id);
                let peer_result =
                    bitquan_network::Peer::new_inbound(std_stream, peer_addr, magic, &noise_config);

                let peer = match peer_result {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("Peer handshake failed for {}: {}", peer_addr, e);
                        return;
                    }
                };

                // Run peer loop with worker context
                let result = crate::worker::run_peer_loop(peer, ctx).await;

                if let Err(e) = result {
                    // Log peer connection errors but continue accepting more
                    log::error!("Peer worker error for {}: {}", peer_addr, e);
                }
            });
        }
    }
}

/// P2P Client - connects to a peer
pub async fn p2p_connect(peer: &str, height: u64, network: NetworkId) -> Result<()> {
    use bitquan_network::{NoiseConfig, PeerManager};
    use std::sync::Arc;

    info!("BitQuan P2P Client");
    info!("Connecting to: {}", peer);
    info!("Our height: {}", height);
    debug!("Network: {:?}", network);

    // Generate Noise Protocol keypair for P2P encryption
    let noise_config = Arc::new(
        NoiseConfig::generate()
            .map_err(|e| Error::Invalid(format!("failed to generate noise config: {e}")))?,
    );
    info!(
        "P2P Encryption enabled (public key: {})",
        noise_config.public_key_hex()
    );

    let peer_manager = Arc::new(PeerManager::new(1, network, noise_config));
    // update_height() is async and returns ()
    peer_manager.update_height(height).await;

    let addr: SocketAddr = peer
        .parse()
        .map_err(|e| Error::Invalid(format!("invalid peer address: {e}")))?;

    info!("Connecting...");
    match peer_manager.connect_peer(addr).await {
        Ok(()) => {
            info!("Connected and handshake complete!");
            info!("Ready peers: {}", peer_manager.ready_peer_count().await);

            // Keep connection alive for a bit
            for i in 1..=5 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                debug!("Connection alive... {}/5", i);
            }

            info!("Test complete");
            Ok(())
        }
        Err(e) => {
            error!("Connection failed: {}", e);
            Err(Error::Invalid(format!("connection failed: {e}")))
        }
    }
}

/// RPC server options structure (copied from main.rs for use in p2p_server)
pub struct RpcServerOptions<'a> {
    /// RPC server listen address (e.g., "127.0.0.1:18445")
    pub listen: Option<&'a str>,
    /// RPC authentication username
    pub username: Option<&'a str>,
    /// RPC authentication password
    pub password: Option<&'a str>,
    /// Maximum request body size in bytes
    pub max_body_bytes: usize,
    /// Rate limit burst size (requests per burst)
    pub rl_burst: u32,
    /// Rate limit refill rate (requests per second)
    pub rl_refill_per_sec: u32,
    /// Connection cooldown in milliseconds
    pub conn_cooldown_ms: u64,
    /// Maximum header size in bytes
    pub max_header_bytes: usize,
    /// Header read timeout in milliseconds
    pub header_timeout_ms: u64,
    /// Trust proxy headers (X-Forwarded-For)
    pub trust_proxy: bool,
    /// Trusted CIDR ranges for proxy connections
    pub trusted_cidr: Vec<String>,
    /// Path to TLS certificate file
    pub tls_cert: Option<&'a str>,
    /// Path to TLS private key file
    pub tls_key: Option<&'a str>,
    /// Allow insecure TLS connections (development only)
    pub allow_insecure: bool,
    /// Path to JWT configuration file
    pub jwt_config_path: Option<&'a str>,
    /// JWT secret for authentication (overrides file)
    pub jwt_secret: Option<&'a str>,
}

impl Default for RpcServerOptions<'_> {
    fn default() -> Self {
        Self {
            listen: None,
            username: None,
            password: None,
            max_body_bytes: 100 * 1024 * 1024, // 100MB
            rl_burst: 500,
            rl_refill_per_sec: 100,
            conn_cooldown_ms: 1000,
            max_header_bytes: 16 * 1024, // 16KB
            header_timeout_ms: 10000,
            trust_proxy: false,
            trusted_cidr: vec![],
            tls_cert: None,
            tls_key: None,
            allow_insecure: false,
            jwt_config_path: None,
            jwt_secret: None,
        }
    }
}

impl<'a> From<&'a clap::ArgMatches> for RpcServerOptions<'a> {
    fn from(matches: &'a clap::ArgMatches) -> Self {
        Self {
            listen: matches.get_one::<String>("rpc-listen").map(|s| s.as_str()),
            username: matches
                .get_one::<String>("rpc-username")
                .map(|s| s.as_str()),
            password: matches
                .get_one::<String>("rpc-password")
                .map(|s| s.as_str()),
            max_body_bytes: matches
                .get_one::<usize>("rpc-max-body-bytes")
                .copied()
                .unwrap_or(100 * 1024 * 1024),
            rl_burst: matches
                .get_one::<usize>("rpc-rl-burst")
                .copied()
                .unwrap_or(500) as u32,
            rl_refill_per_sec: matches
                .get_one::<usize>("rpc-rl-refill-per-sec")
                .copied()
                .unwrap_or(100) as u32,
            conn_cooldown_ms: matches
                .get_one::<u64>("rpc-conn-cooldown-ms")
                .copied()
                .unwrap_or(1000),
            max_header_bytes: matches
                .get_one::<usize>("rpc-max-header-bytes")
                .copied()
                .unwrap_or(16 * 1024),
            header_timeout_ms: matches
                .get_one::<u64>("rpc-header-timeout-ms")
                .copied()
                .unwrap_or(10000),
            trust_proxy: matches.get_flag("rpc-trust-proxy"),
            trusted_cidr: matches
                .get_many::<String>("rpc-trusted-cidr")
                .unwrap_or_default()
                .cloned()
                .collect(),
            tls_cert: matches
                .get_one::<String>("rpc-tls-cert")
                .map(|s| s.as_str()),
            tls_key: matches.get_one::<String>("rpc-tls-key").map(|s| s.as_str()),
            allow_insecure: matches.get_flag("rpc-allow-insecure"),
            jwt_config_path: matches.get_one::<String>("jwt-config").map(|s| s.as_str()),
            jwt_secret: matches.get_one::<String>("jwt-secret").map(|s| s.as_str()),
        }
    }
}

/// Helper function to read password from stdin
pub fn read_password_from_stdin() -> Result<String> {
    rpassword::prompt_password("Password: ")
        .map_err(|e| Error::Invalid(format!("Failed to read password: {}", e)))
}
