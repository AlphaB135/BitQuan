//! Consensus rule scaffolding for BitQuan.
#![warn(missing_docs)]

use bitquan_types::{count_signatures, Block, NetworkId};
use bq_crypto::{CryptoError, CryptoRegistry};
use thiserror::Error;

mod asert;
mod difficulty;
pub mod fork;
pub mod pow;
pub mod script;
pub mod sighash;
pub mod utxo;

#[cfg(test)]
mod tests;

pub use asert::{asert_next_target, BurstGuardState, GuardContext};
pub use difficulty::{compact_to_target, target_to_compact, DifficultyState};
pub use fork::{BlockNode, ForkChoice, ForkError, ReorgInfo};
pub use pow::{
    check_header_pow, clamp_bits_within_bounds, compact_to_target_bytes, header_hash, PowError,
    DEVNET_MAX_BITS, DEVNET_MIN_BITS,
};
pub use script::{verify_script, OpCode, ScriptError, ScriptInterpreter, MAX_SCRIPT_SIZE};
pub use sighash::{compute_sighash_with_context, transaction_sighash};
pub use utxo::{OutPoint, UtxoEntry, UtxoError, UtxoSet};

/// Difficulty retarget parameters (ASERT + burst guard activation).
#[derive(Clone, Debug)]
pub struct DifficultyParams {
    /// Target block interval in seconds.
    pub target_block_time: u64,
    /// ASERT/LWMA half-life in seconds for difficulty retargeting.
    pub difficulty_half_life: u64,
    /// Burst guard window (blocks) for rapid difficulty increases.
    pub burst_guard_window: u64,
    /// Minimum ratio of observed/expected time before burst guard engages.
    pub burst_guard_floor_ratio: f64,
    /// Ratio above which the burst guard releases (hysteresis).
    pub burst_guard_release_ratio: f64,
    /// Difficulty multiplier applied when burst guard triggers.
    pub burst_guard_multiplier: f64,
    /// Cooldown period (blocks) before the guard may trigger again.
    pub burst_guard_cooldown_blocks: u64,
    /// Height at which the burst guard becomes active.
    pub burst_guard_activation_height: u64,
}

impl DifficultyParams {
    /// Returns the default Phase 3 difficulty configuration.
    pub fn phase3_defaults() -> Self {
        Self {
            target_block_time: 600,
            difficulty_half_life: 14_400,
            burst_guard_window: 11,
            burst_guard_floor_ratio: 0.33,
            burst_guard_release_ratio: 0.38,
            burst_guard_multiplier: 1.5,
            burst_guard_cooldown_blocks: 5,
            burst_guard_activation_height: 0,
        }
    }
}

/// Proof-of-Work algorithm set parameters.
#[derive(Clone, Debug)]
pub struct PowSetParams {
    /// Height at which multiple PoW algorithms are activated.
    pub activated_height: u64,
    /// List of allowed PoW algorithms.
    pub allowed_algos: Vec<pow::PowAlgo>,
    /// Default algorithm to use before activation height.
    pub default_algo: pow::PowAlgo,
}

impl PowSetParams {
    /// Mainnet configuration (SHA-256d only, hybrid disabled).
    pub fn mainnet() -> Self {
        Self {
            activated_height: u64::MAX, // Never activate hybrid on mainnet
            allowed_algos: vec![pow::PowAlgo::Sha256d],
            default_algo: pow::PowAlgo::Sha256d,
        }
    }

    /// Testnet configuration (hybrid enabled at height 1000).
    #[cfg(feature = "randomx")]
    pub fn testnet_hybrid() -> Self {
        Self {
            activated_height: 1000,
            allowed_algos: vec![pow::PowAlgo::Sha256d, pow::PowAlgo::RandomX],
            default_algo: pow::PowAlgo::Sha256d,
        }
    }

    /// Devnet configuration (hybrid enabled from genesis).
    #[cfg(feature = "randomx")]
    pub fn devnet_hybrid() -> Self {
        Self {
            activated_height: 0,
            allowed_algos: vec![pow::PowAlgo::Sha256d, pow::PowAlgo::RandomX],
            default_algo: pow::PowAlgo::Sha256d,
        }
    }

    /// Check if an algorithm is allowed at given height.
    pub fn is_algo_allowed(&self, algo: pow::PowAlgo, height: u64) -> bool {
        if height < self.activated_height {
            algo == self.default_algo
        } else {
            self.allowed_algos.contains(&algo)
        }
    }
}

/// Parameters controlling consensus validation.
#[derive(Clone, Debug)]
pub struct ConsensusParams {
    /// Maximum permitted block weight.
    pub block_weight_cap: u64,
    /// Weight multiplier applied per PQ signature.
    pub signature_weight_alpha: u32,
    /// Witness byte discount coefficient (0.0..=1.0).
    pub witness_weight_beta: f32,
    /// Difficulty retarget parameters.
    pub difficulty: DifficultyParams,
    /// Block reward schedule parameters.
    pub reward_schedule: RewardSchedule,
    /// Proof-of-Work algorithm set parameters.
    pub pow_set: PowSetParams,
}

impl ConsensusParams {
    /// Returns the default Phase 3 configuration (mainnet).
    pub fn phase3_defaults() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            difficulty: DifficultyParams::phase3_defaults(),
            reward_schedule: RewardSchedule::phase3_defaults(),
            pow_set: PowSetParams::mainnet(),
        }
    }

    /// Returns testnet configuration with hybrid PoW enabled.
    #[cfg(feature = "randomx")]
    pub fn testnet_hybrid() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            difficulty: DifficultyParams::phase3_defaults(),
            reward_schedule: RewardSchedule::phase3_defaults(),
            pow_set: PowSetParams::testnet_hybrid(),
        }
    }

    /// Returns devnet configuration with hybrid PoW enabled.
    #[cfg(feature = "randomx")]
    pub fn devnet_hybrid() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            witness_weight_beta: 0.5,
            difficulty: DifficultyParams::phase3_defaults(),
            reward_schedule: RewardSchedule::phase3_defaults(),
            pow_set: PowSetParams::devnet_hybrid(),
        }
    }
}

/// Policy controls for mempool admission and DoS protection.
#[derive(Clone, Debug)]
pub struct MempoolPolicy {
    /// Maximum size of any script (scriptSig/scriptPubKey) in bytes.
    pub max_scriptsize: u32,
    /// Maximum number of inputs permitted per transaction.
    pub max_inputs_per_tx: u32,
    /// Maximum number of signature operations permitted per transaction.
    pub max_sigops_per_tx: u32,
    /// Minimum relay fee per weight unit (qbits/WU).
    pub min_relay_fee_per_wu: u64,
    /// Maximum number of in-mempool ancestors allowed.
    pub ancestor_limit: u32,
    /// Maximum number of in-mempool descendants allowed.
    pub descendant_limit: u32,
}

impl MempoolPolicy {
    /// Standard policy suitable for devnet/testnet usage.
    pub fn standard() -> Self {
        Self {
            max_scriptsize: 10_000,
            max_inputs_per_tx: 256,
            max_sigops_per_tx: 80_000,
            min_relay_fee_per_wu: 1,
            ancestor_limit: 50,
            descendant_limit: 50,
        }
    }
}

/// Aggregated parameters for a specific BitQuan network.
#[derive(Clone, Debug)]
pub struct NetworkParams {
    /// Network identifier (mainnet/testnet/devnet/regtest).
    pub network_id: NetworkId,
    /// Genesis hash associated with the network.
    pub genesis_hash: [u8; 32],
    /// Consensus parameters for validation.
    pub consensus: ConsensusParams,
    /// Mempool admission and DoS policy.
    pub mempool: MempoolPolicy,
}

impl NetworkParams {
    /// Returns devnet defaults.
    pub fn devnet() -> Self {
        Self {
            network_id: NetworkId::Devnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
            consensus: ConsensusParams::phase3_defaults(),
            mempool: MempoolPolicy::standard(),
        }
    }

    /// Returns testnet defaults (currently identical to devnet).
    pub fn testnet() -> Self {
        Self {
            network_id: NetworkId::Testnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
            consensus: ConsensusParams::phase3_defaults(),
            mempool: MempoolPolicy::standard(),
        }
    }

    /// Returns mainnet defaults with conservative guard activation.
    pub fn mainnet() -> Self {
        let mut consensus = ConsensusParams::phase3_defaults();
        consensus.difficulty.burst_guard_activation_height = 100_000;
        Self {
            network_id: NetworkId::Mainnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
            consensus,
            mempool: MempoolPolicy::standard(),
        }
    }
}

/// Describes the block reward schedule (halvings + tail emission).
#[derive(Clone, Debug)]
pub struct RewardSchedule {
    /// Initial subsidy in the smallest unit (1 BQ = 10^8 units).
    pub initial_subsidy: u64,
    /// Number of blocks between halvings.
    pub halving_interval: u64,
    /// Tail emission paid per block once halvings decay beneath this value.
    pub tail_emission_per_block: u64,
}

impl RewardSchedule {
    /// Returns the default Phase 3 reward parameters.
    pub fn phase3_defaults() -> Self {
        Self {
            initial_subsidy: 5_000_000_000, // 50 BQ
            halving_interval: 210_000,
            tail_emission_per_block: 50_000_000, // 0.5 BQ
        }
    }

    /// Calculates the subsidy for the given block height (zero-indexed).
    pub fn subsidy_at_height(&self, height: u64) -> u64 {
        if self.halving_interval == 0 {
            return self.tail_emission_per_block.max(self.initial_subsidy);
        }

        let halvings = height / self.halving_interval;
        if halvings >= 63 {
            return self.tail_emission_per_block;
        }

        let candidate = self.initial_subsidy >> (halvings as u32);
        if candidate < self.tail_emission_per_block {
            self.tail_emission_per_block
        } else {
            candidate
        }
    }
}

/// Resulting metrics from block validation.
#[derive(Clone, Debug)]
pub struct BlockValidationReport {
    /// Final weight attributable to the assessed block.
    pub block_weight: u64,
    /// Number of signatures encountered.
    pub signature_count: u64,
    /// Subsidy scheduled for the validated block height.
    pub block_subsidy: u64,
}

/// Errors emitted when consensus validation fails.
#[derive(Debug, Error)]
pub enum ConsensusError {
    /// The block exceeds the configured weight constraints.
    #[error("block weight {actual} exceeds limit {limit}")]
    BlockWeightExceeded {
        /// Actual block weight
        actual: u64,
        /// Maximum allowed weight
        limit: u64,
    },
    /// Merkle root in header does not match computed root of txids.
    #[error("merkle_root mismatch")]
    MerkleRootMismatch,
    /// Witness root in header does not match computed root of wtxids.
    #[error("witness_root mismatch")]
    WitnessRootMismatch,
    /// Coinbase transaction missing or not first.
    #[error("coinbase missing or misordered")]
    CoinbaseMissing,
    /// More than one coinbase detected.
    #[error("multiple coinbase transactions")]
    MultipleCoinbase,
    /// Coinbase value exceeds allowed subsidy (fees placeholder=0).
    #[error("coinbase exceeds allowed subsidy")]
    CoinbaseExceedsSubsidy,
    /// Signature verification failed.
    #[error("signature verification failed: {0}")]
    Signature(#[from] CryptoError),
    /// Arithmetic overflow in weight calculation.
    #[error("overflow in {0}")]
    WeightOverflow(&'static str),
    /// Invalid signature hash computation.
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
}

/// Calculates transaction weight according to BQIP-0002.
///
/// Formula: weight = (base_size * 4) + (signature_count * 384)
pub fn calculate_tx_weight(tx: &bitquan_types::Transaction) -> Result<usize, ConsensusError> {
    const WITNESS_SCALE_FACTOR: usize = 4;
    const SIGNATURE_WEIGHT: usize = 384;

    // Base size: transaction without witness data
    let serialized_size = tx
        .serialized_size_hint()
        .map_err(|_| ConsensusError::WeightOverflow("transaction serialized size calculation"))?;
    let witness_size = tx
        .witness_size_hint()
        .map_err(|_| ConsensusError::WeightOverflow("transaction witness size calculation"))?;
    let base_size =
        serialized_size
            .checked_sub(witness_size)
            .ok_or(ConsensusError::WeightOverflow(
                "transaction base size calculation",
            ))?;

    // Count signatures in witnesses using checked arithmetic
    let sig_count: usize = tx.witnesses.iter().try_fold(0usize, |acc, w| {
        acc.checked_add(w.signatures.len())
            .ok_or(ConsensusError::WeightOverflow("signature count"))
    })?;

    // Calculate base weight: base_size * 4
    let base_weight = base_size
        .checked_mul(WITNESS_SCALE_FACTOR)
        .ok_or(ConsensusError::WeightOverflow("base weight calculation"))?;

    // Calculate signature weight: sig_count * 384
    let sig_weight =
        sig_count
            .checked_mul(SIGNATURE_WEIGHT)
            .ok_or(ConsensusError::WeightOverflow(
                "signature weight calculation",
            ))?;

    // Add them together
    base_weight
        .checked_add(sig_weight)
        .ok_or(ConsensusError::WeightOverflow("total transaction weight"))
}

/// Calculates block weight according to BQIP-0002.
///
/// Formula: sum of all transaction weights
pub fn calculate_block_weight(block: &Block) -> Result<usize, ConsensusError> {
    block.transactions.iter().try_fold(0usize, |acc, tx| {
        let tx_weight = calculate_tx_weight(tx)?;
        acc.checked_add(tx_weight)
            .ok_or(ConsensusError::WeightOverflow("block weight accumulation"))
    })
}

/// Legacy function - calculates the block weight given an `alpha` multiplier.
///
/// Deprecated: Use calculate_block_weight() instead for BQIP-0002 compliance.
///
/// **Note:** This function is internal-only for testing weight formulas.
/// External callers should use `calculate_block_weight()` with production parameters.
#[deprecated(note = "Use calculate_block_weight() for BQIP-0002 compliance")]
#[allow(dead_code)]
pub(crate) fn calculate_block_weight_with_beta(block: &Block, alpha: u32, beta: f32) -> u64 {
    use bitquan_types::CompactUint;
    // Total bytes (base + witness) - return 0 on error (deprecated anyway)
    let total = block.serialized_size_hint().unwrap_or(0) as u64;
    // Approximate witness bytes from tx structure (count prefix + witnesses)
    let mut witness_bytes: u64 = 0;
    for tx in &block.transactions {
        witness_bytes += CompactUint::from_usize(tx.witnesses.len()).encoded_length() as u64;
        witness_bytes += tx
            .witnesses
            .iter()
            .filter_map(|w| w.serialized_size_hint().ok())
            .map(|size| size as u64)
            .sum::<u64>();
    }
    let base_bytes = total.saturating_sub(witness_bytes);
    let signature_weight = count_signatures(block) * alpha as u64;
    let witness_weight = (beta * witness_bytes as f32).round() as u64;
    base_bytes + signature_weight + witness_weight
}

/// Validates a block against the supplied consensus parameters (BQIP-0002).
pub fn validate_block(
    block: &Block,
    height: u64,
    params: &ConsensusParams,
    registry: &CryptoRegistry,
    network_id: bitquan_types::NetworkId,
    genesis_hash: [u8; 32],
) -> Result<BlockValidationReport, ConsensusError> {
    // Calculate block weight using BQIP-0002 formula (with overflow protection)
    let block_weight = calculate_block_weight(block)?;
    let signature_count = count_signatures(block);
    let block_subsidy = params.reward_schedule.subsidy_at_height(height);

    // Enforce MAX_BLOCK_WEIGHT = 4,000,000 WU (BQIP-0002)
    if block_weight as u64 > params.block_weight_cap {
        return Err(ConsensusError::BlockWeightExceeded {
            actual: block_weight as u64,
            limit: params.block_weight_cap,
        });
    }

    // Create transaction context for signature verification
    let ctx = bitquan_types::TxContext::new(network_id, genesis_hash);

    // Verify all transaction signatures
    for tx in &block.transactions {
        let digest = transaction_sighash(tx, &ctx)
            .map_err(|e| ConsensusError::InvalidSignature(e.to_string()))?;
        registry.verify_transaction(tx, &digest)?;
    }

    Ok(BlockValidationReport {
        block_weight: block_weight as u64,
        signature_count,
        block_subsidy,
    })
}

/// Validates a transaction's signatures using TxContext.
///
/// This is a convenience function that combines sighash computation and signature verification.
pub fn validate_transaction_signatures(
    tx: &bitquan_types::Transaction,
    ctx: &bitquan_types::TxContext,
    registry: &CryptoRegistry,
) -> Result<(), ConsensusError> {
    let digest = transaction_sighash(tx, ctx)
        .map_err(|e| ConsensusError::InvalidSignature(e.to_string()))?;
    registry.verify_transaction(tx, &digest)?;
    Ok(())
}

/// Consensus engine bundling parameters, crypto registry, and RNG state.
#[allow(dead_code)]
fn is_coinbase_tx(tx: &bitquan_types::Transaction) -> bool {
    if tx.inputs.is_empty() {
        return false;
    }
    let first = &tx.inputs[0];
    first.prev_txid == [0u8; 32] && first.prev_vout == u32::MAX
}

/// Consensus engine for validating blocks and transactions
pub struct ConsensusEngine {
    /// Consensus parameters
    params: ConsensusParams,
    /// Cryptographic registry for signature verification
    registry: CryptoRegistry,
    /// Difficulty adjustment state
    difficulty: Option<DifficultyState>,
    /// Network identifier for sighash
    network_id: bitquan_types::NetworkId,
    /// Genesis block hash for transaction context
    genesis_hash: [u8; 32],
}

impl ConsensusEngine {
    /// Constructs a new engine using the supplied parameters and registry.
    pub fn new(params: ConsensusParams, registry: CryptoRegistry) -> Self {
        Self {
            params,
            registry,
            difficulty: None,
            network_id: bitquan_types::NetworkId::default(),
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_BYTES,
        }
    }

    /// Constructs a new engine with explicit network ID and genesis hash.
    pub fn with_network(
        params: ConsensusParams,
        registry: CryptoRegistry,
        network_id: bitquan_types::NetworkId,
        genesis_hash: [u8; 32],
    ) -> Self {
        Self {
            params,
            registry,
            difficulty: None,
            network_id,
            genesis_hash,
        }
    }

    /// Constructs an engine from aggregated network parameters.
    pub fn from_network(network: &NetworkParams, registry: CryptoRegistry) -> Self {
        Self::with_network(
            network.consensus.clone(),
            registry,
            network.network_id,
            network.genesis_hash,
        )
    }

    /// Returns the consensus parameters in use.
    pub fn params(&self) -> &ConsensusParams {
        &self.params
    }

    /// Sets the difficulty anchor used for subsequent ASERT calculations.
    pub fn set_difficulty_state(&mut self, state: DifficultyState) {
        self.difficulty = Some(state);
    }

    /// Computes the next compact target if anchor information is available.
    pub fn next_target_from_anchor(
        &mut self,
        next_height: u64,
        next_timestamp: u64,
    ) -> Option<u32> {
        let state = self.difficulty.as_mut()?;
        Some(state.update(next_height, next_timestamp, &self.params))
    }

    /// Computes the next target using ASERT relative to an anchor.
    pub fn next_difficulty_target(
        &self,
        anchor_target: f64,
        height_delta: i64,
        time_delta: i64,
    ) -> f64 {
        asert_next_target(anchor_target, height_delta, time_delta, &self.params, None)
    }

    /// Validates a block using the stored registry and RNG state.
    pub fn validate_block(
        &mut self,
        block: &Block,
        height: u64,
    ) -> Result<BlockValidationReport, ConsensusError> {
        validate_block(
            block,
            height,
            &self.params,
            &self.registry,
            self.network_id,
            self.genesis_hash,
        )
    }
}
