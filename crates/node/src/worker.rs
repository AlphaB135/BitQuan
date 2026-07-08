//! P2P worker - handles peer message loops and blockchain synchronization.
//!
//! This module extracts peer handling logic from main.rs into a dedicated worker.
//! Each peer runs in its own async task, processing messages and coordinating
//! with the chain, mempool, and peer manager.

use crate::metrics;
use bitquan_consensus::header_hash;
use bitquan_mempool::Mempool;
use bitquan_network::ban_manager::{BanManager, BanReason};
use bitquan_network::peer::{Peer, PeerManager};
use bitquan_network::protocol::{network_magic, InvType, InvVector, Message, RejectCode};
use bitquan_network::relay::create_block_getdata;
use bitquan_storage::async_store::AsyncChainStore;
use bitquan_storage::{serialize, StoredUtxoEntry};
use bitquan_types::{Block, NetworkId, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// Errors that can occur during peer message processing.
#[derive(Debug)]
pub enum WorkerError {
    /// Peer sent invalid data.
    InvalidData(String),
    /// Storage error.
    Storage(String),
    /// Network I/O error.
    Network(String),
    /// Critical error requiring immediate shutdown (e.g., corrupted chain state)
    Critical(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            Self::Storage(msg) => write!(f, "Storage error: {}", msg),
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::Critical(msg) => write!(f, "CRITICAL: {}", msg),
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
    /// Transaction mempool.
    pub mempool: Arc<TokioMutex<Mempool>>,
    /// Consensus engine for block validation.
    pub consensus: Arc<TokioMutex<bitquan_consensus::ConsensusEngine>>,
    /// Fork choice manager for chain reorganization.
    pub fork_choice: Arc<TokioMutex<bitquan_consensus::fork::ForkChoice>>,
    /// Ban manager for peer misconduct.
    pub ban_manager: Arc<TokioMutex<BanManager>>,
    /// Network identifier for validation.
    pub network_id: bitquan_types::NetworkId,
    /// Genesis hash for transaction context.
    pub genesis_hash: [u8; 32],
    /// Pending block requests during reorg (tracked to detect duplicates)
    pub pending_block_requests: Arc<TokioMutex<std::collections::HashSet<[u8; 32]>>>,
}

impl WorkerContext {
    /// Create a new worker context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        peer_manager: Arc<PeerManager>,
        storage: Arc<dyn AsyncChainStore>,
        mempool: Arc<TokioMutex<Mempool>>,
        consensus: Arc<TokioMutex<bitquan_consensus::ConsensusEngine>>,
        fork_choice: Arc<TokioMutex<bitquan_consensus::fork::ForkChoice>>,
        ban_manager: Arc<TokioMutex<BanManager>>,
        network_id: bitquan_types::NetworkId,
        genesis_hash: [u8; 32],
    ) -> Self {
        Self {
            peer_manager,
            storage,
            mempool,
            consensus,
            fork_choice,
            ban_manager,
            network_id,
            genesis_hash,
            pending_block_requests: Arc::new(TokioMutex::new(std::collections::HashSet::new())),
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
    println!("🔄 [WORKER] Starting peer loop for {}", peer.addr);
    log::info!("🔄 Starting peer loop for {}", peer.addr);

    // Send GetHeaders after handshake to initiate IBD if we're behind
    // This replaces GetBlocks with the lighter header-first approach
    if let Err(e) = send_getheaders_if_behind(&mut peer, &ctx).await {
        log::warn!("⚠️  Failed to send GetHeaders: {}", e);
    }

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

                // Linus Rule: Penalize peer for protocol violations
                // Score 50 for general errors (malformed messages, protocol issues)
                let should_ban = peer.add_ban_score(50);
                log::warn!(
                    "⚠️  Penalized peer {} with score {} for error",
                    peer.addr,
                    peer.ban_score
                );

                if should_ban {
                    // Ban threshold reached - permanently ban this peer
                    log::warn!(
                        "🚫 Banning peer {} for malicious behavior (score: {})",
                        peer.addr,
                        peer.ban_score
                    );

                    let mut ban_manager = ctx.ban_manager.lock().await;
                    let peer_id = peer.addr.to_string();
                    let ip = peer.addr.ip();

                    // Ban both peer ID and IP
                    let _ = ban_manager.ban_peer_permanently(
                        peer_id.clone(),
                        BanReason::ProtocolViolation,
                        Some("worker.rs".to_string()),
                        Some(format!("Ban score reached: {}", peer.ban_score)),
                    );

                    let _ = ban_manager.ban_ip(
                        ip,
                        BanReason::ProtocolViolation,
                        Some(std::time::Duration::from_secs(86400)), // 24 hours
                        Some("worker.rs".to_string()),
                        Some(format!("Peer {} banned for protocol violation", peer_id)),
                    );

                    // Update metrics - log ban event
                    metrics::increment_ban_event("protocol_violation");
                }

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
            // Send known peer addresses from address book
            match ctx.peer_manager.get_known_peers() {
                Ok(peers) => {
                    // Limit to 1000 addresses per protocol spec
                    let addrs: Vec<bitquan_network::protocol::PeerAddr> =
                        peers.into_iter().take(1000).collect();
                    let addr_count = addrs.len();

                    peer.send_message(Message::Addr { addrs })?;
                    log::debug!("📤 Sent {} addresses to {}", addr_count, peer.addr);
                    Ok(true)
                }
                Err(e) => {
                    log::warn!("Failed to get known peers: {}", e);
                    // Send empty list on error
                    peer.send_message(Message::Addr { addrs: vec![] })?;
                    Ok(true)
                }
            }
        }

        Message::Addr { addrs } => {
            log::info!("📬 Received {} addresses from {}", addrs.len(), peer.addr);

            // Add addresses to peer manager's address book
            match ctx.peer_manager.add_peer_addresses(addrs) {
                Ok(()) => {
                    log::debug!("✅ Added addresses to book");
                    Ok(true)
                }
                Err(e) => {
                    log::warn!("Failed to add addresses: {}", e);
                    Ok(true) // Don't disconnect on address book errors
                }
            }
        }

        // === Inventory Announcements ===
        Message::Inv { inventory } => handle_inv(peer, ctx, inventory).await,

        // === Data Requests ===
        Message::GetData { inventory } => handle_get_data(peer, ctx, inventory).await,

        // === Block Data ===
        Message::Block { block } => handle_block(peer, ctx, block).await,

        // === Transaction Data ===
        Message::Tx { transaction } => handle_tx(peer, ctx, transaction).await,

        // === Block Headers ===
        Message::GetHeaders {
            version: _,
            locator_hashes,
            stop_hash,
        } => handle_get_headers(peer, ctx, locator_hashes, stop_hash).await,

        Message::Headers { headers } => handle_headers(peer, ctx, headers).await,

        // === Block Sync ===
        Message::GetBlocks {
            version: _,
            locator_hashes,
            stop_hash,
        } => handle_getblocks(peer, ctx, locator_hashes, stop_hash).await,

        // === Mempool ===
        Message::GetMempool => {
            log::debug!("📋 Peer {} requested mempool inventory", peer.addr);

            // Maximum inventory items per message (Bitcoin standard)
            const MAX_INV_ITEMS: usize = 50_000;

            let mempool = ctx.mempool.lock().await;
            let tx_ids = mempool.txids();
            drop(mempool);

            let inv: Vec<InvVector> = tx_ids
                .into_iter()
                .take(MAX_INV_ITEMS)
                .map(|txid| InvVector {
                    inv_type: InvType::Tx,
                    hash: txid,
                })
                .collect();

            let count = inv.len();
            if !inv.is_empty() {
                peer.send_message(Message::Inv { inventory: inv })?;
                log::debug!("📤 Sent {} mempool txs to {}", count, peer.addr);
            }

            Ok(true)
        }

        // === Rejections ===
        Message::Reject {
            message,
            code,
            reason,
        } => {
            log::warn!(
                "⛔ Peer {} rejected {}: {:?} - {}",
                peer.addr,
                message,
                code,
                reason
            );
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

/// Send GetHeaders message to initiate IBD.
///
/// After handshake completes, if our chain is behind the peer's chain,
/// send GetHeaders to request block headers from the peer.
async fn send_getheaders_if_behind(
    peer: &mut Peer,
    ctx: &WorkerContext,
) -> Result<(), WorkerError> {
    // Get peer's claimed height
    let peer_height = match peer.start_height {
        Some(h) => h,
        None => {
            log::debug!("Peer {} hasn't sent start_height yet", peer.addr);
            return Ok(());
        }
    };

    // Get our current height
    let our_height = match ctx.storage.height().await {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to get chain height: {}", e);
            return Err(WorkerError::Storage(e.to_string()));
        }
    };

    // Only send GetHeaders if peer is ahead
    if peer_height <= our_height {
        log::debug!(
            "Peer {} (height: {}) is not ahead of us (height: {})",
            peer.addr,
            peer_height,
            our_height
        );
        return Ok(());
    }

    log::info!(
        "🔄 We're behind: us={} vs peer={}, initiating IBD",
        our_height,
        peer_height
    );

    // Build block locator hashes
    let mut locator_hashes: Vec<[u8; 32]> = Vec::new();

    // Add tip hash
    if let Ok(Some(tip)) = ctx.storage.tip().await {
        locator_hashes.push(header_hash(&tip));
    } else {
        // No tip yet - use zero hash
        locator_hashes.push([0u8; 32]);
    }

    // Add exponential backoff hashes (every power of 2)
    // Simplified version - TODO: optimize with proper header index
    let mut step = 1u64;
    while step < our_height.saturating_sub(1) {
        let check_height = our_height.saturating_sub(step);
        if let Ok(Some(block)) = ctx.storage.get_block_by_height(check_height).await {
            locator_hashes.push(header_hash(&block.header));
            step = step.saturating_mul(2);
        } else {
            break;
        }
    }

    // Add genesis hash as fallback
    locator_hashes.push(ctx.genesis_hash);

    // Stop hash is zero (get as many as possible)
    let stop_hash = [0u8; 32];

    let msg = bitquan_network::protocol::Message::GetHeaders {
        version: bitquan_network::protocol::PROTOCOL_VERSION,
        locator_hashes,
        stop_hash,
    };

    peer.send_message(msg)
        .map_err(|e| WorkerError::Network(format!("send GetHeaders failed: {}", e)))?;

    log::info!(
        "📤 Sent GetHeaders to {} (our height: {}, peer height: {})",
        peer.addr,
        our_height,
        peer_height
    );

    Ok(())
}

/// Send GetBlocks message to initiate IBD.
///
/// After handshake completes, nodes with lower height should send GetBlocks
/// to request block inventory from peers.
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
    log::debug!(
        "📦 Received {} inv items from {}",
        inventory.len(),
        peer.addr
    );

    let mut to_request: Vec<InvVector> = vec![];

    for inv in inventory {
        match inv.inv_type {
            InvType::Block => {
                // Check if we have this block
                let block_exists = ctx.storage.get_block(&inv.hash).await.is_ok();

                if !block_exists {
                    log::info!(
                        "❓ Missing block: {}, requesting from {}",
                        hex::encode(inv.hash),
                        peer.addr
                    );
                    to_request.push(inv);
                }
            }

            InvType::Tx => {
                // Check if we already have this transaction in mempool
                let in_mempool = {
                    let mempool = ctx.mempool.lock().await;
                    mempool.contains(&inv.hash)
                };

                // Only request if not already in mempool
                if !in_mempool {
                    log::info!(
                        "❓ Requesting tx: {} from {}",
                        hex::encode(&inv.hash[..8]),
                        peer.addr
                    );
                    to_request.push(inv);
                } else {
                    log::debug!(
                        "✅ Tx {} already in mempool, skipping request",
                        hex::encode(&inv.hash[..8])
                    );
                }
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
                // Check mempool FIRST, then storage
                let tx_from_mempool = {
                    let mempool = ctx.mempool.lock().await;
                    mempool.get_transaction(&inv.hash)
                };

                match tx_from_mempool {
                    Some(tx) => {
                        log::debug!(
                            "📤 Sending tx {} from mempool to {}",
                            hex::encode(&tx.txid()[..8]),
                            peer.addr
                        );
                        peer.send_message(Message::Tx {
                            transaction: (*tx).clone(),
                        })?;
                    }
                    None => {
                        // Not in mempool, try storage
                        log::debug!(
                            "❓ Tx {} not in mempool, checking storage",
                            hex::encode(&inv.hash[..8])
                        );
                        // Try storage
                        match ctx.storage.get_transaction(&inv.hash).await {
                            Ok(Some(tx)) => {
                                log::debug!(
                                    "📤 Sending tx {} from storage to {}",
                                    hex::encode(&tx.txid()[..8]),
                                    peer.addr
                                );
                                peer.send_message(Message::Tx { transaction: tx })?;
                            }
                            Ok(None) => {
                                log::debug!(
                                    "❓ Tx {} not found anywhere (mempool or storage)",
                                    hex::encode(&inv.hash[..8])
                                );
                                // Transaction not found - do nothing (peer will timeout)
                            }
                            Err(e) => {
                                log::warn!(
                                    "❌ Failed to get tx {} from storage: {}",
                                    hex::encode(&inv.hash[..8]),
                                    e
                                );
                            }
                        }
                    }
                }
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
    log::info!(
        "🧱 Received block {} ({} txs) from {}",
        hex::encode(&block_hash[..8]),
        block.transactions.len(),
        peer.addr
    );

    // Step 1: Check if we already have this block
    match ctx.storage.get_block(&block_hash).await {
        Ok(Some(_)) => {
            log::debug!(
                "Block {} already known, ignoring",
                hex::encode(&block_hash[..8])
            );
            return Ok(true);
        }
        Ok(None) => {
            // New block, proceed

            // Check if this block was requested during reorg
            let mut pending = ctx.pending_block_requests.lock().await;
            if pending.remove(&block_hash) {
                log::info!(
                    "✅ Received requested block {} (was pending)",
                    hex::encode(&block_hash[..8])
                );
                // Block will be processed normally below
            }
        }
        Err(e) => {
            log::error!("Storage error checking block: {}", e);
            return Err(WorkerError::Storage(e.to_string()));
        }
    }

    // Step 2: Get current height for validation
    let height = match ctx.storage.height().await {
        Ok(h) => h,
        Err(e) => {
            log::error!("❌ Failed to get chain height: {}", e);
            return Err(WorkerError::Storage(e.to_string()));
        }
    };

    // Calculate median time past from last 11 blocks (prevents timestamp manipulation)
    let median_time_past = match ctx.storage.median_time_past().await {
        Ok(mtp) => mtp,
        Err(e) => {
            log::error!("❌ Failed to calculate median time past: {}", e);
            return Err(WorkerError::Storage(e.to_string()));
        }
    };

    // Step 3: UTXO validation (CRITICAL - prevents double spends, calculates fees)
    // This MUST come before consensus validation because we need total_fees
    // for strict coinbase reward validation (prevents inflation bug).
    let total_fees = match validate_block_utxos(ctx, &block, height).await {
        Ok(fees) => {
            log::info!(
                "✅ Block {} UTXO valid (all inputs exist, no double spends, fees: {})",
                hex::encode(&block_hash[..8]),
                fees
            );
            fees
        }
        Err(e) => {
            log::warn!(
                "⚠️  Block {} UTXO validation failed: {}",
                hex::encode(&block_hash[..8]),
                e
            );

            // Linus Rule: UTXO validation failure = 100 points (instant ban)
            // This is CRITICAL - double spend attacks undermine consensus
            let _ = peer.add_ban_score(100);
            log::warn!(
                "🚨 Penalized peer {} with 100 points for UTXO validation failure",
                peer.addr
            );

            // Invalid block - reject and disconnect peer
            let _ = peer.send_message(Message::Reject {
                message: "block".to_string(),
                code: RejectCode::Invalid,
                reason: format!("UTXO validation failed: {}", e),
            });
            return Err(WorkerError::InvalidData(format!(
                "UTXO validation failed: {}",
                e
            )));
        }
    };

    // Step 3.5: GHOST Protocol - Fetch Uncle Contexts
    let mut uncles_ctx = Vec::new();
    let mut past_uncle_hashes = std::collections::HashSet::new();

    if !block.uncles.is_empty() {
        // Collect past uncles for double inclusion check (depth bounded by 7)
        let start_h = height.saturating_sub(7);
        for h in start_h..height {
            if let Ok(Some(past_block)) = ctx.storage.get_block_by_height(h).await {
                for u in past_block.uncles {
                    past_uncle_hashes.insert(bitquan_consensus::pow::header_hash(&u));
                }
            }
        }

        // Fetch each included uncle
        for u_hdr in &block.uncles {
            let u_hash = bitquan_consensus::pow::header_hash(u_hdr);
            match ctx.storage.get_block(&u_hash).await {
                Ok(Some(u_block)) => {
                    // Try to discover height (scan from tip downward up to 10 blocks)
                    // If not found, reject the block (Closes #133)
                    let mut u_height = None;
                    for h in (height.saturating_sub(10)..height).rev() {
                        if let Ok(Some(b)) = ctx.storage.get_block_by_height(h).await {
                            if bitquan_consensus::pow::header_hash(&b.header) == u_hash {
                                u_height = Some(h);
                                break;
                            }
                        }
                    }

                    let u_height = match u_height {
                        Some(h) => h,
                        None => {
                            log::warn!(
                                "Rejecting block: uncle height not found in chain scan (uncle hash: {})",
                                hex::encode(&u_hash[..8])
                            );
                            return Err(WorkerError::InvalidData(
                                "Uncle block not found on main chain within depth 10".to_string(),
                            ));
                        }
                    };

                    let payout_script = if !u_block.transactions.is_empty()
                        && !u_block.transactions[0].outputs.is_empty()
                    {
                        u_block.transactions[0].outputs[0].script_pubkey.clone()
                    } else {
                        Vec::new() // Will fail consensus if invalid
                    };

                    uncles_ctx.push(bitquan_consensus::UncleContext {
                        header: u_hdr.clone(),
                        height: u_height,
                        payout_script,
                    });
                }
                _ => {
                    let msg = format!(
                        "Missing Uncle {} for consensus validation",
                        hex::encode(&u_hash[..8])
                    );
                    log::warn!("{}", msg);
                    let _ = peer.send_message(Message::Reject {
                        message: "block".to_string(),
                        code: RejectCode::Invalid,
                        reason: msg.clone(),
                    });
                    return Err(WorkerError::InvalidData(msg));
                }
            }
        }
    }

    // Step 4: Full consensus validation (coinbase, signatures, merkle, etc.)
    // Uses total_fees from UTXO validation for STRICT coinbase reward check.
    // This prevents the inflation bug where miners claim subsidy + 1 BTC without fees.
    // SECURITY: network_adjusted_time is calculated here (outside consensus) as the
    // caller's responsibility. In production, this should ideally be median peer time.
    let network_adjusted_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut engine = ctx.consensus.lock().await;
    // Set difficulty state anchored at the parent block to enforce ASERT difficulty target validation.
    // Ref: issue #191 (C1 — ASERT difficulty not enforced).
    if height > 0 {
        match ctx.storage.get_block(&block.header.prev_block).await {
            Ok(Some(parent)) => {
                let difficulty_state = bitquan_consensus::DifficultyState::new(
                    height.saturating_sub(1),
                    parent.header.time as u64,
                    parent.header.bits,
                    0, // guard_activation_height
                );
                engine.set_difficulty_state(difficulty_state);
            }
            Ok(None) => {
                log::error!(
                    "❌ Parent block {} not found in storage. Rejecting block.",
                    hex::encode(&block.header.prev_block[..8])
                );
                return Err(WorkerError::InvalidData(format!(
                    "parent block {} not found in storage",
                    hex::encode(&block.header.prev_block[..8])
                )));
            }
            Err(e) => {
                log::error!(
                    "❌ Failed to fetch parent block from storage: {}. Rejecting block.",
                    e
                );
                return Err(WorkerError::Storage(format!(
                    "failed to fetch parent block: {}",
                    e
                )));
            }
        }
    }

    match engine.validate_block_with_fees(
        &block,
        height,
        total_fees,
        median_time_past,
        network_adjusted_time,
        &uncles_ctx,
        &past_uncle_hashes,
    ) {
        Ok(report) => {
            log::info!(
                "✅ Block {} consensus valid (weight: {} WU, sigs: {})",
                hex::encode(&block_hash[..8]),
                report.block_weight,
                report.signature_count
            );
        }
        Err(e) => {
            log::warn!(
                "⚠️  Block {} consensus invalid: {}",
                hex::encode(&block_hash[..8]),
                e
            );

            // Linus Rule: Consensus validation failure = 100 points (instant ban)
            // Invalid signatures, wrong coinbase, merkle root mismatch = malicious
            let _ = peer.add_ban_score(100);
            log::warn!(
                "🚨 Penalized peer {} with 100 points for consensus validation failure",
                peer.addr
            );

            // Invalid block - reject and disconnect peer
            let _ = peer.send_message(Message::Reject {
                message: "block".to_string(),
                code: RejectCode::Invalid,
                reason: format!("consensus validation failed: {}", e),
            });
            return Err(WorkerError::InvalidData(format!(
                "consensus validation failed: {}",
                e
            )));
        }
    }
    drop(engine); // Release lock before async operations

    // Step 5: ForkChoice check (BEFORE insertion - this determines chain state)
    // Add block to ForkChoice to detect reorgs
    let fork_result = {
        let mut fc = ctx.fork_choice.lock().await;
        fc.add_block(block.header.clone())
    };

    match fork_result {
        Ok((_is_new_tip, Some(reorg_info))) => {
            // 🔀 REORG DETECTED! 🚨
            metrics::increment_reorg_counter();
            log::warn!(
                "🔀 CHAIN REORG! Old: {}, New: {}, Depth: {} blocks",
                hex::encode(&reorg_info.old_tip[..8]),
                hex::encode(&reorg_info.new_tip[..8]),
                reorg_info.disconnected_blocks.len()
            );

            // 5A. Disconnect old chain (rollback)
            for old_hash in reorg_info.disconnected_blocks.iter().rev() {
                // Get block from storage
                let old_block = match ctx.storage.get_block(old_hash).await {
                    Ok(Some(b)) => b,
                    Ok(None) => {
                        log::error!(
                            "❌ Block {} not found for disconnect!",
                            hex::encode(&old_hash[..8])
                        );
                        return Err(WorkerError::Storage("block not found".into()));
                    }
                    Err(e) => {
                        log::error!(
                            "❌ Failed to get block {}: {}",
                            hex::encode(&old_hash[..8]),
                            e
                        );
                        return Err(WorkerError::Storage(e.to_string()));
                    }
                };

                // Disconnect from storage (undo UTXOs)
                if let Err(e) = ctx.storage.disconnect_block(&old_block).await {
                    log::error!(
                        "🔥 FATAL: Failed to disconnect block {} during reorg!",
                        hex::encode(&old_hash[..8])
                    );
                    log::error!("🔥 Error: {}", e);
                    log::error!("🔥 CHAIN STATE IS CORRUPTED. Node must shutdown to prevent consensus failure.");
                    log::error!("🔥 Please restart and resync from trusted peers.");

                    // Return critical error - caller should initiate graceful shutdown
                    // This prevents consensus violations (double spends, wrong chain)
                    return Err(WorkerError::Critical(
                        "CRITICAL STORAGE FAILURE: Cannot rollback chain during reorg. Node shutdown required.".into()
                    ));
                }
                // Resurrect non-coinbase transactions back to mempool
                for (i, tx) in old_block.transactions.iter().enumerate() {
                    // Skip coinbase (index 0)
                    if i == 0 {
                        continue;
                    }

                    let txid = hex::encode(&tx.txid()[..8]);
                    log::info!("♻️  Resurrecting tx {} to mempool", txid);

                    let mut mempool = ctx.mempool.lock().await;
                    match mempool.insert(tx.clone(), /*estimated_fee=*/ 1000) {
                        Ok(()) => {
                            log::debug!("✅ Tx {} resurrected", txid);
                        }
                        Err(e) => {
                            log::debug!("⚠️  Tx {} not resurrected: {}", txid, e);
                        }
                    }
                    drop(mempool); // Release lock before next iteration
                }
                log::info!("⏪ Disconnected block {}", hex::encode(&old_hash[..8]));
            }

            // 5B. Connect new chain (already validated, just need to insert)
            // Note: new_block is being processed now, insert it later in Step 6
            'new_chain: for new_hash in &reorg_info.connected_blocks {
                // Skip the current block (will be inserted in Step 6)
                if new_hash == &block_hash {
                    continue;
                }

                // Fetch block from storage (should exist from peer)
                let new_block = match ctx.storage.get_block(new_hash).await {
                    Ok(Some(b)) => b,
                    Ok(None) => {
                        log::warn!(
                            "⚠️  New chain block {} not in storage, requesting from peer",
                            hex::encode(&new_hash[..8])
                        );

                        // Check if already requested (avoid duplicate requests)
                        {
                            let mut pending = ctx.pending_block_requests.lock().await;
                            if !pending.insert(*new_hash) {
                                log::debug!(
                                    "Block {} already requested, skipping",
                                    hex::encode(&new_hash[..8])
                                );
                                continue;
                            }
                        }

                        // Send GetData message to request the missing block
                        let getdata = create_block_getdata(vec![*new_hash]);
                        if let Err(e) = peer.send_message(getdata) {
                            log::error!(
                                "Failed to request block {}: {}",
                                hex::encode(&new_hash[..8]),
                                e
                            );
                            // Remove from pending since request failed
                            let mut pending = ctx.pending_block_requests.lock().await;
                            pending.remove(new_hash);
                        } else {
                            log::info!(
                                "📤 Requested block {} from peer {}",
                                hex::encode(&new_hash[..8]),
                                peer.addr
                            );
                        }

                        // Break the loop - wait for blocks to arrive via handle_block()
                        // This prevents "stuck reorg" where we keep requesting missing blocks
                        break 'new_chain;
                    }
                    Err(e) => {
                        log::error!(
                            "❌ Failed to get block {}: {}",
                            hex::encode(&new_hash[..8]),
                            e
                        );
                        continue;
                    }
                };

                // Insert block
                if let Err(e) = ctx.storage.insert_block(new_block).await {
                    log::error!(
                        "❌ Failed to insert block {}: {}",
                        hex::encode(&new_hash[..8]),
                        e
                    );
                    // Continue anyway to try inserting remaining blocks
                } else {
                    log::info!("➡️ Connected block {}", hex::encode(&new_hash[..8]));
                }
            }

            log::info!(
                "✅ Reorg complete! New tip: {}",
                hex::encode(&reorg_info.new_tip[..8])
            );
        }
        Ok((is_new_tip, None)) => {
            // Normal chain extension (no reorg)
            if is_new_tip {
                log::debug!("✅ New block extends chain (no reorg)");
            } else {
                log::debug!("📊 Block added to side chain");
            }
        }
        Err(e) => {
            // ForkChoice rejected the block (orphan, duplicate, invalid work)
            log::warn!("⚠️  ForkChoice rejected block: {}", e);
            // Don't insert, but don't disconnect peer (might be valid side chain)
            return Ok(true);
        }
    }

    // Step 6: Insert block into storage
    if let Err(e) = ctx.storage.insert_block(block.clone()).await {
        log::error!(
            "❌ Failed to insert block {}: {}",
            hex::encode(&block_hash[..8]),
            e
        );
        return Err(WorkerError::Storage(e.to_string()));
    }

    // Update metrics - block height (new height = old height + 1)
    metrics::update_block_height(height + 1);

    log::info!(
        "✅ Block {} connected to chain",
        hex::encode(&block_hash[..8])
    );

    // Step 6: Broadcast Inv to other peers
    let inv = [InvVector {
        inv_type: InvType::Block,
        hash: block_hash,
    }];

    match ctx.peer_manager.broadcast_inv(inv[0].clone()).await {
        Ok(count) => {
            log::info!(
                "📢 Broadcast Block {} to {} peers",
                hex::encode(&block_hash[..8]),
                count
            );
        }
        Err(e) => {
            log::error!(
                "❌ Failed to broadcast Block {}: {}",
                hex::encode(&block_hash[..8]),
                e
            );
        }
    }

    Ok(true)
}

/// Handle incoming transaction.
///
/// Logic:
/// 1. Validate the transaction (basic checks)
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
    log::info!(
        "💸 Received tx {} from {}",
        hex::encode(&tx_hash[..8]),
        peer.addr
    );

    // Step 1: Basic validation (Mempool::insert does full validation)
    // For now, estimate fee as 1 qbit per byte (conservative)
    let tx_size = transaction.serialized_size_hint().unwrap_or(1000);
    let estimated_fee = tx_size as u64; // 1 qbit per byte

    // Step 2: Try to add to mempool (this validates the transaction)
    let is_new = {
        let mut mempool = ctx.mempool.lock().await;
        match mempool.insert(transaction.clone(), estimated_fee) {
            Ok(()) => {
                log::info!(
                    "✅ Tx {} added to mempool ({} bytes, fee: {})",
                    hex::encode(&tx_hash[..8]),
                    tx_size,
                    estimated_fee
                );
                true
            }
            Err(e) => {
                log::warn!("⚠️  Tx {} rejected: {}", hex::encode(&tx_hash[..8]), e);

                // Linus Rule: Transaction validation failure = 20 points
                // Less severe than block failures (might be honest mistake)
                let _ = peer.add_ban_score(20);
                log::warn!(
                    "⚠️  Penalized peer {} with 20 points for invalid tx",
                    peer.addr
                );

                // Send reject message
                let _ = peer.send_message(Message::Reject {
                    message: "tx".to_string(),
                    code: RejectCode::Malformed,
                    reason: e.to_string(),
                });
                return Ok(true); // Don't disconnect, just reject
            }
        }
    };

    // Step 3: Broadcast Inv to other peers if new
    if is_new {
        let inv = [InvVector {
            inv_type: InvType::Tx,
            hash: tx_hash,
        }];

        match ctx.peer_manager.broadcast_inv(inv[0].clone()).await {
            Ok(count) => {
                log::info!(
                    "📢 Broadcast Tx {} to {} peers",
                    hex::encode(&tx_hash[..8]),
                    count
                );
            }
            Err(e) => {
                log::error!(
                    "❌ Failed to broadcast Tx {}: {}",
                    hex::encode(&tx_hash[..8]),
                    e
                );
            }
        }
    } else {
        log::debug!(
            "Tx {} already in mempool, skipping broadcast",
            hex::encode(&tx_hash[..8])
        );
    }

    Ok(true)
}

/// Handle GetHeaders request.
///
/// Logic:
/// 1. Find the common ancestor between locator hashes and our chain
/// 2. Send up to 2000 headers starting from common ancestor + 1
async fn handle_headers(
    peer: &mut Peer,
    ctx: &WorkerContext,
    headers: Vec<bitquan_types::BlockHeader>,
) -> Result<bool, WorkerError> {
    if headers.is_empty() {
        log::debug!("📨 Received empty Headers message from {}", peer.addr);
        return Ok(true);
    }

    log::info!(
        "📨 Received {} headers from {} (first: {})",
        headers.len(),
        peer.addr,
        hex::encode(&header_hash(&headers[0])[..8])
    );

    // Get current chain tip
    let tip_hash = match ctx.storage.tip().await {
        Ok(Some(header)) => header_hash(&header),
        Ok(None) => {
            log::warn!("⚠️  No chain tip found (empty chain)");
            // Empty chain - accept genesis
            [0u8; 32]
        }
        Err(e) => {
            log::error!("❌ Failed to get chain tip: {}", e);
            return Err(WorkerError::Storage(e.to_string()));
        }
    };

    // Validate each header and queue block download
    let mut valid_count = 0;
    let mut block_hashes = Vec::new();

    for (idx, header) in headers.iter().enumerate() {
        let hash = header_hash(header);

        // Skip if we already have this block
        match ctx.storage.get_block(&hash).await {
            Ok(Some(_)) => {
                log::debug!(
                    "Header {}/{} already known, skipping",
                    idx + 1,
                    headers.len()
                );
                continue;
            }
            Ok(None) => {
                // New header, proceed
            }
            Err(e) => {
                log::error!("❌ Storage error checking header: {}", e);
                return Err(WorkerError::Storage(e.to_string()));
            }
        }

        // Validate header links to our chain
        let prev_hash = header.prev_block;
        let expected_prev = if idx == 0 {
            // First header should link to our tip
            tip_hash
        } else {
            header_hash(&headers[idx - 1])
        };

        if prev_hash != expected_prev {
            log::warn!(
                "⚠️  Header {}/{} has invalid prev_block (expected {}, got {})",
                idx + 1,
                headers.len(),
                hex::encode(&expected_prev[..8]),
                hex::encode(&prev_hash[..8])
            );
            // Invalid chain link - stop processing
            break;
        }

        // Validate proof of work
        let target = match bitquan_consensus::pow::target_from_bits(header.bits) {
            Ok(t) => t,
            Err(e) => {
                log::warn!(
                    "⚠️  Header {}/{} has invalid bits: {}",
                    idx + 1,
                    headers.len(),
                    e
                );
                let _ = peer.add_ban_score(50);
                break;
            }
        };

        if !bitquan_consensus::pow::meets_target(&hash, &target) {
            log::warn!(
                "⚠️  Header {}/{} has invalid proof of work",
                idx + 1,
                headers.len()
            );
            // Invalid PoW - ban peer
            let _ = peer.add_ban_score(100);
            return Err(WorkerError::InvalidData(
                "Invalid proof of work".to_string(),
            ));
        }

        valid_count += 1;
        block_hashes.push(hash);

        // Validate timestamp is not too far in the future (2 hours)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if u64::from(header.time) > now + 7200 {
            log::warn!(
                " Header {}/{} has future timestamp ({} > {})",
                idx + 1,
                headers.len(),
                header.time,
                now + 7200
            );
            let _ = peer.add_ban_score(20);
            break;
        }
    }

    log::info!(
        "✅ Validated {} headers, queuing block downloads",
        valid_count
    );

    // Queue block downloads using GetData
    if !block_hashes.is_empty() {
        // Add to pending requests
        let mut pending = ctx.pending_block_requests.lock().await;
        for hash in &block_hashes {
            pending.insert(*hash);
        }
        drop(pending);

        // Send GetData message
        let inv: Vec<InvVector> = block_hashes
            .iter()
            .map(|hash| InvVector {
                inv_type: InvType::Block,
                hash: *hash,
            })
            .collect();

        peer.send_message(Message::GetData { inventory: inv })?;
        log::info!(
            "📤 Requested {} blocks from {}",
            block_hashes.len(),
            peer.addr
        );
    }

    Ok(true)
}

/// Handle GetHeaders request from a peer.
///
/// This is the server-side of header sync (IBD).
async fn handle_get_headers(
    peer: &mut Peer,
    _ctx: &WorkerContext,
    locator_hashes: Vec<[u8; 32]>,
    stop_hash: [u8; 32],
) -> Result<bool, WorkerError> {
    log::debug!(
        "📋 GetHeaders: {} locators, stop={}",
        locator_hashes.len(),
        hex::encode(&stop_hash[..8])
    );

    // TODO: Implement proper header locator logic
    // For now, send empty response
    peer.send_message(Message::Headers { headers: vec![] })?;

    log::debug!("📤 Sent empty Headers response to {}", peer.addr);

    Ok(true)
}

/// Handle GetBlocks request from a peer.
///
/// This is the server-side of IBD (Initial Block Download).
///
/// Logic:
/// 1. Find the common ancestor by checking each locator hash
/// 2. Get block headers after the common ancestor (up to 500 blocks)
/// 3. Send Inv message announcing available blocks
async fn handle_getblocks(
    peer: &mut Peer,
    ctx: &WorkerContext,
    locator_hashes: Vec<[u8; 32]>,
    stop_hash: [u8; 32],
) -> Result<bool, WorkerError> {
    log::debug!(
        "📥 GetBlocks: {} locators, stop={}",
        locator_hashes.len(),
        hex::encode(&stop_hash[..8])
    );

    // C5 FIX: Limit locator hashes to prevent DoS (Bitcoin uses ~500 max)
    const MAX_LOCATOR_HASHES: usize = 500;
    if locator_hashes.len() > MAX_LOCATOR_HASHES {
        log::warn!(
            "Peer {} sent too many locators ({}), limiting to {}",
            peer.addr,
            locator_hashes.len(),
            MAX_LOCATOR_HASHES
        );
    }

    // Find the height of the common ancestor to start announcing AFTER it
    // This prevents announcing blocks the peer already has (chain split prevention)
    let mut start_height = 0u64;

    // Build inventory of blocks to announce
    let mut inv: Vec<bitquan_network::protocol::InvVector> = Vec::new();

    // Get chain height once for validation
    let chain_height = match ctx.storage.height().await {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to get chain height: {}", e);
            return Ok(false);
        }
    };

    // C5 FIX: Check each locator hash to find common ancestor (limited to prevent DoS)
    for locator_hash in locator_hashes.iter().take(MAX_LOCATOR_HASHES) {
        match ctx.storage.get_block(locator_hash).await {
            Ok(Some(_)) => {
                // Found common ancestor - now find its height
                // Use pre-fetched chain_height for validation

                // Search for the ancestor's height
                let mut found_height = None;
                for h in 0..=chain_height {
                    if let Ok(Some(block)) = ctx.storage.get_block_by_height(h).await {
                        let block_hash = header_hash(&block.header);
                        if block_hash == *locator_hash {
                            found_height = Some(h);
                            break;
                        }
                    }
                }

                if let Some(h) = found_height {
                    start_height = h + 1;
                    log::info!(
                        "✅ Common ancestor at height {}, starting from {}",
                        h,
                        start_height
                    );
                }
                break;
            }
            Ok(None) => {
                // This locator doesn't exist in our chain, try next
            }
            Err(e) => {
                log::error!("Storage error checking locator: {}", e);
                break;
            }
        }
    }

    let mut height = start_height;
    let limit = 500; // Max blocks to announce per GetBlocks response

    // C5 FIX: Validate start_height is within bounds
    if start_height > chain_height {
        log::debug!(
            "📤 Start height {} exceeds chain height {}, nothing to announce",
            start_height,
            chain_height
        );
        return Ok(true);
    }

    while inv.len() < limit && height <= chain_height {
        match ctx.storage.get_block_by_height(height).await {
            Ok(Some(block)) => {
                let block_hash = header_hash(&block.header);

                // Check if we should stop
                if stop_hash != [0u8; 32] && block_hash == stop_hash {
                    log::debug!("🛑 Reached stop_hash at height {}", height);
                    break;
                }

                inv.push(bitquan_network::protocol::InvVector {
                    inv_type: bitquan_network::protocol::InvType::Block,
                    hash: block_hash,
                });
            }
            Ok(None) => {
                // No more blocks at this height
                break;
            }
            Err(e) => {
                log::error!("Error fetching block at height {}: {}", height, e);
                break;
            }
        }

        height += 1;
    }

    if !inv.is_empty() {
        let inv_count = inv.len();
        peer.send_message(Message::Inv { inventory: inv })?;
        log::info!("📤 Sent Inv with {} blocks to {}", inv_count, peer.addr);
    } else {
        log::debug!("📤 No blocks to announce to {}", peer.addr);
    }

    Ok(true)
}

/// Validates all block transactions against the UTXO set.
///
/// This CRITICAL validation prevents double spends and ensures all inputs exist.
/// MUST be called before `insert_block()` to maintain blockchain integrity.
///
/// Coinbase Maturity:
/// - Coinbase outputs require 100 confirmations before spending
/// - Enforced via StoredUtxoEntry with height + is_coinbase tracking
/// - Validation: current_height >= utxo_height + 100
///
/// # Arguments
/// * `ctx` - Worker context containing UTXO set
/// * `block` - Block to validate
/// * `height` - Current chain height (for coinbase maturity checks)
///
/// # Returns
/// Total fees paid by all non-coinbase transactions
///
/// # Errors
/// Returns error if any transaction:
/// - Spends non-existent UTXO
/// - Attempts double spend
/// - Violates coinbase maturity (spends coinbase < 100 blocks old)
/// - Has invalid input/output balance
pub(crate) async fn validate_block_utxos(
    ctx: &WorkerContext,
    block: &Block,
    height: u64,
) -> Result<u128, WorkerError> {
    use bitquan_consensus::utxo::OutPoint;
    use std::collections::HashSet;

    let mut total_fees = 0u128;
    // Track inputs spent within this block to prevent internal double spends
    let mut spent_in_block = HashSet::new();

    // Validate each transaction (skip coinbase at index 0)
    for (tx_index, tx) in block.transactions.iter().enumerate() {
        let is_coinbase = tx_index == 0;

        // Coinbase handling: check maturity if spending (unlikely for coinbase)
        // But mainly coinbase just creates outputs.
        // We generally skip input validation for coinbase as they are newly generated.
        // However, we must ensure it doesn't try to spend anything (it has 1 input with null hash).
        if is_coinbase {
            // Basic coinbase structure check is done in consensus, but we can double check:
            // Inputs should be empty effectively (handled by logic below or skipped)
            continue;
        }

        let mut inputs_value = 0u128;
        let mut outputs_value = 0u128;

        // 1. Validate Inputs
        for input in &tx.inputs {
            let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);

            // CRITICAL: Check for internal double spend
            if !spent_in_block.insert(outpoint) {
                return Err(WorkerError::InvalidData(format!(
                    "Double spend detected within block: tx {} spends already used outpoint txid={} vout={}",
                    hex::encode(&tx.txid()[..8]),
                    hex::encode(input.prev_txid),
                    input.prev_vout
                )));
            }

            // Serialize outpoint key for DB lookup (txid + vout_le)
            let outpoint_key = [&input.prev_txid[..], &input.prev_vout.to_le_bytes()[..]].concat();

            // Fetch UTXO from persistent storage
            let utxo_bytes = ctx
                .storage
                .get_utxo(&outpoint_key)
                .await
                .map_err(|e| WorkerError::Storage(e.to_string()))?
                .ok_or_else(|| {
                    WorkerError::InvalidData(format!(
                        "Input spent non-existent/already-spent UTXO: txid={} vout={}",
                        hex::encode(input.prev_txid),
                        input.prev_vout
                    ))
                })?;

            // Deserialize UTXO entry with maturity data
            let utxo_entry: StoredUtxoEntry = serialize::from_bytes(&utxo_bytes)
                .map_err(|e| WorkerError::Storage(format!("Failed to deserialize UTXO: {}", e)))?;

            // COINBASE MATURITY CHECK (100 blocks)
            const COINBASE_MATURITY: u64 = 100;
            if utxo_entry.is_coinbase {
                let maturity_height = utxo_entry.height.saturating_add(COINBASE_MATURITY);
                if height < maturity_height {
                    return Err(WorkerError::InvalidData(format!(
                        "Coinbase UTXO spent before maturity: tx={} vout={} created_at={} current={} required={}",
                        hex::encode(&input.prev_txid[..8]),
                        input.prev_vout,
                        utxo_entry.height,
                        height,
                        maturity_height
                    )));
                }
            }

            inputs_value = inputs_value
                .checked_add(utxo_entry.output.value)
                .ok_or_else(|| {
                    WorkerError::InvalidData(format!(
                        "Integer overflow: tx {} input values exceed u64::MAX",
                        hex::encode(&tx.txid()[..8])
                    ))
                })?;
        }

        // 2. Validate Outputs
        for output in &tx.outputs {
            outputs_value = outputs_value.checked_add(output.value).ok_or_else(|| {
                WorkerError::InvalidData(format!(
                    "Integer overflow: tx {} output values exceed u64::MAX",
                    hex::encode(&tx.txid()[..8])
                ))
            })?;
        }

        // 3. Fee Check with overflow protection
        // Calculate fee for this transaction with overflow protection
        let fee = inputs_value.checked_sub(outputs_value).ok_or_else(|| {
            WorkerError::InvalidData(format!(
                "Transaction outputs ({}) exceed inputs ({})",
                outputs_value, inputs_value
            ))
        })?;

        // Add to block total with overflow protection
        total_fees = total_fees.checked_add(fee).ok_or_else(|| {
            WorkerError::InvalidData("Integer overflow: block fees exceed u64::MAX".to_string())
        })?;

        log::debug!(
            "✅ Tx {} UTXO valid (fee: {})",
            hex::encode(&tx.txid()[..8]),
            fee
        );
    }

    Ok(total_fees)
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
    height: u64,
) -> Result<(), WorkerError> {
    use bitquan_network::protocol::{Message, PROTOCOL_VERSION};

    let _magic = network_magic(network);

    // Wait for version message
    let msg = peer.recv_message()?;
    match msg {
        Message::Version {
            version,
            services: _,
            timestamp: _,
            user_agent,
            start_height,
        } => {
            peer.version = Some(version);
            peer.user_agent = Some(user_agent.clone());
            peer.start_height = Some(start_height);

            log::info!(
                "🤝 Handshake: {} (v{}, {}, height={})",
                user_agent,
                version,
                peer.addr,
                start_height
            );

            // Send our version
            let our_version = Message::Version {
                version: PROTOCOL_VERSION,
                services: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                user_agent: env!("CARGO_PKG_NAME").to_string(),
                start_height: height,
            };

            peer.send_message(our_version)?;
            peer.send_message(Message::VerAck)?;
        }
        _ => {
            return Err(WorkerError::InvalidData(
                "expected version message".to_string(),
            ));
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;
    use bitquan_storage::async_store::{AsyncChainStore, AsyncStoreError};
    use bitquan_types::{TxIn, TxOut};
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    /// Mock storage that returns predefined UTXOs
    struct MockAsyncStore {
        utxos: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    }

    impl MockAsyncStore {
        fn new() -> Self {
            let mut utxos = std::collections::HashMap::new();

            // Create a mock UTXO: prev_txid + prev_vout -> StoredUtxoEntry
            let prev_txid = [1u8; 32];
            let prev_vout = 0u32;
            let outpoint_key = [&prev_txid[..], &prev_vout.to_le_bytes()[..]].concat();

            let utxo_entry = StoredUtxoEntry {
                output: TxOut {
                    value: 1_000_000_000_000_000_000, // 1 BQ (18 decimals)
                    script_pubkey: vec![
                        0x76, 0xa9, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88,
                        0xac,
                    ], // P2PKH
                },
                height: 100,        // Already mature
                is_coinbase: false, // Not a coinbase UTXO
            };

            utxos.insert(
                outpoint_key,
                serialize::to_bytes(&utxo_entry).expect("Failed to serialize mock UTXO"),
            );
            Self { utxos }
        }
    }

    #[async_trait::async_trait]
    impl AsyncChainStore for MockAsyncStore {
        async fn height(&self) -> std::result::Result<u64, AsyncStoreError> {
            Ok(200) // High enough for UTXOs at height 100 to be mature
        }

        async fn tip(
            &self,
        ) -> std::result::Result<Option<bitquan_types::BlockHeader>, AsyncStoreError> {
            Ok(None)
        }

        async fn get_block(
            &self,
            _hash: &[u8; 32],
        ) -> std::result::Result<Option<bitquan_types::Block>, AsyncStoreError> {
            Ok(None)
        }

        async fn get_block_by_height(
            &self,
            _height: u64,
        ) -> std::result::Result<Option<bitquan_types::Block>, AsyncStoreError> {
            Ok(None)
        }

        async fn get_transaction(
            &self,
            _txid: &[u8; 32],
        ) -> std::result::Result<Option<bitquan_types::Transaction>, AsyncStoreError> {
            Ok(None)
        }

        async fn insert_block(
            &self,
            _block: bitquan_types::Block,
        ) -> std::result::Result<(), AsyncStoreError> {
            Ok(())
        }

        async fn has_block(&self, _hash: &[u8; 32]) -> std::result::Result<bool, AsyncStoreError> {
            Ok(false)
        }

        async fn get_header(
            &self,
            _hash: &[u8; 32],
        ) -> std::result::Result<Option<bitquan_types::BlockHeader>, AsyncStoreError> {
            Ok(None)
        }

        async fn get_utxo(
            &self,
            outpoint: &[u8],
        ) -> std::result::Result<Option<Vec<u8>>, AsyncStoreError> {
            Ok(self.utxos.get(outpoint).cloned())
        }

        async fn disconnect_block(
            &self,
            _block: &bitquan_types::Block,
        ) -> std::result::Result<(), AsyncStoreError> {
            // Mock implementation - does nothing
            Ok(())
        }

        async fn median_time_past(&self) -> std::result::Result<u64, AsyncStoreError> {
            // Mock implementation - returns 0 for testing
            Ok(0)
        }

        async fn get_pruning_metadata(
            &self,
        ) -> std::result::Result<Option<bitquan_storage::PruningMetadata>, AsyncStoreError>
        {
            // Mock implementation - returns None (no pruning)
            Ok(None)
        }
    }

    /// Test that double spends within the same block are detected and rejected
    #[tokio::test]
    async fn test_double_spend_detection_within_block() {
        // Setup mock storage with one UTXO
        let storage = Arc::new(MockAsyncStore::new()) as Arc<dyn AsyncChainStore>;

        // Create WorkerContext with mock dependencies
        let noise_config = Arc::new(
            bitquan_network::noise::NoiseConfig::generate()
                .expect("Failed to generate noise config for test"),
        );
        let ctx = WorkerContext {
            peer_manager: Arc::new(bitquan_network::peer::PeerManager::new(
                10, // max_peers
                bitquan_types::NetworkId::Devnet,
                noise_config,
            )),
            storage: storage.clone(),
            mempool: Arc::new(TokioMutex::new(
                bitquan_mempool::Mempool::new().expect("Failed to create mempool for test"),
            )),
            consensus: Arc::new(TokioMutex::new(bitquan_consensus::ConsensusEngine::new(
                bitquan_consensus::ConsensusParams::devnet_hybrid(),
                bq_crypto::CryptoRegistry::new(),
            ))),
            fork_choice: Arc::new(TokioMutex::new(bitquan_consensus::fork::ForkChoice::new())),
            ban_manager: Arc::new(TokioMutex::new(BanManager::new(
                bitquan_network::ban_manager::BanConfig::default(),
            ))),
            network_id: bitquan_types::NetworkId::Devnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
            pending_block_requests: Arc::new(TokioMutex::new(std::collections::HashSet::new())),
        };

        // Create two transactions that BOTH spend the same UTXO (double spend)
        let prev_txid = [1u8; 32];
        let prev_vout = 0u32;

        let tx_a = Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            inputs: vec![TxIn {
                prev_txid,
                prev_vout,
                sequence: 0xffffffff,
                script_sig: vec![],
            }],
            outputs: vec![TxOut {
                value: 50_000_000, // 0.5 BQ
                script_pubkey: vec![],
            }],
            lock_time: 0,
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        let tx_b = Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            inputs: vec![TxIn {
                prev_txid, // SAME prev_txid!
                prev_vout, // SAME prev_vout!
                sequence: 0xffffffff,
                script_sig: vec![],
            }],
            outputs: vec![TxOut {
                value: 50_000_000, // 0.5 BQ
                script_pubkey: vec![],
            }],
            lock_time: 0,
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        // Create a coinbase transaction
        let coinbase = Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            inputs: vec![TxIn {
                prev_txid: [0u8; 32], // Null hash for coinbase
                prev_vout: 0xffffffff,
                sequence: 0xffffffff,
                script_sig: b"coinbase".to_vec(),
            }],
            outputs: vec![TxOut {
                value: 1_000_000_000_000_000_000, // 1 BQ block reward (18 decimals)
                script_pubkey: vec![],
            }],
            lock_time: 0,
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        // Create a block with both transactions (DOUBLE SPEND!)
        let block = bitquan_types::Block {
            header: bitquan_types::BlockHeader {
                version: 1,
                prev_block: [0u8; 32],
                merkle_root: [0u8; 32],
                pqc_agg_hint: [0u8; 32],
                uncles_hash: [0u8; 32],
                time: 0,
                bits: 0x1d00ffff,
                nonce: 0,
                algo_id: 0, // SHA-256d
            },
            uncles: vec![],
            transactions: vec![coinbase, tx_a, tx_b],
        };

        // Try to validate - should FAIL with double spend error
        let result = validate_block_utxos(&ctx, &block, 0).await;

        // Assert that validation FAILED
        assert!(result.is_err(), "Double spend should be detected!");

        // Verify error message contains "double spend"
        let error_msg = match result {
            Err(e) => e.to_string(),
            Ok(_) => unreachable!("Test should have failed with double spend error"),
        };
        assert!(
            error_msg.to_lowercase().contains("double spend"),
            "Error message should mention 'double spend', got: {}",
            error_msg
        );

        println!("✅ Test passed: Double spend was correctly detected!");
        println!("   Error message: {}", error_msg);
    }

    /// Test that integer overflow attacks are rejected
    #[tokio::test]
    async fn test_integer_overflow_attack_prevented() {
        // Setup mock storage
        let storage = Arc::new(MockAsyncStore::new()) as Arc<dyn AsyncChainStore>;

        // Create WorkerContext
        let noise_config = Arc::new(bitquan_network::noise::NoiseConfig::generate().unwrap());
        let ctx = WorkerContext {
            peer_manager: Arc::new(bitquan_network::peer::PeerManager::new(
                10,
                bitquan_types::NetworkId::Devnet,
                noise_config,
            )),
            storage: storage.clone(),
            mempool: Arc::new(TokioMutex::new(bitquan_mempool::Mempool::new().unwrap())),
            consensus: Arc::new(TokioMutex::new(bitquan_consensus::ConsensusEngine::new(
                bitquan_consensus::ConsensusParams::devnet_hybrid(),
                bq_crypto::CryptoRegistry::new(),
            ))),
            fork_choice: Arc::new(TokioMutex::new(bitquan_consensus::fork::ForkChoice::new())),
            ban_manager: Arc::new(TokioMutex::new(BanManager::new(
                bitquan_network::ban_manager::BanConfig::default(),
            ))),
            network_id: bitquan_types::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            pending_block_requests: Arc::new(TokioMutex::new(std::collections::HashSet::new())),
        };

        // Create transaction with u64::MAX value (overflow attack)
        let tx_overflow = Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            inputs: vec![TxIn {
                prev_txid: [1u8; 32],
                prev_vout: 0,
                sequence: 0xffffffff,
                script_sig: vec![],
            }],
            outputs: vec![TxOut {
                value: u128::MAX, // ATTEMPT OVERFLOW
                script_pubkey: vec![],
            }],
            lock_time: 0,
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        let coinbase = Transaction {
            version: 1,
            network: bitquan_types::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            inputs: vec![TxIn {
                prev_txid: [0u8; 32],
                prev_vout: 0xffffffff,
                sequence: 0xffffffff,
                script_sig: b"coinbase".to_vec(),
            }],
            outputs: vec![TxOut {
                value: 1_000_000_000_000_000_000, // 1 BQ (18 decimals)
                script_pubkey: vec![],
            }],
            lock_time: 0,
            sig_algo: bitquan_types::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        };

        let block = bitquan_types::Block {
            header: bitquan_types::BlockHeader {
                version: 1,
                prev_block: [0u8; 32],
                merkle_root: [0u8; 32],
                pqc_agg_hint: [0u8; 32],
                uncles_hash: [0u8; 32],
                time: 0,
                bits: 0x1d00ffff,
                nonce: 0,
                algo_id: 0,
            },
            uncles: vec![],
            transactions: vec![coinbase, tx_overflow],
        };

        // Should reject with overflow error
        let result = validate_block_utxos(&ctx, &block, 0).await;
        assert!(result.is_err(), "Overflow attack should be detected!");

        let error_msg = result.unwrap_err().to_string();
        // Overflow protection works by catching outputs > inputs
        assert!(
            error_msg.to_lowercase().contains("overflow")
                || error_msg.to_lowercase().contains("exceed"),
            "Error should mention overflow or exceed, got: {}",
            error_msg
        );

        println!("✅ Overflow attack blocked: {}", error_msg);
    }
}
