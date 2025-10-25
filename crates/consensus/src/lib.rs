//! Consensus rule scaffolding for BitQuan.
#![warn(missing_docs)]

use bitquan_types::{count_signatures, Block};
use bq_crypto::{
    rng::{RandomSource, RngError, RngService},
    CryptoError, CryptoRegistry,
};
use thiserror::Error;

mod asert;

pub use asert::asert_next_target;

/// Parameters controlling consensus validation.
#[derive(Clone, Debug)]
pub struct ConsensusParams {
    /// Maximum permitted block weight.
    pub block_weight_cap: u64,
    /// Weight multiplier applied per PQ signature.
    pub signature_weight_alpha: u32,
    /// Target block interval in seconds.
    pub target_block_time: u64,
    /// ASERT/LWMA half-life in seconds for difficulty retargeting.
    pub difficulty_half_life: u64,
    /// Block reward schedule parameters.
    pub reward_schedule: RewardSchedule,
}

impl ConsensusParams {
    /// Returns the default Phase 3 configuration.
    pub fn phase3_defaults() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
            target_block_time: 600,
            difficulty_half_life: 86_400,
            reward_schedule: RewardSchedule::phase3_defaults(),
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
        /// Observed weight for the block under evaluation.
        actual: u64,
        /// Configured upper bound for block weight.
        limit: u64,
    },
    /// Signature verification failed.
    #[error("signature verification failed: {0}")]
    Signature(#[from] CryptoError),
    /// RNG failure when preparing validation digests.
    #[error("rng failure: {0}")]
    Entropy(#[from] RngError),
}

/// Calculates the block weight given an `alpha` multiplier.
pub fn calculate_block_weight(block: &Block, alpha: u32) -> u64 {
    let raw_bytes = block.serialized_size_hint() as u64;
    let signature_weight = count_signatures(block) * alpha as u64;
    raw_bytes + signature_weight
}

/// Validates a block against the supplied consensus parameters.
pub fn validate_block(
    block: &Block,
    height: u64,
    params: &ConsensusParams,
    registry: &CryptoRegistry,
    rng: &mut RngService,
) -> Result<BlockValidationReport, ConsensusError> {
    let block_weight = calculate_block_weight(block, params.signature_weight_alpha);
    let signature_count = count_signatures(block);
    let block_subsidy = params.reward_schedule.subsidy_at_height(height);

    if block_weight > params.block_weight_cap {
        return Err(ConsensusError::BlockWeightExceeded {
            actual: block_weight,
            limit: params.block_weight_cap,
        });
    }

    // TODO: Replace placeholder digest handling with canonical transaction digest construction.
    for tx in &block.transactions {
        let digest = rng.bytes(32)?;
        registry.verify_transaction(tx, &digest)?;
    }

    Ok(BlockValidationReport {
        block_weight,
        signature_count,
        block_subsidy,
    })
}

/// Consensus engine bundling parameters, crypto registry, and RNG state.
pub struct ConsensusEngine {
    params: ConsensusParams,
    registry: CryptoRegistry,
    rng: RngService,
}

impl ConsensusEngine {
    /// Constructs a new engine using the supplied parameters and registry.
    pub fn new(params: ConsensusParams, registry: CryptoRegistry) -> Result<Self, ConsensusError> {
        let rng = RngService::new()?;
        Ok(Self {
            params,
            registry,
            rng,
        })
    }

    /// Provides mutable access to the underlying RNG for advanced scenarios.
    pub fn rng_mut(&mut self) -> &mut RngService {
        &mut self.rng
    }

    /// Returns the consensus parameters in use.
    pub fn params(&self) -> &ConsensusParams {
        &self.params
    }

    /// Computes the next target using ASERT relative to an anchor.
    pub fn next_difficulty_target(
        &self,
        anchor_target: u128,
        height_delta: i64,
        time_delta: i64,
    ) -> u128 {
        asert_next_target(anchor_target, height_delta, time_delta, &self.params)
    }

    /// Validates a block using the stored registry and RNG state.
    pub fn validate_block(
        &mut self,
        block: &Block,
        height: u64,
    ) -> Result<BlockValidationReport, ConsensusError> {
        validate_block(block, height, &self.params, &self.registry, &mut self.rng)
    }
}
