//! RPC handler implementation for the BitQuan node.

#![allow(dead_code)]

use std::sync::Arc;

use async_trait::async_trait;

use bitquan_consensus::header_hash;
use bitquan_mempool::Mempool;
use bitquan_network::async_sync::AsyncSyncManager;
use bitquan_rpc::{
    methods::{
        BlockTemplate, BlockchainInfo, MinerStatsResponse, MiningInfo, NetworkStatusResponse,
        PayoutRequest, PayoutResponse, PoolStatsResponse, RpcMethods, SyncResponse, TxInfo,
        WorkData,
    },
    RpcError,
};
use bitquan_storage::{async_store::AsyncChainStore, StorageError};
use bitquan_types::{Transaction, GENESIS_BITS};
use hex::FromHex;
use tokio::sync::Mutex;

// Import address utilities for generatetoaddress
use crate::address::{decode_bech32m, script_from_pubkey_hash};
use crate::tx_builder::TransactionBuilder;
use crate::wallet::WalletKeypair;

/// Node RPC handler backed by an async chain store.
pub struct NodeRpcHandler {
    store: Arc<dyn AsyncChainStore>,
    chain_name: String,
    sync_manager: Option<Arc<AsyncSyncManager>>,
    mempool: Option<Arc<Mutex<Mempool>>>,
}

impl NodeRpcHandler {
    /// Create a new RPC handler using the given async store.
    pub fn new(store: Arc<dyn AsyncChainStore>, chain_name: impl Into<String>) -> Self {
        Self {
            store,
            chain_name: chain_name.into(),
            sync_manager: None,
            mempool: None,
        }
    }

    /// Create a new RPC handler with sync manager and mempool.
    pub fn with_components(
        store: Arc<dyn AsyncChainStore>,
        chain_name: impl Into<String>,
        sync_manager: Arc<AsyncSyncManager>,
        mempool: Option<Arc<Mutex<Mempool>>>,
    ) -> Self {
        Self {
            store,
            chain_name: chain_name.into(),
            sync_manager: Some(sync_manager),
            mempool,
        }
    }

    /// Helper to convert storage errors to RPC errors safely
    fn storage_error_to_rpc(e: bitquan_storage::async_store::AsyncStoreError) -> RpcError {
        match e {
            bitquan_storage::async_store::AsyncStoreError::Storage(se) => {
                RpcError::InternalError(format!("storage error: {}", se))
            }
            bitquan_storage::async_store::AsyncStoreError::TaskSpawn(te) => {
                RpcError::InternalError(format!("task spawn error: {}", te))
            }
            bitquan_storage::async_store::AsyncStoreError::Poisoned(s) => {
                RpcError::InternalError(format!("mutex poisoned during {}", s))
            }
            bitquan_storage::async_store::AsyncStoreError::Cancelled => {
                RpcError::InternalError("operation cancelled".to_string())
            }
        }
    }
}

#[async_trait]
impl RpcMethods for NodeRpcHandler {
    async fn getblockcount(&self) -> Result<u64, RpcError> {
        let height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        Ok(height)
    }

    async fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError> {
        let height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;

        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;

        let tip_hash = tip.as_ref().map(|header| hex::encode(header_hash(header)));

        let difficulty = tip
            .as_ref()
            .map(|header| difficulty_from_bits(header.bits))
            .unwrap_or(1.0);

        // Get pruning info if available
        let (pruning_mode, pruning_height, blocks_kept) =
            if let Some(metadata) = self.store.get_pruning_metadata().await.ok().flatten() {
                let mode_str = match metadata.mode {
                    bitquan_storage::PruningMode::Full => Some("full".to_string()),
                    bitquan_storage::PruningMode::Pruned { .. } => Some("pruned".to_string()),
                    bitquan_storage::PruningMode::UtxoOnly => Some("utxo_only".to_string()),
                };
                let blocks = match metadata.mode {
                    bitquan_storage::PruningMode::Full => None,
                    bitquan_storage::PruningMode::Pruned { keep_blocks } => Some(keep_blocks),
                    bitquan_storage::PruningMode::UtxoOnly => Some(0),
                };
                (mode_str, metadata.pruning_height, blocks)
            } else {
                (None, None, None)
            };

        Ok(BlockchainInfo {
            chain: self.chain_name.clone(),
            blocks: height,
            bestblockhash: tip_hash.unwrap_or_else(|| String::from("0")),
            difficulty,
            chainwork: String::from("0"),
            pruning_mode,
            pruning_height,
            blocks_kept,
        })
    }

    async fn getmininginfo(&self) -> Result<MiningInfo, RpcError> {
        let blocks = self.getblockcount().await?;
        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;
        let difficulty = tip
            .map(|header| difficulty_from_bits(header.bits))
            .unwrap_or(1.0);

        Ok(MiningInfo {
            blocks,
            difficulty,
            networkhashps: 0.0,
        })
    }

    async fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError> {
        Err(RpcError::InternalError(
            "getblocktemplate not implemented".into(),
        ))
    }

    async fn getwork(&self) -> Result<WorkData, RpcError> {
        Err(RpcError::InternalError("getwork not implemented".into()))
    }

    async fn submitblock(&self, _block_hex: String) -> Result<bool, RpcError> {
        Err(RpcError::InternalError(
            "submitblock not implemented".into(),
        ))
    }

    async fn submitwork(&self, _data: String) -> Result<bool, RpcError> {
        Err(RpcError::InternalError("submitwork not implemented".into()))
    }

    async fn gettransaction(&self, txid: String) -> Result<TxInfo, RpcError> {
        let bytes = Vec::from_hex(&txid)
            .map_err(|_| RpcError::InvalidParams("txid must be hex-encoded".into()))?;
        if bytes.len() != 32 {
            return Err(RpcError::InvalidParams(
                "txid must be 32 bytes (64 hex chars)".into(),
            ));
        }
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes);

        let tx = self
            .store
            .get_transaction(&id)
            .await
            .map_err(Self::storage_error_to_rpc)?
            .ok_or_else(|| RpcError::InternalError("transaction not found".into()))?;

        let summary = TransactionSummary::from(tx);
        Ok(summary.into())
    }

    async fn submittransaction(&self, tx_hex: String) -> Result<String, RpcError> {
        // Decode transaction from hex
        let tx_bytes = Vec::from_hex(&tx_hex)
            .map_err(|_| RpcError::InvalidParams("transaction must be hex-encoded".into()))?;

        // Parse transaction (for now, assume it's JSON format)
        let tx: Transaction = serde_json::from_slice(&tx_bytes)
            .map_err(|e| RpcError::InvalidParams(format!("failed to parse transaction: {}", e)))?;

        // Calculate transaction ID
        let txid = hex::encode(tx.txid());

        // For now, just validate and return the txid
        // In a full implementation, this would:
        // 1. Validate transaction syntax
        // 2. Verify signatures
        // 3. Check inputs/outputs
        // 4. Add to mempool
        // 5. Broadcast to peers

        Ok(txid)
    }

    async fn getbestblockhash(&self) -> Result<String, RpcError> {
        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;

        Ok(tip
            .map(|header| hex::encode(header_hash(&header)))
            .unwrap_or_else(|| String::from("0")))
    }

    async fn getblockhash(&self, height: u64) -> Result<String, RpcError> {
        let block = self
            .store
            .get_block_by_height(height)
            .await
            .map_err(Self::storage_error_to_rpc)?
            .ok_or_else(|| RpcError::InternalError("block not found".into()))?;

        Ok(hex::encode(header_hash(&block.header)))
    }

    async fn getpoolstats(&self) -> Result<PoolStatsResponse, RpcError> {
        // Pool stats require reward engine integration
        // For now, return placeholder values from chain state
        let height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        Ok(PoolStatsResponse {
            height,
            total_rewards: 0,
            miner_count: 0,
            pool_balance: 0,
            block_count: height,
        })
    }

    async fn getminerstats(&self, miner_id: String) -> Result<MinerStatsResponse, RpcError> {
        // Miner stats require reward engine integration
        // For now, return placeholder
        Ok(MinerStatsResponse {
            miner_id,
            total_reward: 0,
            blocks_mined: 0,
            recent_blocks: vec![],
        })
    }

    async fn createpayout(&self, _request: PayoutRequest) -> Result<PayoutResponse, RpcError> {
        // Payout creation requires reward engine integration
        // For now, return mock success
        Ok(PayoutResponse {
            payout_id: format!("payout_{}", uuid::Uuid::new_v4()),
            txid: None,
        })
    }

    async fn getnetworkstatus(&self) -> Result<NetworkStatusResponse, RpcError> {
        // Network status requires network manager integration
        // For now, return placeholder values
        let local_height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        Ok(NetworkStatusResponse {
            peers_connected: 0,
            blocks_broadcast: 0,
            blocks_received: 0,
            sync_status: "idle".to_string(),
            local_height,
            best_height: local_height,
        })
    }

    async fn sync(&self) -> Result<SyncResponse, RpcError> {
        // Get current chain height safely
        let local_height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;

        // Check if sync manager is available
        if let Some(sync_manager) = &self.sync_manager {
            // Get real sync status from sync manager
            let progress = sync_manager
                .get_sync_progress()
                .await
                .map_err(|e| RpcError::InternalError(format!("sync manager error: {}", e)))?;

            // Determine if syncing is active based on status
            let is_syncing = matches!(
                progress.status,
                bitquan_network::sync::SyncStatus::Discovering
                    | bitquan_network::sync::SyncStatus::DownloadingHeaders
                    | bitquan_network::sync::SyncStatus::DownloadingBlocks
            );

            Ok(SyncResponse {
                status: format!("{:?}", progress.status),
                local_height: progress.local_height,
                best_height: progress.best_height,
                blocks_behind: progress.blocks_behind,
                progress: progress.progress,
                syncing: is_syncing,
                last_sync_attempt: progress.last_sync_attempt,
                sync_errors: progress.sync_errors,
                headers_synced: progress.headers_synced,
                blocks_synced: progress.blocks_synced,
                peers_connected: progress.peers_connected,
            })
        } else {
            // Fallback when sync manager is not initialized
            Ok(SyncResponse {
                status: "sync_manager_unavailable".to_string(),
                local_height,
                best_height: local_height,
                blocks_behind: 0,
                progress: 100.0,
                syncing: false,
                last_sync_attempt: 0,
                sync_errors: 0,
                headers_synced: 0,
                blocks_synced: 0,
                peers_connected: 0,
            })
        }
    }

    async fn generate(
        &self,
        n_blocks: u64,
        _address: Option<String>,
    ) -> Result<Vec<String>, RpcError> {
        use bitquan_consensus::pow::{PowEngine, Sha256dEngine};
        use bitquan_types::{Block, BlockHeader, SigAlgorithm, Transaction, TxOut};

        let mut generated_hashes = Vec::new();

        // Get current chain state
        let _height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;

        // For regtest/devnet, use very easy difficulty
        let bits = if self.chain_name == "regtest" || self.chain_name == "devnet" {
            0x207fffff // Ultra-easy target (16 million x easier than mainnet)
        } else {
            tip.as_ref()
                .map(|h| h.bits)
                .unwrap_or(bitquan_types::GENESIS_BITS)
        };

        // Get previous block hash
        let prev_block = match tip {
            Some(header) => header,
            None => {
                // Use genesis block header if no tip
                bitquan_types::BlockHeader {
                    version: bitquan_types::GENESIS_VERSION,
                    prev_block: [0u8; 32],
                    merkle_root: bitquan_types::GENESIS_HASH_BYTES,
                    pqc_agg_hint: [0u8; 32],
                    time: bitquan_types::GENESIS_TIME,
                    bits: bitquan_types::GENESIS_BITS,
                    nonce: bitquan_types::GENESIS_NONCE,
                    algo_id: 0,
                }
            }
        };

        let mut prev_hash = bitquan_consensus::header_hash(&prev_block);

        // Mine n_blocks
        for i in 0..n_blocks {
            // Create coinbase transaction
            let coinbase_tx = Transaction {
                version: 1,
                network: bitquan_types::NetworkId::Regtest,
                genesis_hash: bitquan_types::GENESIS_HASH_BYTES,
                lock_time: 0,
                inputs: vec![],
                outputs: vec![TxOut {
                    value: bitquan_types::GENESIS_REWARD, // 50 BQ in qbits
                    script_pubkey: vec![0x51],            // Simple OP_1 for now
                }],
                sig_algo: SigAlgorithm::Dilithium5,
                witnesses: vec![],
            };

            // Calculate merkle root (just coinbase txid for now)
            let txid = coinbase_tx.txid();
            let mut merkle_root = [0u8; 32];
            merkle_root.copy_from_slice(&txid);

            // Create block header template
            let mut header = BlockHeader {
                version: bitquan_types::GENESIS_VERSION,
                prev_block: prev_hash,
                merkle_root,
                pqc_agg_hint: [0u8; 32],
                time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as u32,
                bits,
                nonce: 0,
                algo_id: 0, // SHA256d
            };

            // Mine the block using simple SHA256d PoW
            let engine = Sha256dEngine;
            // Use higher limit for regtest/devnet (easy difficulty still needs many attempts)
            let max_nonce = if self.chain_name == "regtest" || self.chain_name == "devnet" {
                100_000_000 // 100M attempts for testing
            } else {
                1_000_000 // 1M for production
            };

            let mut found = false;
            for nonce in 0..max_nonce {
                header.nonce = nonce;

                // Verify if this nonce meets the target
                if engine.verify(&header).is_ok() {
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(RpcError::InternalError(format!(
                    "Failed to mine block {} after {} attempts",
                    i, max_nonce
                )));
            }

            // Create full block
            let block = Block {
                header: header.clone(),
                transactions: vec![coinbase_tx],
            };

            // Insert block into storage
            self.store
                .insert_block(block)
                .await
                .map_err(Self::storage_error_to_rpc)?;

            let block_hash = bitquan_consensus::header_hash(&header);
            generated_hashes.push(hex::encode(block_hash));

            // Update prev_hash for next block
            prev_hash = block_hash;
        }

        Ok(generated_hashes)
    }

    async fn generatetoaddress(
        &self,
        n_blocks: u64,
        address: String,
    ) -> Result<Vec<String>, RpcError> {
        use bitquan_consensus::pow::{PowEngine, Sha256dEngine};
        use bitquan_types::{Block, BlockHeader, SigAlgorithm, TxOut};

        // Parse address and extract pubkey hash
        let pubkey_hash = decode_bech32m(&address)
            .map_err(|e| RpcError::InvalidParams(format!("Invalid address: {}", e)))?;

        // Create script_pubkey from address
        let script_pubkey = script_from_pubkey_hash(&pubkey_hash);

        let mut generated_hashes = Vec::new();

        // Get current chain state
        let _height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        let tip = self.store.tip().await.map_err(Self::storage_error_to_rpc)?;

        // For regtest/devnet, use very easy difficulty
        let bits = if self.chain_name == "regtest" || self.chain_name == "devnet" {
            0x207fffff // Ultra-easy target (16 million x easier than mainnet)
        } else {
            tip.as_ref()
                .map(|h| h.bits)
                .unwrap_or(bitquan_types::GENESIS_BITS)
        };

        // Get previous block hash
        let prev_block = match tip {
            Some(header) => header,
            None => {
                // Use genesis block header if no tip
                bitquan_types::BlockHeader {
                    version: bitquan_types::GENESIS_VERSION,
                    prev_block: [0u8; 32],
                    merkle_root: bitquan_types::GENESIS_HASH_BYTES,
                    pqc_agg_hint: [0u8; 32],
                    time: bitquan_types::GENESIS_TIME,
                    bits: bitquan_types::GENESIS_BITS,
                    nonce: bitquan_types::GENESIS_NONCE,
                    algo_id: 0,
                }
            }
        };

        let mut prev_hash = bitquan_consensus::header_hash(&prev_block);

        // Mine n_blocks
        for i in 0..n_blocks {
            // Create coinbase transaction with specified address
            let coinbase_tx = Transaction {
                version: 1,
                network: bitquan_types::NetworkId::Regtest,
                genesis_hash: bitquan_types::GENESIS_HASH_BYTES,
                lock_time: 0,
                inputs: vec![],
                outputs: vec![TxOut {
                    value: bitquan_types::GENESIS_REWARD, // 50 BQ in qbits
                    script_pubkey: script_pubkey.clone(), // Use address script
                }],
                sig_algo: SigAlgorithm::Dilithium5,
                witnesses: vec![],
            };

            // Calculate merkle root (just coinbase txid for now)
            let txid = coinbase_tx.txid();
            let mut merkle_root = [0u8; 32];
            merkle_root.copy_from_slice(&txid);

            // Create block header template
            let mut header = BlockHeader {
                version: bitquan_types::GENESIS_VERSION,
                prev_block: prev_hash,
                merkle_root,
                pqc_agg_hint: [0u8; 32],
                time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as u32,
                bits,
                nonce: 0,
                algo_id: 0, // SHA256d
            };

            // Mine the block using simple SHA256d PoW
            let engine = Sha256dEngine;
            // Use higher limit for regtest/devnet (easy difficulty still needs many attempts)
            let max_nonce = if self.chain_name == "regtest" || self.chain_name == "devnet" {
                100_000_000 // 100M attempts for testing
            } else {
                1_000_000 // 1M for production
            };

            let mut found = false;
            for nonce in 0..max_nonce {
                header.nonce = nonce;

                // Verify if this nonce meets the target
                if engine.verify(&header).is_ok() {
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(RpcError::InternalError(format!(
                    "Failed to mine block {} after {} attempts",
                    i, max_nonce
                )));
            }

            // Fetch pending transactions from mempool (if available)
            let mut transactions = vec![coinbase_tx];

            if let Some(mempool) = &self.mempool {
                let mut mp = mempool.lock().await;
                // Select transactions up to 4M weight units (standard block weight)
                let selected = mp.select_for_block(4_000_000);
                if !selected.is_empty() {
                    log::info!("Mining block with {} mempool transactions", selected.len());
                    transactions.extend(selected);
                }
            } else {
                log::info!("Warning: No mempool available for mining");
            }

            // Recalculate merkle root including all transactions
            let merkle_root = bitquan_consensus::calculate_merkle_root(&transactions)
                .map_err(|e| RpcError::InternalError(format!("merkle root: {}", e)))?;
            header.merkle_root = merkle_root;

            // Create full block
            let block = Block {
                header: header.clone(),
                transactions,
            };

            // Insert block into storage
            self.store
                .insert_block(block)
                .await
                .map_err(Self::storage_error_to_rpc)?;

            let block_hash = bitquan_consensus::header_hash(&header);
            generated_hashes.push(hex::encode(block_hash));

            // Update prev_hash for next block
            prev_hash = block_hash;
        }

        Ok(generated_hashes)
    }

    async fn sendtoaddress(
        &self,
        address: String,
        amount: u128,
        _comment: Option<String>,
    ) -> Result<String, RpcError> {
        use bitquan_types::NetworkId;
        use std::env;
        use std::path::Path;

        // For testing: load miner wallet from default location
        // SECURITY: Password must be provided via environment variable
        let wallet_path = Path::new("miner_wallet.json");
        let wallet_password = env::var("BITQUAN_WALLET_PASSWORD").map_err(|_| {
            RpcError::InternalError(
                "BITQUAN_WALLET_PASSWORD environment variable not set. \
                 Set with: export BITQUAN_WALLET_PASSWORD=\"your-password\""
                    .to_string(),
            )
        })?;

        if wallet_password.is_empty() {
            return Err(RpcError::InternalError(
                "BITQUAN_WALLET_PASSWORD cannot be empty".to_string(),
            ));
        }

        // SECURITY: Input validation for address and amount
        if address.trim().is_empty() || address.len() > 500 {
            return Err(RpcError::InvalidParams("Invalid address".to_string()));
        }

        if amount == 0 {
            return Err(RpcError::InvalidParams(
                "Amount must be greater than zero".to_string(),
            ));
        }

        // Maximum reasonable amount (prevent overflow attacks)
        const MAX_SEND_AMOUNT: u128 = 1_000_000_000_000_000; // 1 trillion BQ (very high limit)
        if amount > MAX_SEND_AMOUNT {
            return Err(RpcError::InvalidParams(
                "Amount exceeds maximum allowed".to_string(),
            ));
        }

        // Log warning if using insecure default password
        if wallet_password == "miner_dev_password" {
            log::warn!(
                "⚠️  WARNING: Using INSECURE default wallet password! \
                       Set BITQUAN_WALLET_PASSWORD env var with a strong password."
            );
        }

        let wallet = WalletKeypair::load_from_file(wallet_path, &wallet_password)
            .map_err(|e| RpcError::InternalError(format!("Failed to load wallet: {}", e)))?;

        // Parse recipient address
        let recipient_pubkey_hash = decode_bech32m(&address)
            .map_err(|e| RpcError::InvalidParams(format!("Invalid recipient address: {}", e)))?;
        let recipient_script = script_from_pubkey_hash(&recipient_pubkey_hash);

        // Find a spendable UTXO (coinbase from block >= 2, since maturity is 100 blocks)
        let height = self
            .store
            .height()
            .await
            .map_err(Self::storage_error_to_rpc)?;
        if height < 101 {
            return Err(RpcError::InternalError(
                "Coinbase maturity not reached (need 101 blocks)".to_string(),
            ));
        }

        // Get block 2 (first mature coinbase)
        let block = self
            .store
            .get_block_by_height(2)
            .await
            .map_err(Self::storage_error_to_rpc)?
            .ok_or_else(|| RpcError::InternalError("Block 2 not found".to_string()))?;

        if block.transactions.is_empty() {
            return Err(RpcError::InternalError("Invalid block state".to_string()));
        }

        let coinbase_tx = &block.transactions[0];
        let coinbase_txid = coinbase_tx.txid();

        if coinbase_tx.outputs.is_empty() {
            return Err(RpcError::InternalError(
                "Invalid transaction state".to_string(),
            ));
        }

        // Get the coinbase output value (50 BQ = 5,000,000,000 satoshis)
        let input_value = coinbase_tx.outputs[0].value;
        let output_value = amount;

        if output_value > input_value {
            return Err(RpcError::InternalError(format!(
                "Insufficient funds: have {} qbits, need {}",
                input_value, output_value
            )));
        }

        // Calculate change (for simplicity, send change back to sender)
        // Estimate fee: ~10KB for Dilithium transaction @ 1 sat/byte = 10,000 satoshis
        let estimated_fee = 10_000u64;
        let change_value = input_value - output_value - (estimated_fee as u128);
        let sender_pubkey_hash = wallet.public_key_hash();
        let change_script = script_from_pubkey_hash(&sender_pubkey_hash);

        // Build transaction
        let tx = TransactionBuilder::new()
            .network(NetworkId::Regtest)
            .add_input(coinbase_txid, 0, input_value)
            .add_output(recipient_script, output_value)
            .add_output(change_script, change_value);

        // Sign transaction with wallet
        let tx = tx
            .build_and_sign(|msg| {
                wallet.sign(msg).map_err(|e| {
                    bitquan_types::error::Error::Invalid(format!("Signing failed: {}", e))
                })
            })
            .map_err(|e| RpcError::InternalError(format!("Failed to build transaction: {}", e)))?;

        let txid = tx.txid();

        // Submit to mempool if available
        if let Some(mempool) = &self.mempool {
            let mut mp = mempool.lock().await;
            mp.insert(tx.clone(), estimated_fee)
                .map_err(|e| RpcError::InternalError(format!("Failed to add to mempool: {}", e)))?;
        } else {
            return Err(RpcError::InternalError("No mempool available".to_string()));
        }

        Ok(hex::encode(txid))
    }
}

fn storage_to_rpc(err: StorageError) -> RpcError {
    RpcError::InternalError(err.to_string())
}

fn difficulty_from_bits(bits: u32) -> f64 {
    let max_target = bitquan_consensus::compact_to_target(GENESIS_BITS);
    let target = bitquan_consensus::compact_to_target(bits);
    if target == 0 {
        return 0.0;
    }
    (max_target / target) as f64
}

struct TransactionSummary {
    txid: String,
    version: i32,
    lock_time: u32,
    vin_count: usize,
    vout_count: usize,
    value_out: u128,
}

impl From<Transaction> for TransactionSummary {
    fn from(tx: Transaction) -> Self {
        // Use saturating_add to prevent overflow when summing output values
        let value_out = tx
            .outputs
            .iter()
            .fold(0u128, |acc, o| acc.saturating_add(o.value));
        Self {
            txid: hex::encode(tx.txid()),
            version: tx.version,
            lock_time: tx.lock_time,
            vin_count: tx.inputs.len(),
            vout_count: tx.outputs.len(),
            value_out,
        }
    }
}

impl From<TransactionSummary> for TxInfo {
    fn from(summary: TransactionSummary) -> Self {
        TxInfo {
            txid: summary.txid,
            version: summary.version,
            locktime: summary.lock_time,
            vin_count: summary.vin_count,
            vout_count: summary.vout_count,
            value_out: summary.value_out,
        }
    }
}
