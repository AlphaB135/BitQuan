# Attack Report #012: Transaction Malleability & Signature Mutation

**Date**: 2026-08-15 11:08:00 UTC  
**Attack Type**: Mempool / Transaction Malleability & Relay Hijacking  
**Severity**: High  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/types/src/transaction.rs`, `crates/crypto/src/dilithium.rs`

---

## 1. Attack Objective & Vector Description

The objective of transaction malleability is to alter the cryptographic signature or witness encoding of a valid unconfirmed transaction in the P2P mempool such that:
1. The modified signature remains mathematically valid.
2. The transaction hash (`txid` or `wtxid`) changes to a new value $TxID'$.
3. If $TxID'$ confirms on-chain instead of $TxID$, unconfirmed child transactions chained to $TxID$ are invalidated and merchant accounting systems are tricked into issuing double payouts.

---

## 2. Steps to Reproduce (PoC)

```rust
use bq_crypto::dilithium::{Keypair, verify};
use bitquan_types::{Transaction, Witness, SignaturePayload};

let keypair = Keypair::generate();
let message = b"Transaction Canonical Sighash";
let valid_sig = keypair.sign(message);

// Attempt Vector 1: Bit-flip mutation inside signature bytes
let mut mutated_sig = valid_sig;
mutated_sig[42] ^= 0xff;
assert!(verify(&mutated_sig, message, &keypair.public).is_err());

// Attempt Vector 2: Witness separation from TXID
let tx = Transaction { /* ... */ };
let txid_1 = tx.txid();

// Mutating witness signatures does NOT alter canonical base txid
let mut tx_malleated = tx.clone();
tx_malleated.witnesses = vec![Witness { /* alternative witness */ }];
let txid_2 = tx_malleated.txid();
assert_eq!(txid_1, txid_2, "TXID must be strictly immune to witness mutation (SegWit design)");
```

---

## 3. Observed Behavior & Red Team Findings

1. **Strict Signature Verification (Zero Non-Canonical Encoding)**:
   - Dilithium5 signature format is fixed-length (4,595 bytes) and deterministic.
   - Any bit mutation or polynomial trailing garbage fails lattice reconstruction with immediate rejection.
2. **Segregated Witness (SegWit) Architecture**:
   - The transaction ID (`txid`) is calculated exclusively over base transaction fields (`version`, `network`, `genesis_hash`, `inputs`, `outputs`, `lock_time`).
   - Witness data and post-quantum signatures are committed separately into the Merkle witness commitment root and `wtxid`.
   - Modifying witness payloads cannot alter the base `txid` referenced by child transactions.
3. **Replay & Cross-Chain Guard**:
   - Every transaction signature commits directly to `genesis_hash` and `network_id`, preventing transaction replay across testnet, devnet, and mainnet.

---

## 4. Impact Assessment

- **Availability**: Unaffected.
- **Integrity**: Maintained (Zero transaction malleability; deterministic TXIDs guaranteed).
- **Confidentiality**: N/A.

---

## 5. Defense Verification

- Automated test executed: `cargo test --test chaos_adversarial_suite -- test_chaos_scenario_5_signature_malleability_and_banning`
- Test Output:
  ```text
  ✅ Original Dilithium5 signature verified OK
  🛡️  Mutated Signature rejected by Dilithium5 verification
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
