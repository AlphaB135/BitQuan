//! Consensus rule scaffolding for BitQuan.
#![warn(missing_docs)]

use bitquan_types::{count_signatures, Block};
use bq_crypto::{CryptoError, CryptoRegistry};
use thiserror::Error;

/// Parameters controlling consensus validation.
#[derive(Clone, Debug)]
pub struct ConsensusParams {
    /// Maximum permitted block weight.
    pub block_weight_cap: u64,
    /// Weight multiplier applied per PQ signature.
    pub signature_weight_alpha: u32,
}

impl ConsensusParams {
    /// Returns the default Phase 3 configuration.
    pub fn phase3_defaults() -> Self {
        Self {
            block_weight_cap: 4_000_000,
            signature_weight_alpha: 384,
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
    params: &ConsensusParams,
    registry: &CryptoRegistry,
) -> Result<BlockValidationReport, ConsensusError> {
    let block_weight = calculate_block_weight(block, params.signature_weight_alpha);
    let signature_count = count_signatures(block);

    if block_weight > params.block_weight_cap {
        return Err(ConsensusError::BlockWeightExceeded {
            actual: block_weight,
            limit: params.block_weight_cap,
        });
    }

    // TODO: Replace placeholder digest handling with canonical transaction digest construction.
    for tx in &block.transactions {
        registry.verify_transaction(tx, &[])?;
    }

    Ok(BlockValidationReport {
        block_weight,
        signature_count,
    })
}
