//! BQIP-0008: Same-Sender Input Aggregation
//!
//! BitQuan transactions use a single signature hash (SIGHASH_ALL) for the entire
//! transaction by default. If a transaction spends multiple inputs that are all
//! controlled by the exact same Dilithium5 public key, we only need a single
//! signature to authorize the entire transaction.
//!
//! This reduces the weight of multi-input transactions by collapsing redundant
//! signatures into a single witness payload, significantly increasing TPS.

use crate::{Transaction, Witness};
use std::collections::HashSet;

/// Evaluates if all signatures in a transaction come from the same public key.
/// If they do, it aggregates them by retaining only the first signature in a single witness,
/// discarding the redundant ones, and returns `true`.
/// Otherwise, returns `false` and leaves the transaction unchanged.
pub fn aggregate_same_sender(tx: &mut Transaction) -> bool {
    let mut unique_pubkeys = HashSet::new();
    let mut total_signatures = 0;

    for witness in &tx.witnesses {
        for sig in &witness.signatures {
            // HashSets require owned data or specific borrowing; cloning the pk is fine here
            // since this is run exactly once during tx finalization.
            unique_pubkeys.insert(sig.public_key.clone());
            total_signatures += 1;
        }
    }

    // Only aggregate if there are multiple signatures but they all share exactly ONE public key.
    if total_signatures > 1 && unique_pubkeys.len() == 1 {
        // Safe to unwrap since we know there's at least one signature
        let mut first_sig = None;
        'outer: for witness in &tx.witnesses {
            if let Some(sig) = witness.signatures.first() {
                first_sig = Some(sig.clone());
                break 'outer;
            }
        }

        if let Some(sig) = first_sig {
            // Collapse all witnesses into a single witness containing the single signature
            tx.witnesses = vec![Witness {
                signatures: vec![sig],
            }];
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SignaturePayload, Witness};

    fn make_dummy_sig(pk: u8, sig_val: u8) -> SignaturePayload {
        SignaturePayload {
            signer_index: 0,
            public_key: vec![pk; 32],
            signature: vec![sig_val; 64],
            aux: None,
        }
    }

    fn dummy_tx() -> Transaction {
        Transaction {
            version: 1,
            network: crate::NetworkId::Devnet,
            genesis_hash: [0u8; 32],
            lock_time: 0,
            inputs: vec![],
            outputs: vec![],
            sig_algo: crate::SigAlgorithm::Dilithium5,
            witnesses: vec![],
        }
    }

    #[test]
    fn test_aggregate_same_sender_success() {
        let mut tx = dummy_tx();

        // 3 inputs, all from pubkey 'A'
        tx.witnesses = vec![
            Witness {
                signatures: vec![make_dummy_sig(0xAA, 1)],
            },
            Witness {
                signatures: vec![make_dummy_sig(0xAA, 2), make_dummy_sig(0xAA, 3)],
            },
        ];

        let aggregated = aggregate_same_sender(&mut tx);

        assert!(aggregated);
        assert_eq!(tx.witnesses.len(), 1);
        assert_eq!(tx.witnesses[0].signatures.len(), 1);
        // Should keep the first signature found
        assert_eq!(tx.witnesses[0].signatures[0].signature[0], 1);
    }

    #[test]
    fn test_aggregate_different_senders_fails() {
        let mut tx = dummy_tx();

        // Inputs from pubkey 'A' and pubkey 'B'
        tx.witnesses = vec![
            Witness {
                signatures: vec![make_dummy_sig(0xAA, 1)],
            },
            Witness {
                signatures: vec![make_dummy_sig(0xBB, 2)],
            },
        ];

        let aggregated = aggregate_same_sender(&mut tx);

        assert!(!aggregated); // Should not aggregate
        assert_eq!(tx.witnesses.len(), 2); // Unchanged
    }

    #[test]
    fn test_aggregate_single_sig_ignored() {
        let mut tx = dummy_tx();

        // Only 1 input, no need to aggregate
        tx.witnesses = vec![Witness {
            signatures: vec![make_dummy_sig(0xAA, 1)],
        }];

        let aggregated = aggregate_same_sender(&mut tx);

        assert!(!aggregated); // Already optimized
    }
}
