//! Transaction memory pool scaffolding.
#![warn(missing_docs)]

use std::collections::BTreeMap;

use bitquan_consensus::{calculate_block_weight_with_beta, ConsensusParams};
use bitquan_types::Transaction;
use bq_crypto::rng::{RandomSource, RngError, RngService};
use thiserror::Error;

/// Represents the fundamental data for ordering transactions in the mempool.
#[derive(Clone, Debug)]
pub struct MempoolEntry {
    /// Transaction object retained in-memory.
    pub tx: Transaction,
    /// Calculated weight used for fee prioritisation.
    pub weight: u64,
    /// Fee per weight unit (sat/weight equivalent).
    pub fee_per_weight: u64,
    /// Random tie-breaker used when multiple transactions share the same fee density.
    pub tie_breaker: u64,
}

impl MempoolEntry {
    /// Calculates fee density from transaction totals.
    pub fn from_transaction(
        tx: Transaction,
        params: &ConsensusParams,
        fee: u64,
        tie_breaker: u64,
    ) -> Self {
        let block_context = crate::block_from_single_transaction(tx.clone(), tie_breaker);
        let weight = calculate_block_weight_with_beta(
            &block_context,
            params.signature_weight_alpha,
            params.witness_weight_beta,
        );
        let fee_per_weight = if weight == 0 { 0 } else { fee / weight };

        Self {
            tx,
            weight,
            fee_per_weight,
            tie_breaker,
        }
    }
}

/// Errors emitted by mempool operations.
#[derive(Debug, Error)]
pub enum MempoolError {
    /// Transaction already exists in the mempool.
    #[error("duplicate transaction detected")]
    Duplicate,
    /// Transaction failed preliminary validation checks.
    #[error("transaction rejected: {0}")]
    Rejected(String),
    /// RNG failure while generating tie-breaker values.
    #[error("rng failure: {0}")]
    Entropy(#[from] RngError),
}

/// Mempool storage keyed by (fee_per_weight, insertion order).
pub struct Mempool {
    params: ConsensusParams,
    entries: BTreeMap<u64, Vec<MempoolEntry>>,
    rng: RngService,
}

impl Mempool {
    /// Constructs a new mempool instance with the provided parameters.
    pub fn new(params: ConsensusParams) -> Result<Self, MempoolError> {
        let rng = RngService::new()?;
        Ok(Self {
            params,
            entries: BTreeMap::new(),
            rng,
        })
    }

    /// Returns the total number of transactions stored.
    pub fn len(&self) -> usize {
        self.entries.values().map(|v| v.len()).sum()
    }

    /// Inserts a transaction together with its absolute fee.
    pub fn insert(&mut self, tx: Transaction, fee: u64) -> Result<(), MempoolError> {
        let tie_breaker = self.rng.u64()?;
        let entry = MempoolEntry::from_transaction(tx, &self.params, fee, tie_breaker);
        let bucket = self
            .entries
            .entry(entry.fee_per_weight)
            .or_insert_with(Vec::new);
        bucket.push(entry);
        Ok(())
    }

    /// Drains up to `limit` transactions ordered by fee density.
    pub fn drain_high_priority(&mut self, limit: usize) -> Vec<MempoolEntry> {
        let mut collected = Vec::new();

        while collected.len() < limit {
            let next_key = match self.entries.iter().rev().next() {
                Some((key, _)) => *key,
                None => break,
            };

            if let Some(mut group) = self.entries.remove(&next_key) {
                group.sort_by(|a, b| a.tie_breaker.cmp(&b.tie_breaker));
                while let Some(entry) = group.pop() {
                    if collected.len() == limit {
                        self.entries.insert(next_key, group);
                        return collected;
                    }
                    collected.push(entry);
                }
            }
        }

        collected
    }
}

fn block_from_single_transaction(tx: Transaction, nonce: u64) -> bitquan_types::Block {
    bitquan_types::Block {
        header: bitquan_types::BlockHeader {
            version: 1,
            prev_block: [0u8; 32],
            merkle_root: [0u8; 32],
            pqc_agg_hint: [0u8; 32],
            time: 0,
            bits: 0,
            nonce,
        },
        transactions: vec![tx],
    }
}
