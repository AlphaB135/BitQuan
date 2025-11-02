# 🔴 Critical Security Tasks (H-K Priority P0)

## Overview
These tasks address the most critical security vulnerabilities that must be fixed before production use.

## Status Summary
- [x] Basic Auth Removal - **COMPLETE** ✅
- [ ] H) Cryptographic Timing Attacks - **IN PROGRESS** ⏳
- [ ] I) Integer Overflow Protection - **TODO** 
- [ ] J) Replay Attack Prevention - **TODO**
- [ ] K) Entropy Quality - **TODO**

═══════════════════════════════════════════════════════════════════

## H) Cryptographic Timing Attacks & Side Channels ⏳ IN PROGRESS

### Goal
Ensure all cryptographic comparisons use constant-time operations to prevent timing attacks.

### Critical Locations
1. **Signature Verification** (`crates/crypto/`)
   - Dilithium signature comparison
   - HMAC verification
   
2. **Password Comparison** (`crates/rpc/src/jwt/auth.rs`)
   - JWT password verification (uses Argon2 - should be OK)
   - Need to verify implementation

3. **MAC Verification** (`crates/wallet/`)
   - Wallet backup HMAC
   - Keystore integrity checks

### Implementation Plan
1. Add `subtle` crate dependency
2. Replace `==` with `ct_eq()` for all sensitive comparisons
3. Audit all crypto modules
4. Add tests for timing-attack resistance

### Files to Check
- [ ] `crates/crypto/src/signature.rs` (if exists)
- [ ] `crates/crypto/src/wallet/keystore.rs`
- [ ] `crates/wallet/src/backup.rs`
- [ ] `crates/rpc/src/jwt/auth.rs`

### Testing
- [ ] Add timing comparison tests
- [ ] Fuzzing for side-channel leaks
- [ ] Code review by security expert

### Timeline
- Audit: 1 day
- Implementation: 2 days
- Testing: 1 day
- **Total: 4 days**

═══════════════════════════════════════════════════════════════════

## I) Integer Overflow in Block Weight/Fee Calculation ❌ TODO

### Goal
Prevent integer overflows in critical arithmetic operations that could allow attackers to bypass limits.

### Critical Locations
```rust
// DANGEROUS - Current code uses .sum()
crates/consensus/src/lib.rs:170    - tx.witnesses.iter().map(|w| w.signatures.len()).sum()
crates/consensus/src/lib.rs:179    - block.transactions.iter().map(calculate_tx_weight).sum()
crates/types/src/transaction.rs:224 - self.witnesses.iter().map(|w| w.signatures.len()).sum()
crates/mempool/src/lib.rs:21       - tx.witnesses.iter().map(|w| w.signatures.len()).sum()
crates/node/src/tx_builder.rs:294  - selected.iter().map(|u| u.value).sum()
crates/node/src/rpc.rs:166         - tx.outputs.iter().map(|o| o.value).sum()
```

### Implementation Plan
1. Replace all `.sum()` with checked arithmetic:
   ```rust
   // ❌ Before
   let total: usize = items.iter().map(|x| x.weight).sum();
   
   // ✅ After
   let total = items.iter()
       .try_fold(0u64, |acc, x| {
           acc.checked_add(x.weight)
               .ok_or(Error::WeightOverflow)
       })?;
   ```

2. Add overflow tests:
   ```rust
   #[test]
   fn test_weight_overflow_rejected() {
       let tx = create_tx_with_max_inputs();
       assert!(validate_weight(tx).is_err());
   }
   ```

3. Define constants:
   ```rust
   const MAX_BLOCK_WEIGHT: u64 = 4_000_000;
   const MAX_TX_WEIGHT: u64 = 400_000;
   const MAX_BLOCK_SIGOPS: u64 = 80_000;
   ```

### Files to Modify
- [ ] `crates/consensus/src/lib.rs`
- [ ] `crates/types/src/transaction.rs`
- [ ] `crates/mempool/src/lib.rs`
- [ ] `crates/node/src/tx_builder.rs`
- [ ] `crates/node/src/rpc.rs`

### Testing
- [ ] Overflow tests for each calculation
- [ ] Fuzzing with extreme values
- [ ] Monster transaction tests

### Timeline
- Audit: 1 day
- Implementation: 3 days
- Testing: 2 days
- **Total: 6 days**

═══════════════════════════════════════════════════════════════════

## J) Replay Attack Prevention ❌ TODO

### Goal
Prevent transactions from being replayed across different networks or forks.

### Current Status
- ✅ `genesis_hash()` function exists in `crates/types/src/genesis.rs`
- ❌ No `network_magic` in Transaction struct
- ❌ Genesis hash NOT included in sighash

### Implementation Plan

1. **Add Network Magic**
   ```rust
   // crates/types/src/network.rs (NEW)
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct NetworkMagic([u8; 4]);
   
   impl NetworkMagic {
       pub const MAINNET: Self = Self([0xBF, 0x0C, 0x6B, 0xBD]);
       pub const TESTNET: Self = Self([0x0B, 0x11, 0x09, 0x07]);
       pub const DEVNET: Self = Self([0xFA, 0xBF, 0xB5, 0xDA]);
   }
   ```

2. **Update Transaction Structure**
   ```rust
   // crates/types/src/transaction.rs
   pub struct Transaction {
       pub version: u32,
       pub network_magic: NetworkMagic,  // NEW
       pub inputs: Vec<TxIn>,
       pub outputs: Vec<TxOut>,
       pub lock_time: u32,
   }
   ```

3. **Include in Sighash**
   ```rust
   // crates/consensus/src/sighash.rs
   fn compute_sighash(tx: &Transaction, genesis_hash: &[u8; 32]) -> [u8; 32] {
       let mut hasher = Sha256::new();
       hasher.update(&tx.network_magic.0);
       hasher.update(genesis_hash);
       // ... rest of transaction data
   }
   ```

### Files to Modify
- [ ] `crates/types/src/network.rs` (NEW)
- [ ] `crates/types/src/transaction.rs`
- [ ] `crates/consensus/src/sighash.rs`
- [ ] `crates/types/src/genesis.rs`

### Testing
- [ ] Test cross-network replay rejection
- [ ] Test cross-fork replay rejection
- [ ] Test mainnet/testnet isolation

### Timeline
- Design: 1 day
- Implementation: 3 days
- Testing: 2 days
- **Total: 6 days**

═══════════════════════════════════════════════════════════════════

## K) Dilithium Key Generation - Entropy Quality ✅ PARTIAL

### Current Status
- ✅ All random generation uses `OsRng` (cryptographically secure)
- ✅ BIP39 deterministic generation **COMPLETE**
- ❓ Need to verify entropy quality tests

### Verification Needed
1. **Entropy Source Quality**
   ```rust
   // Verify OsRng is used everywhere
   crates/crypto/src/wallet/kdf.rs:7    - use rand::rngs::OsRng
   crates/wallet/src/keystore.rs:7      - use rand::rngs::OsRng  
   crates/rpc/src/jwt/auth.rs:10        - use rand_core::OsRng
   ```

2. **Collision Tests**
   ```rust
   #[test]
   fn test_no_key_collisions() {
       let mut keys = HashSet::new();
       for _ in 0..10_000 {
           let key = generate_keypair();
           assert!(keys.insert(key), "Collision detected!");
       }
   }
   ```

3. **Deterministic Generation Tests**
   - BIP39 tests (12/12 passing) ✅
   - Verify no RNG in deterministic path ✅

### Implementation Plan
- [ ] Add entropy quality tests
- [ ] Add collision detection tests
- [ ] Verify no weak RNG usage
- [ ] Document entropy sources

### Files to Check
- [x] `crates/crypto/src/wallet/kdf.rs` ✅
- [x] `crates/wallet/src/keystore.rs` ✅
- [x] `crates/rpc/src/jwt/auth.rs` ✅
- [ ] Add more comprehensive tests

### Timeline
- Audit: 0.5 days (mostly done)
- Add tests: 1 day
- Documentation: 0.5 days
- **Total: 2 days**

═══════════════════════════════════════════════════════════════════

## Overall Timeline

| Task | Days | Priority |
|------|------|----------|
| H) Timing Attacks | 4 | P0 🔴 |
| I) Integer Overflow | 6 | P0 🔴 |
| J) Replay Prevention | 6 | P0 🔴 |
| K) Entropy Quality | 2 | P1 🟡 |
| **TOTAL** | **18 days** | |

## Risk Assessment

### Critical (Must Fix Before Launch)
- H) Timing Attacks - High exploitability
- I) Integer Overflow - Can bypass consensus rules
- J) Replay Attacks - Cross-network security

### High (Fix Soon)
- K) Entropy Quality - Already mostly good, needs verification

## Next Steps

1. **Week 1 (Nov 2-8):** Task H - Timing Attacks
2. **Week 2 (Nov 9-15):** Task I - Integer Overflow  
3. **Week 3 (Nov 16-22):** Task J - Replay Prevention
4. **Week 4 (Nov 23-29):** Task K - Entropy Testing + Buffer

## Success Criteria

All tasks complete when:
- ✅ Code changes committed
- ✅ Tests passing (>90% coverage)
- ✅ Documentation updated
- ✅ Code reviewed
- ✅ No new vulnerabilities introduced

═══════════════════════════════════════════════════════════════════
