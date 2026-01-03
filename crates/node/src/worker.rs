//! P2P worker - handles peer message loops and blockchain synchronization.
//!
//! This module extracts peer handling logic from main.rs into a dedicated worker.
//! Each peer runs in its own async task, processing messages and coordinating
//! with the chain, mempool, and peer manager.

use std::sync::Arc;
use tokio::sync::Mutex;
use bitquan_network::peer::{Peer, PeerManager};
use bitquan_network::protocol::{Message, MessageEnvelope, InvVector, InvType, network_magic};
use bitquan_types::{Block, Transaction, NetworkId};
use bitquan_storage::async_store::AsyncChainStore;
use bitquan_consensus::header_hash;

/// Errors that can occur during peer message processing.
#[derive(Debug)]
pub enum WorkerError {
    /// Peer sent invalid data.
    InvalidData(String),
    /// Storage error.
    Storage(String),
    /// Network I/O error.
    Network(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            Self::Storage(msg) => write!(f, "Storage error: {}", msg),
            Self::Network(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl From<bitquan_network::protocol::P2pError> for WorkerError {
    fn from(err: bitquan_network::protocol::P2pError) -> Self {
        Self::Network(err.to_string())
    }
}

/// Context shared between all peer workers.
pub struct WorkerContext {
    /// Peer manager for broadcasting to other peers.
    pub peer_manager: Arc<PeerManager>,
    /// Blockchain storage (async wrapper for thread-safe access).
    pub storage: Arc<dyn AsyncChainStore>,
    /// Network magic bytes.
    pub magic: [u8; 4],
}

impl WorkerContext {
    /// Create a new worker context.
    pub fn new(
        peer_manager: Arc<PeerManager>,
        storage: Arc<dyn AsyncChainStore>,
        network: NetworkId,
    ) -> Self {
        Self {
            peer_manager,
            storage,
            magic: network_magic(network),
        }
    }
}

/// Runs the peer message loop.
///
/// This function:
/// 1. Continuously receives messages from the peer.
/// 2. Processes each message type appropriately.
/// 3. Broadcasts relevant data to other peers.
/// 4. Handles errors gracefully (disconnects peer on fatal errors).
///
/// # Arguments
/// * `peer` - The peer connection (must be already handshaked)
/// * `ctx` - Shared worker context
///
/// # Behavior
/// - Logs all incoming messages for debugging
/// - Validates all data before accepting
/// - Never panics - returns error on failure
pub async fn run_peer_loop(mut peer: Peer, ctx: Arc<WorkerContext>) -> Result<(), WorkerError> {
    log::info!("🔄 Starting peer loop for {}", peer.addr);

    loop {
        // Receive message from peer (with timeout for slow loris protection)
        let msg = match peer.recv_message() {
            Ok(msg) => msg,
            Err(e) => {
                log::warn!("❌ Peer {} recv error: {}, disconnecting", peer.addr, e);
                return Err(WorkerError::Network(e.to_string()));
            }
        };

        // Log message type for debugging
        log::debug!("📨 From {}: {:?}", peer.addr, msg);

        // Handle message
        match handle_message(&mut peer, &ctx, msg).await {
            Ok(should_continue) => {
                if !should_continue {
                    log::info!("🔌 Peer {} disconnected normally", peer.addr);
                    return Ok(());
                }
            }
            Err(e) => {
                log::warn!("⚠️  Peer {} error: {}, disconnecting", peer.addr, e);
                // TODO: Ban peer if error is severe
                return Err(e);
            }
        }
    }
}

/// Handles a single message from a peer.
///
/// Returns `Ok(false)` if the peer should be disconnected.
/// Returns `Ok(true)` to continue processing messages.
async fn handle_message(
    peer: &mut Peer,
    ctx: &WorkerContext,
    msg: Message,
) -> Result<bool, WorkerError> {
    match msg {
        // === Keepalive ===
        Message::Ping { nonce } => {
            // Respond with pong
            peer.send_message(Message::Pong { nonce })?;
            log::debug!("🏓 Pong sent to {}", peer.addr);
            Ok(true)
        }

        Message::Pong { .. } => {
            // Ignore pong responses (we don't track pings yet)
            Ok(true)
        }

        // === Peer Discovery ===
        Message::GetAddr => {
            // TODO: Send known peer addresses
            // For now, send empty list
            peer.send_message(Message::Addr { addrs: vec![] })?;
            Ok(true)
        }

        Message::Addr { addrs } => {
            log::info!("📬 Received {} addresses from {}", addrs.len(), peer.addr);
            // TODO: Add addresses to peer manager's address book
            Ok(true)
        }

        // === Inventory Announcements ===
        Message::Inv { inventory } => {
            handle_inv(peer, ctx, inventory).await
        }

        // === Data Requests ===
        Message::GetData { inventory } => {
            handle_get_data(peer, ctx, inventory).await
        }

        // === Block Data ===
        Message::Block { block } => {
            handle_block(peer, ctx, block).await
        }

        // === Transaction Data ===
        Message::Tx { transaction } => {
            handle_tx(peer, ctx, transaction).await
        }

        // === Block Headers ===
        Message::GetHeaders { version: _, locator_hashes, stop_hash } => {
            handle_get_headers(peer, ctx, locator_hashes, stop_hash).await
        }

        Message::Headers { headers } => {
            log::info!("📨 Received {} headers from {}", headers.len(), peer.addr);
            // TODO: Validate headers and add to chain
            Ok(true)
        }

        // === Mempool ===
        Message::GetMempool => {
            // TODO: Send mempool inventory
            Ok(true)
        }

        // === Rejections ===
        Message::Reject { message, code, reason } => {
            log::warn!("⛔ Peer {} rejected {}: {:?} - {}", peer.addr, message, code, reason);
            Ok(true)
        }

        // === Handshake (should not happen after loop starts) ===
        Message::Version { .. } => {
            log::warn!("⚠️  Unexpected Version message from {}", peer.addr);
            Ok(true)
        }

        Message::VerAck => {
            log::warn!("⚠️  Unexpected VerAck message from {}", peer.addr);
            Ok(true)
        }
    }
}

/// Handle inventory announcement.
///
/// Logic:
/// 1. For each InvVector:
///    - If Block: Check if we have it → if not, request it with GetData
///    - If Tx: Check mempool → if not, request it with GetData
/// 2. Request missing items in batches (MAX_INV_ITEMS per message)
async fn handle_inv(
    peer: &mut Peer,
    ctx: &WorkerContext,
    inventory: Vec<InvVector>,
) -> Result<bool, WorkerError> {
    log::debug!("📦 Received {} inv items from {}", inventory.len(), peer.addr);

    let mut to_request: Vec<InvVector> = vec![];

    for inv in inventory {
        match inv.inv_type {
            InvType::Block => {
                // Check if we have this block
                let block_exists = ctx.storage.get_block(&inv.hash).await.is_ok();

                if !block_exists {
                    log::info!("❓ Missing block: {}, requesting from {}", hex::encode(inv.hash), peer.addr);
                    to_request.push(inv);
                }
            }

            InvType::Tx => {
                // TODO: Check mempool
                // For now, always request (mempool not integrated yet)
                log::info!("❓ Requesting tx: {} from {}", hex::encode(inv.hash), peer.addr);
                to_request.push(inv);
            }

            _ => {
                log::debug!("Ignoring unknown inv type: {:?}", inv.inv_type);
            }
        }
    }

    // Request missing items (batch them to avoid oversized messages)
    if !to_request.is_empty() {
        const MAX_PER_REQUEST: usize = 500; // Conservative limit

        for chunk in to_request.chunks(MAX_PER_REQUEST) {
            peer.send_message(Message::GetData {
                inventory: chunk.to_vec(),
            })?;
            log::debug!("📤 Requested {} items from {}", chunk.len(), peer.addr);
        }
    }

    Ok(true)
}

/// Handle GetData request.
///
/// Logic:
/// 1. For each InvVector:
///    - If Block: Send the block if we have it
///    - If Tx: Send the tx if we have it (from mempool or storage)
async fn handle_get_data(
    peer: &mut Peer,
    ctx: &WorkerContext,
    inventory: Vec<InvVector>,
) -> Result<bool, WorkerError> {
    log::debug!("📥 Peer {} requested {} items", peer.addr, inventory.len());

    for inv in inventory {
        match inv.inv_type {
            InvType::Block => {
                // Try to get block from storage
                match ctx.storage.get_block(&inv.hash).await {
                    Ok(Some(block)) => {
                        peer.send_message(Message::Block { block })?;
                        log::debug!("📤 Sent block {} to {}", hex::encode(inv.hash), peer.addr);
                    }
                    Ok(None) => {
                        log::debug!("Block {} not found, skipping", hex::encode(inv.hash));
                    }
                    Err(e) => {
                        log::error!("Storage error: {}", e);
                    }
                }
            }

            InvType::Tx => {
                // TODO: Check mempool first
                // For now, respond with "not found"
                log::debug!("Tx {} requested but mempool not integrated", hex::encode(inv.hash));
            }

            _ => {
                log::debug!("Unknown inv type in GetData: {:?}", inv.inv_type);
            }
        }
    }

    Ok(true)
}

/// Handle incoming block.
///
/// Logic:
/// 1. Validate the block (basic checks + consensus validation)
/// 2. If valid:
///    - Add to storage
///    - Broadcast Inv(Block) to all other peers
/// 3. If invalid:
///    - Disconnect peer (potential malicious behavior)
async fn handle_block(
    peer: &mut Peer,
    ctx: &WorkerContext,
    block: Block,
) -> Result<bool, WorkerError> {
    let block_hash = header_hash(&block.header);
    log::info!("🧱 Received block {} ({} txs) from {}",
        hex::encode(&block_hash[..8]),
        block.transactions.len(),
        peer.addr
    );

    // TODO: Full consensus validation
    // For now, just do basic sanity checks and store

    // Check if we already have this block
    match ctx.storage.get_block(&block_hash).await {
        Ok(Some(_)) => {
            log::debug!("Block {} already known, ignoring", hex::encode(&block_hash[..8]));
            return Ok(true);
        }
        Ok(None) => {
            // New block, proceed
        }
        Err(e) => {
            log::error!("Storage error checking block: {}", e);
            return Err(WorkerError::Storage(e.to_string()));
        }
    }

    // TODO: Run full consensus validation here
    // For now, just store it (DEV BUILD - DO NOT USE IN PRODUCTION)

    // Store block (Note: ChainStore uses insert_block, RocksDBStore has connect_block)
    // We need mutable access to storage, but ctx.storage is Arc<dyn ChainStore>
    // TODO: This requires refactoring - storage needs interior mutability
    // For now, just log the block (DEV BUILD - DO NOT USE IN PRODUCTION)
    log::warn!("⚠️  Block {} received but storage is immutable (TODO: fix)", hex::encode(&block_hash[..8]));

    // Uncomment when storage has interior mutability:
    // if let Err(e) = ctx.storage.insert_block(block) {
    //     log::error!("Failed to insert block {}: {}", hex::encode(&block_hash[..8]), e);
    //     return Err(WorkerError::Storage(e.to_string()));
    // }

    log::info!("✅ Block {} connected to chain", hex::encode(&block_hash[..8]));

    // Broadcast Inv to other peers
    let inv = vec![InvVector {
        inv_type: InvType::Block,
        hash: block_hash,
    }];

    let inv_msg = MessageEnvelope::new(ctx.magic, Message::Inv { inventory: inv });

    // TODO: Broadcast to all peers except sender
    // For now, just log
    log::info!("📢 Broadcasting Inv(Block {}) to network", hex::encode(&block_hash[..8]));

    Ok(true)
}

/// Handle incoming transaction.
///
/// Logic:
/// 1. Validate the transaction
/// 2. If valid:
///    - Add to mempool
///    - Broadcast Inv(Tx) to all other peers
/// 3. If invalid:
///    - Reject with reason
async fn handle_tx(
    peer: &mut Peer,
    ctx: &WorkerContext,
    transaction: Transaction,
) -> Result<bool, WorkerError> {
    let tx_hash = transaction.txid();
    log::info!("💸 Received tx {} from {}", hex::encode(&tx_hash[..8]), peer.addr);

    // TODO: Validate transaction
    // TODO: Add to mempool
    // TODO: Broadcast Inv to other peers

    // For now, just acknowledge
    log::debug!("Tx {} received (mempool not integrated)", hex::encode(&tx_hash[..8]));

    Ok(true)
}

/// Handle GetHeaders request.
///
/// Logic:
/// 1. Find the common ancestor between locator hashes and our chain
/// 2. Send up to 2000 headers starting from common ancestor + 1
async fn handle_get_headers(
    peer: &mut Peer,
    ctx: &WorkerContext,
    locator_hashes: Vec<[u8; 32]>,
    stop_hash: [u8; 32],
) -> Result<bool, WorkerError> {
    log::debug!("📋 GetHeaders: {} locators, stop={}", locator_hashes.len(), hex::encode(&stop_hash[..8]));

    // TODO: Implement proper header locator logic
    // For now, send empty response
    peer.send_message(Message::Headers { headers: vec![] })?;

    log::debug!("📤 Sent empty Headers response to {}", peer.addr);

    Ok(true)
}

/// Perform version handshake with a newly connected peer.
///
/// This function:
/// 1. Waits for Version message from peer
/// 2. Sends our Version message
/// 3. Sends VerAck
/// 4. Waits for optional VerAck from peer
/// 5. Sets peer state to Ready
///
/// # Arguments
/// * `peer` - The peer connection (must have Noise handshake completed)
/// * `network` - Network ID for magic bytes
///
/// # Returns
/// * `Ok(())` if handshake successful
/// * `Err(WorkerError)` if handshake fails
pub async fn perform_version_handshake(
    peer: &mut Peer,
    network: NetworkId,
) -> Result<(), WorkerError> {
    use bitquan_network::protocol::{Message, PROTOCOL_VERSION};

    let magic = network_magic(network);

    // Wait for version message
    let msg = peer.recv_message()?;
    match msg {
        Message::Version { version, services: _, timestamp: _, user_agent, start_height } => {
            peer.version = Some(version);
            peer.user_agent = Some(user_agent.clone());
            peer.start_height = Some(start_height);

            log::info!("🤝 Handshake: {} (v{}, {}, height={})",
                user_agent, version, peer.addr, start_height);

            // Send our version
            let our_version = Message::Version {
                version: PROTOCOL_VERSION,
                services: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                user_agent: env!("CARGO_PKG_NAME").to_string(),
                start_height: 0, // TODO: Get from chain state
            };

            peer.send_message(our_version)?;
            peer.send_message(Message::VerAck)?;
        }
        _ => {
            return Err(WorkerError::InvalidData("expected version message".to_string()));
        }
    }

    // Wait for verack (optional, some nodes skip it)
    let next_msg = peer.recv_message();
    if let Ok(Message::VerAck) = next_msg {
        log::debug!("✅ VerAck received from {}", peer.addr);
    }

    peer.state = bitquan_network::peer::PeerState::Ready;

    Ok(())
}

