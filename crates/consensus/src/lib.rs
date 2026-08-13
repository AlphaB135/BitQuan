//! Consensus rule scaffolding for BitQuan.
#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use bitquan_types::{count_signatures, Block, NetworkId};
use blake3::Hasher;
use bq_crypto::{CryptoError, CryptoRegistry};
use rayon::prelude::*;
use thiserror::Error;

mod asert;
pub mod difficulty;

pub mod fork;
mod monitoring;
pub mod pow;
pub mod script;
pub mod sighash;
pub mod utxo;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod comprehensive_tests;

pub use asert::{asert_next_target, BurstGuardState, GuardContext, FP_SCALE};
pub use difficulty::{
    compact_to_target, target_to_compact, target_to_compact_u64, DifficultyState,
};

pub use fork::{BlockNode, ForkChoice, ForkError, ReorgInfo};
pub use monitoring::{
    Alert, AlertSeverity, Monitor, MonitorConfig, MonitorError, MonitorEventType, MonitorStats,
};
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
    pub burst_guard_floor_ratio_fp: u64, // Fixed-point representation (32.32 format)
    /// Ratio above which the burst guard releases (hysteresis).
    pub burst_guard_release_ratio_fp: u64, // Fixed-point representation (32.32 format)
    /// Difficulty multiplier applied when burst guard triggers.
    pub burst_guard_multiplier_fp: u64, // Fixed-point representation (32.32 format)
    /// Cooldown period (blocks) before the guard may trigger again.
    pub burst_guard_cooldown_blocks: u64,
    /// Height at which the burst guard becomes active.
    pub burst_guard_activation_height: u64,
}

impl DifficultyParams {
    /// Returns the default Phase 3 difficulty configuration.
    /// Uses mainnet parameters for production safety.
    pub fn phase3_defaults() -> Self {
        Self::mainnet()
    }

    /// Mainnet difficulty configuration per BIP-0340.
    ///
    /// Uses a 2-day half-life (172,800 seconds) for smoother difficulty
    /// adjustments on production networks with stable hashrate.
    ///
    /// # ASERT Parameters (BIP-0340 compliant)
    /// - `difficulty_half_life`: 172,800s (2 days) - time for difficulty to halve/double
    /// - `target_block_time`: 600s (10 minutes)
    ///
    /// # Burst Guard
    /// Protects against rapid block bursts from hashrate spikes.
    pub fn mainnet() -> Self {
        Self {
            target_block_time: 120, // Reduced from 600s (10 min) to 120s (2 min) for Phase 4
            difficulty_half_life: 14_400,
            burst_guard_window: 11,
            burst_guard_floor_ratio_fp: 1417339207, // 0.33 in 32.32 fixed-point
            burst_guard_release_ratio_fp: 1632087572, // 0.38 in 32.32 fixed-point
            burst_guard_multiplier_fp: 6442450944,  // 1.5 in 32.32 fixed-point
            burst_guard_cooldown_blocks: 5,
            burst_guard_activation_height: 0,
        }
    }

    /// Testnet/devnet difficulty configuration for faster adjustment.
    ///
    /// Uses a 4-hour half-life (14,400 seconds) for quicker difficulty
    /// adjustments on test networks where hashrate may vary significantly.
    ///
    /// # ASERT Parameters (accelerated for testing)
    /// - `difficulty_half_life`: 14,400s (4 hours) - faster response to hashrate changes
    /// - `target_block_time`: 120s (2 minutes)
    ///
    /// # Use Cases
    /// - Test networks with variable hashrate
    /// - Development environments
    /// - Regtest mode
    pub fn testnet() -> Self {
        Self {
            target_block_time: 120,
            difficulty_half_life: 14_400, // 4 hours for faster testnet adjustment
            burst_guard_window: 11,
            burst_guard_floor_ratio_fp: 1417339207, // 0.33 in 32.32 fixed-point
            burst_guard_release_ratio_fp: 1632087572, // 0.38 in 32.32 fixed-point
            burst_guard_multiplier_fp: 6442450944,  // 1.5 in 32.32 fixed-point
            burst_guard_cooldown_blocks: 5,
            burst_guard_activation_height: 0,
        }
    }

    /// Regtest difficulty configuration for instant mining.
    ///
    /// Uses a very short half-life for development/testing where
    /// blocks need to be mined quickly without waiting for difficulty
    /// to adjust naturally.
    pub fn regtest() -> Self {
        Self {
            target_block_time: 120,
            difficulty_half_life: 120, // 2 minutes for instant adjustment
            burst_guard_window: 0,     // Disable burst guard for regtest
            burst_guard_floor_ratio_fp: 0,
            burst_guard_release_ratio_fp: 0,
            burst_guard_multiplier_fp: 0,
            burst_guard_cooldown_blocks: 0,
            burst_guard_activation_height: u64::MAX, // Never activate
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
    /// Mainnet configuration (all algorithms enabled, hybrid activated at block 10000).
    pub fn mainnet() -> Self {
        Self {
            activated_height: 10000, // Activate hybrid mining at block 10000
            allowed_algos: vec![
                pow::PowAlgo::Sha256d,
                pow::PowAlgo::RandomX,
                pow::PowAlgo::Ethash,
            ],
            default_algo: pow::PowAlgo::Sha256d,
        }
    }

    /// Testnet configuration (hybrid enabled at height 1000).
    pub fn testnet_hybrid() -> Self {
        Self {
            activated_height: 1000,
            allowed_algos: vec![
                pow::PowAlgo::Sha256d,
                pow::PowAlgo::RandomX,
                pow::PowAlgo::Ethash,
            ],
            default_algo: pow::PowAlgo::Sha256d,
        }
    }

    /// Devnet configuration (hybrid enabled from genesis).
    pub fn devnet_hybrid() -> Self {
        Self {
            activated_height: 0,
            allowed_algos: vec![
                pow::PowAlgo::Sha256d,
                pow::PowAlgo::RandomX,
                pow::PowAlgo::Ethash,
            ],
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
            difficulty: DifficultyParams::phase3_defaults(),
            reward_schedule: RewardSchedule::phase3_defaults(),
            pow_set: PowSetParams::mainnet(),
        }
    }

    /// Returns testnet configuration with hybrid PoW enabled.
    pub fn testnet_hybrid() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            difficulty: DifficultyParams::phase3_defaults(),
            reward_schedule: RewardSchedule::phase3_defaults(),
            pow_set: PowSetParams::testnet_hybrid(),
        }
    }

    /// Returns devnet configuration with hybrid PoW enabled.
    pub fn devnet_hybrid() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
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
    /// Dust threshold in qbits (satoshis).
    pub dust_threshold: u128,
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
            dust_threshold: 546,
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
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_DEVNET_BYTES,
            consensus: ConsensusParams::phase3_defaults(),
            mempool: MempoolPolicy::standard(),
        }
    }

    /// Returns testnet defaults (currently identical to devnet).
    pub fn testnet() -> Self {
        Self {
            network_id: NetworkId::Testnet,
            genesis_hash: bitquan_types::genesis::GENESIS_HASH_TESTNET_BYTES,
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
    /// Initial subsidy in the smallest unit (1 BQ = 10^18 qbits).
    pub initial_subsidy: u128,
    /// Number of blocks between halvings.
    pub halving_interval: u64,
    /// Tail emission paid per block once halvings decay beneath this value.
    pub tail_emission_per_block: u128,
}

impl RewardSchedule {
    /// Returns the default Phase 3 reward parameters.
    pub fn phase3_defaults() -> Self {
        Self {
            initial_subsidy: 50_000_000_000_000_000_000, // 50 BQ (18 decimals)
            halving_interval: 210_000,
            tail_emission_per_block: 500_000_000_000_000_000, // 0.5 BQ (18 decimals)
        }
    }

    /// Calculates the subsidy for the given block height (zero-indexed).
    pub fn subsidy_at_height(&self, height: u64) -> u128 {
        if self.halving_interval == 0 {
            return self.tail_emission_per_block.max(self.initial_subsidy);
        }

        let halvings = height / self.halving_interval;
        if halvings >= 127 {
            // u128 supports up to 127 shifts (vs 63 for u64)
            return self.tail_emission_per_block;
        }

        let candidate = self.initial_subsidy >> (halvings as u32);
        if candidate < self.tail_emission_per_block {
            self.tail_emission_per_block
        } else {
            candidate
        }
    }

    /// Calculates the uncle reward (for the miner of the uncle block).
    pub fn uncle_reward(&self, block_height: u64, uncle_height: u64) -> u128 {
        let base_subsidy = self.subsidy_at_height(block_height);
        let depth = block_height.saturating_sub(uncle_height);
        if depth == 0 || depth > 7 {
            return 0; // Uncles must be strictly before and within 7 blocks
        }
        // (8 - depth) * Base Reward / 8
        let multiplier = 8 - depth as u128;
        (base_subsidy * multiplier) / 8
    }

    /// Calculates the nephew reward (bonus for the current miner per uncle included).
    pub fn nephew_reward(&self, block_height: u64, uncles_count: usize) -> u128 {
        let base_subsidy = self.subsidy_at_height(block_height);
        let bonus_per_uncle = base_subsidy / 32;
        bonus_per_uncle * (uncles_count as u128)
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
    pub block_subsidy: u128,
}

/// Contextual information about an uncle block required for validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncleContext {
    /// The uncle's block header
    pub header: bitquan_types::BlockHeader,
    /// The block height where the uncle was mined
    pub height: u64,
    /// The payout script (script_pubkey) for the uncle miner
    pub payout_script: Vec<u8>,
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
    /// Output value is below dust threshold.
    #[error("dust output at index {index}: value {value} < threshold {threshold}")]
    DustOutput {
        /// Index of the output
        index: usize,
        /// Value of the output
        value: u128,
        /// Dust threshold
        threshold: u128,
    },
    /// Block timestamp is too far in the future.
    #[error("block timestamp {0} too far in future (max {1})")]
    TimestampTooFarInFuture(u64, u64),
    /// Block timestamp is at or below median time past.
    #[error("block timestamp {0} at or below median time past {1}")]
    TimestampBelowMTP(u64, u64),
    /// Invalid difficulty target value.
    #[error("invalid difficulty target: {0:#x}")]
    InvalidDifficultyTarget(u32),
    /// Proof-of-Work hash does not meet target.
    #[error("invalid proof of work: {0}")]
    InvalidPoW(String),
    /// Invalid uncle block.
    #[error("invalid uncle: {0}")]
    InvalidUncle(String),
    /// Coinbase validation failed.
    #[error("invalid coinbase: {0}")]
    InvalidCoinbase(String),
    /// Fee validation failed.
    #[error("fee validation: {0}")]
    FeeValidation(String),
}

/// Calculates transaction weight according to BQIP-0007 (BQSegWit).
///
/// Formula: `weight = (base_bytes × 4) + (witness_bytes × 1)`
///
/// This gives witness data (Dilithium5 signatures) a 4x discount compared to
/// base transaction data, allowing ~4x more transactions per block.
/// Equivalent to Bitcoin's SegWit weight formula.
pub fn calculate_tx_weight(tx: &bitquan_types::Transaction) -> Result<usize, ConsensusError> {
    // Witness scale factor: base data costs 4 weight units per byte
    const WITNESS_SCALE_FACTOR: usize = 4;

    // Total serialized size (base + witness)
    let total_size = tx
        .serialized_size_hint()
        .map_err(|_| ConsensusError::WeightOverflow("transaction serialized size calculation"))?;

    // Witness-only size (Dilithium5 signatures + pubkeys)
    let witness_size = tx
        .witness_size_hint()
        .map_err(|_| ConsensusError::WeightOverflow("transaction witness size calculation"))?;

    // Base size = total minus witness
    let base_size = total_size
        .checked_sub(witness_size)
        .ok_or(ConsensusError::WeightOverflow(
            "transaction base size calculation",
        ))?;

    // BQIP-0007: weight = base_bytes*4 + witness_bytes*1
    // Witness (signatures) get a 4x discount → 4x more txs fit per block
    let base_weight = base_size
        .checked_mul(WITNESS_SCALE_FACTOR)
        .ok_or(ConsensusError::WeightOverflow("base weight calculation"))?;

    // witness_bytes × 1 (discount factor)
    base_weight
        .checked_add(witness_size)
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
#[allow(dead_code)] // Deprecated API - kept for potential external references
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
#[allow(clippy::too_many_arguments)]
pub fn validate_block(
    block: &Block,
    height: u64,
    params: &ConsensusParams,
    registry: &CryptoRegistry,
    network_id: bitquan_types::NetworkId,
    genesis_hash: [u8; 32],
    total_fees: Option<u128>,
    median_time_past: u64,
    network_adjusted_time: u64,
    expected_bits: Option<u32>,
    _expected_uncles_bits: Option<&[u32]>,
    _uncles_ctx: &[UncleContext],
    _past_uncle_hashes: &std::collections::HashSet<[u8; 32]>,
) -> Result<BlockValidationReport, ConsensusError> {
    // Bitcoin-style block header validation (includes ASERT difficulty enforcement)
    validate_block_header(
        block,
        height,
        params,
        median_time_past,
        network_adjusted_time,
        expected_bits,
        &genesis_hash,
    )?;

    // CRITICAL: Validate witness root against actual transaction witness data
    // Without this, an attacker can submit forged PQC signatures that pass header validation.
    let computed_witness_root = block
        .compute_witness_root()
        .map_err(|_| ConsensusError::WitnessRootMismatch)?;
    if computed_witness_root != block.header.pqc_agg_hint {
        return Err(ConsensusError::WitnessRootMismatch);
    }

    // Coinbase transaction validation
    validate_coinbase_transaction(block, height)?;

    // Enforce NO UNCLE BLOCKS. Uncle/GHOST protocol is inappropriate for a 120s block time
    // PoW chain. It creates unnecessary complexity and potential attack vectors.
    if !block.uncles.is_empty() {
        return Err(ConsensusError::InvalidUncle(
            "Uncle blocks are deprecated and not supported".to_string(),
        ));
    }

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

    // Validate all transactions (e.g. dust checks)
    for tx in &block.transactions {
        validate_transaction(tx)?;
    }

    // Validate transaction fees and rewards
    validate_transaction_fees(block, height, params, total_fees, _uncles_ctx)?;

    // Create transaction context for signature verification
    let ctx = bitquan_types::TxContext::new(network_id, genesis_hash);

    // Verify all transaction signatures (PARALLEL + DETERMINISTIC)
    // - Parallel execution for speed (Dilithium5 is expensive!)
    // - find_first guarantees first invalid tx (by index) is returned
    let first_failure = block
        .transactions
        .par_iter()
        .map(|tx| {
            let digest = transaction_sighash(tx, &ctx)
                .map_err(|e| ConsensusError::InvalidSignature(e.to_string()))?;
            registry.verify_transaction(tx, &digest)?;
            Ok::<(), ConsensusError>(())
        })
        .find_first(|res| res.is_err());

    if let Some(Err(e)) = first_failure {
        return Err(e);
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

/// Validates block header according to Bitcoin-style rules.
///
/// # Arguments
/// * `expected_bits` - ASERT-computed compact target for this height. The caller
///   must compute this via [`DifficultyState::peek_next_bits`] and pass it here.
///   SECURITY: Passing `None` disables difficulty enforcement; only safe for
///   genesis (height 0) or contexts with no difficulty anchor.
/// * `network_adjusted_time` - Caller provides NTP-synced median peer time.
///   SECURITY: Never call SystemTime::now() inside consensus — non-deterministic
///   across replays and exploitable via NTP poisoning.
fn validate_block_header(
    block: &Block,
    height: u64,
    params: &ConsensusParams,
    median_time_past: u64,
    network_adjusted_time: u64,
    expected_bits: Option<u32>,
    genesis_hash: &[u8; 32],
) -> Result<(), ConsensusError> {
    let header = &block.header;

    // Genesis block has no parent
    if height > 0 {
        // SECURITY: Use network-adjusted time instead of SystemTime::now()
        // to prevent timejacking attacks and ensure deterministic replay.
        let max_future_time = network_adjusted_time + 7200;
        let block_time = u64::from(header.time);
        if block_time > max_future_time {
            return Err(ConsensusError::TimestampTooFarInFuture(
                block_time,
                max_future_time,
            ));
        }

        // Validate timestamp is greater than median of past 11 blocks
        if block_time <= median_time_past {
            return Err(ConsensusError::TimestampBelowMTP(
                block_time,
                median_time_past,
            ));
        }
    }

    // Validate proof of work target — range check first.
    let target = header.bits;
    if target == 0 || target > 0x2100ffff {
        return Err(ConsensusError::InvalidDifficultyTarget(target));
    }

    // SECURITY: Enforce ASERT-computed target. Without this check a miner can
    // submit blocks with bits = 0x207fffff (easiest difficulty, ~1 hash) and
    // the node would accept them — allowing chain takeover in seconds.
    // Ref: issue #187
    if let Some(exp_bits) = expected_bits {
        if target != exp_bits {
            return Err(ConsensusError::InvalidDifficultyTarget(target));
        }
    }

    // CRITICAL: Validate proof-of-work hash meets target.
    // Previously check_header_pow returned Result<bool> and the bool was discarded,
    // allowing blocks with invalid PoW to pass validation.
    let pow_valid = crate::pow::check_header_pow(header, height, &params.pow_set, genesis_hash)
        .map_err(|e| ConsensusError::InvalidPoW(format!("{e}")))?;
    if !pow_valid {
        return Err(ConsensusError::InvalidPoW(
            "hash does not meet target".into(),
        ));
    }

    // CRITICAL: Validate merkle root using Block::compute_merkle_root() which uses
    // BLAKE3 over base-only txids. Previously used calculate_merkle_root() which
    // used BLAKE3 over witness-including hashes — causing consensus split between
    // block builder and validator.
    let calculated_merkle = block
        .compute_merkle_root()
        .map_err(|_| ConsensusError::MerkleRootMismatch)?;
    if calculated_merkle != header.merkle_root {
        return Err(ConsensusError::MerkleRootMismatch);
    }

    Ok(())
}

/// Validates coinbase transaction according to Bitcoin rules
fn validate_coinbase_transaction(block: &Block, _height: u64) -> Result<(), ConsensusError> {
    if block.transactions.is_empty() {
        return Err(ConsensusError::InvalidCoinbase(
            "Block must contain at least one transaction".to_string(),
        ));
    }

    let coinbase = &block.transactions[0];

    // Coinbase must have exactly one input with null prev_txid and prev_vout = MAX
    if coinbase.inputs.len() != 1 {
        return Err(ConsensusError::InvalidCoinbase(
            "Coinbase must have exactly one input".to_string(),
        ));
    }

    let coinbase_input = &coinbase.inputs[0];
    if coinbase_input.prev_txid != [0u8; 32] || coinbase_input.prev_vout != u32::MAX {
        return Err(ConsensusError::InvalidCoinbase(
            "Invalid coinbase input".to_string(),
        ));
    }

    // Coinbase scriptSig must be at least 2 bytes and at most 100 bytes
    if coinbase_input.script_sig.len() < 2 || coinbase_input.script_sig.len() > 100 {
        return Err(ConsensusError::InvalidCoinbase(
            "Invalid coinbase script length".to_string(),
        ));
    }

    // No other transactions should have coinbase-like inputs
    for tx in block.transactions.iter().skip(1) {
        for input in &tx.inputs {
            if input.prev_txid == [0u8; 32] && input.prev_vout == u32::MAX {
                return Err(ConsensusError::InvalidCoinbase(
                    "Non-coinbase transaction has coinbase input".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Validates transaction fees and block reward
fn validate_transaction_fees(
    block: &Block,
    height: u64,
    params: &ConsensusParams,
    total_fees: Option<u128>,
    _uncles_ctx: &[UncleContext],
) -> Result<(), ConsensusError> {
    let block_subsidy = params.reward_schedule.subsidy_at_height(height);
    // SECURITY: Use checked arithmetic to prevent integer overflow attacks.
    // An attacker could craft outputs that sum to > u64::MAX, causing wrap-around.
    let coinbase_output = block.transactions[0]
        .outputs
        .iter()
        .try_fold(0u128, |acc, o| acc.checked_add(o.value))
        .ok_or(ConsensusError::WeightOverflow("coinbase output sum"))?;

    // 🔴 CRITICAL: STRICT VALIDATION - NO BUFFER ALLOWED
    //
    // The "loose validation with fee buffer" was REMOVED because it allowed
    // miners to claim block_subsidy + 1 BTC WITHOUT any fees, creating a
    // permanent inflation bug (~6 BTC/hour at 10 min/block).
    //
    // Economic security requires EXACT fee calculation. We cannot accept
    // blocks with unknown fees - this would enable monetary inflation.
    //
    // CALLER MUST: Calculate fees from UTXO set before calling this function.
    // Use validate_block_with_fees() or calculate fees externally.
    //
    let fees = total_fees.ok_or_else(|| {
        ConsensusError::FeeValidation(
            "Total fees MUST be provided for coinbase validation. \
             Use validate_block_with_fees() or calculate from UTXO set. \
             Blocks with unknown fees CANNOT be accepted (inflation risk)."
                .to_string(),
        )
    })?;

    // Uncle blocks are deprecated; uncle rewards are zero

    // Treasury System: 10% of the block subsidy goes to the on-chain treasury
    let treasury_reward = block_subsidy / 10;
    let miner_subsidy = block_subsidy - treasury_reward;

    // Strict validation: Coinbase <= MinerSubsidy + Fees + TreasuryReward
    let max_miner_allowed = miner_subsidy
        .checked_add(fees)
        .ok_or(ConsensusError::WeightOverflow("block reward calculation"))?;

    let max_total_allowed = max_miner_allowed
        .checked_add(treasury_reward)
        .ok_or(ConsensusError::WeightOverflow("total block reward limit"))?;

    if coinbase_output > max_total_allowed {
        return Err(ConsensusError::CoinbaseExceedsSubsidy);
    }

    // Verify that the Treasury received its exact required 10% share
    if treasury_reward > 0 {
        let actual_treasury_reward: u128 = block.transactions[0]
            .outputs
            .iter()
            .filter(|o| o.script_pubkey == bitquan_types::genesis::TREASURY_PAYOUT_SCRIPT_BYTES)
            .map(|o| o.value)
            .sum();

        if actual_treasury_reward < treasury_reward {
            return Err(ConsensusError::FeeValidation(format!(
                "Treasury reward missing or insufficient: expected {}, found {}",
                treasury_reward, actual_treasury_reward
            )));
        }
    }

    // Coinbase should be at least block subsidy (Prevent burning/mistakes)
    // Note: Miners may voluntarily claim less than full subsidy (e.g., fee donation).
    // This is allowed per Bitcoin convention. The critical check is the ceiling above.
    if coinbase_output < block_subsidy {
        log::warn!(
            "⚠ Coinbase output {} is below block subsidy {} (miner forfeited {})",
            coinbase_output,
            block_subsidy,
            block_subsidy - coinbase_output
        );
    }

    Ok(())
}

/// Calculates merkle root from transactions.
///
/// # Security
/// Includes mitigation for CVE-2012-2459: rejects blocks where the last two
/// transactions are identical, which would allow an attacker to mutate the
/// transaction list while preserving the merkle root.
pub fn calculate_merkle_root(
    transactions: &[bitquan_types::Transaction],
) -> Result<[u8; 32], ConsensusError> {
    if transactions.is_empty() {
        return Ok([0u8; 32]);
    }

    // Calculate transaction hashes
    let mut hashes: Vec<[u8; 32]> = transactions.iter().map(hash_transaction).collect();

    // CVE-2012-2459: Reject blocks where the last two transactions are identical.
    // When the tx count is odd, Bitcoin duplicates the last hash for pairing.
    // An attacker can exploit this by submitting a block with the last tx
    // actually duplicated, producing the same merkle root as the honest block
    // but with a different (invalid) transaction set.
    if hashes.len() >= 2 && hashes[hashes.len() - 1] == hashes[hashes.len() - 2] {
        return Err(ConsensusError::MerkleRootMismatch);
    }

    // Build merkle tree
    while hashes.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in hashes.chunks(2) {
            if chunk.len() == 1 {
                // Duplicate odd hash
                next_level.push(hash_pair(chunk[0], chunk[0]));
            } else {
                next_level.push(hash_pair(chunk[0], chunk[1]));
            }
        }

        hashes = next_level;
    }

    Ok(hashes[0])
}

/// Hashes a transaction for merkle root calculation
fn hash_transaction(tx: &bitquan_types::Transaction) -> [u8; 32] {
    let mut hasher = Hasher::new();

    // Simple transaction serialization for merkle root
    hasher.update(&tx.version.to_le_bytes());
    hasher.update(&(tx.network as u8).to_le_bytes());
    hasher.update(&tx.genesis_hash);
    hasher.update(&tx.lock_time.to_le_bytes());

    // Hash inputs
    for input in &tx.inputs {
        hasher.update(&input.prev_txid);
        hasher.update(&input.prev_vout.to_le_bytes());
        hasher.update(&input.script_sig);
        hasher.update(&input.sequence.to_le_bytes());
    }

    // Hash outputs
    for output in &tx.outputs {
        hasher.update(&output.value.to_le_bytes());
        hasher.update(&output.script_pubkey);
    }

    // SECURITY (M-13): Hash witnesses to prevent malleability
    for witness in &tx.witnesses {
        for sig in &witness.signatures {
            hasher.update(&sig.signer_index.to_le_bytes());
            hasher.update(&sig.signature);
            hasher.update(&sig.public_key);
        }
    }

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(result.as_bytes());
    hash
}

/// Hashes two merkle tree nodes
fn hash_pair(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&a);
    hasher.update(&b);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(result.as_bytes());
    hash
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

    /// Validates a block using the stored registry and RNG state.
    ///
    /// SECURITY: Callers MUST supply the exact total transaction fees collected
    /// from the UTXO set for this block. Passing incorrect fees may allow a miner
    /// to claim inflated coinbase rewards. If fees are unavailable, use
    /// `validate_block_with_fees()` after computing them from the UTXO set.
    pub fn validate_block(
        &mut self,
        block: &Block,
        height: u64,
        total_fees: u128,
        median_time_past: u64,
        network_adjusted_time: u64,
        uncles_ctx: &[UncleContext],
        past_uncle_hashes: &std::collections::HashSet<[u8; 32]>,
    ) -> Result<BlockValidationReport, ConsensusError> {
        self.validate_block_with_fees(
            block,
            height,
            total_fees,
            median_time_past,
            network_adjusted_time,
            uncles_ctx,
            past_uncle_hashes,
        )
    }

    /// Validates a block with known total fees (for strict coinbase validation).
    ///
    /// SECURITY: This method also enforces the ASERT-computed difficulty target.
    /// If a `DifficultyState` anchor has been set via `set_difficulty_state()`,
    /// the block's `header.bits` must exactly match `peek_next_bits()` for this height.
    /// Ref: issue #187 (C1 — ASERT difficulty not enforced).
    #[allow(clippy::too_many_arguments)]
    pub fn validate_block_with_fees(
        &mut self,
        block: &Block,
        height: u64,
        total_fees: u128,
        median_time_past: u64,
        network_adjusted_time: u64,
        uncles_ctx: &[UncleContext],
        past_uncle_hashes: &std::collections::HashSet<[u8; 32]>,
    ) -> Result<BlockValidationReport, ConsensusError> {
        // Compute expected ASERT bits if difficulty anchor is available.
        // For genesis or contexts without an anchor, enforcement is skipped.
        let expected_bits = self
            .difficulty
            .as_ref()
            .map(|d| d.peek_next_bits(height, block.header.time as u64, &self.params));
        let expected_uncles_bits = self.difficulty.as_ref().map(|d| {
            uncles_ctx
                .iter()
                .map(|u| d.peek_next_bits(u.height, u.header.time as u64, &self.params))
                .collect::<Vec<u32>>()
        });
        validate_block(
            block,
            height,
            &self.params,
            &self.registry,
            self.network_id,
            self.genesis_hash,
            Some(total_fees),
            median_time_past,
            network_adjusted_time,
            expected_bits,
            expected_uncles_bits.as_deref(),
            uncles_ctx,
            past_uncle_hashes,
        )
    }
}

/// Standard dust threshold in qbits (satoshis).
pub const DUST_THRESHOLD_QBITS: u128 = 546;

/// Validates a single transaction against consensus rules (e.g. dust).
pub fn validate_transaction(tx: &bitquan_types::Transaction) -> Result<(), ConsensusError> {
    for (i, output) in tx.outputs.iter().enumerate() {
        // Check for dust outputs
        if output.value < DUST_THRESHOLD_QBITS {
            // Allow provably unspendable outputs (e.g. OP_RETURN) to be dust
            // OP_RETURN is 0x6a
            let is_op_return = !output.script_pubkey.is_empty() && output.script_pubkey[0] == 0x6a;

            if !is_op_return {
                return Err(ConsensusError::DustOutput {
                    index: i,
                    value: output.value,
                    threshold: DUST_THRESHOLD_QBITS,
                });
            }
        }
    }
    Ok(())
}
