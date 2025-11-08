# 🔐 Unwrap Elimination Plan - Priority 1

## 📊 Current Status
- **Before**: 430 unwraps
- **Current**: 343 unwraps (-87 ✅)
- **Target**: 50 unwraps
- **Remaining**: 293 unwraps to fix

## 🎯 Phase 1: Critical Security Files (137 unwraps)

### File 1: `crates/wallet/src/multisig.rs` (33 unwraps)
**Priority**: 🔴 CRITICAL (wallet = money)
**Strategy**:
- Replace HashMap unwraps with `get().ok_or(Error::KeyNotFound)?`
- Replace Vec unwraps with `get().ok_or(Error::IndexOutOfBounds)?`
- Add proper error types: `MultisigError::MissingSignature`, etc.

### File 2: `crates/node/src/mnemonic.rs` (32 unwraps)
**Priority**: 🔴 CRITICAL (private keys)
**Strategy**:
- BIP39 word list access: validate indices first
- Checksum validation: return Error instead of panic
- Add `MnemonicError::InvalidWord`, `InvalidChecksum`

### File 3: `crates/consensus/src/fork.rs` (27 unwraps)
**Priority**: 🔴 CRITICAL (consensus)
**Strategy**:
- Chain state unwraps → `ok_or(ConsensusError::InvalidChainState)?`
- Block index unwraps → proper error handling
- Add `ForkError::OrphanBlock`, `InvalidReorg`

### File 4: `crates/mempool/src/lib.rs` (24 unwraps)
**Priority**: 🟠 HIGH (DoS vector)
**Strategy**:
- Transaction validation unwraps → Result types
- Fee calculation unwraps → `checked_sub()`
- Add `MempoolError::TxNotFound`, `FeeCalculationFailed`

### File 5: `crates/consensus/src/sighash.rs` (21 unwraps)
**Priority**: 🔴 CRITICAL (signature validation)
**Strategy**:
- Byte array conversions → `try_into().map_err(...)?`
- Buffer slicing → bounds checking
- Add `SighashError::InvalidLength`, `BufferTooShort`

## ⏱️ Time Estimate
- **File 1-2**: 4 hours (wallet + mnemonic)
- **File 3**: 2 hours (fork logic)
- **File 4**: 2 hours (mempool)
- **File 5**: 2 hours (sighash)
- **Total**: ~10 hours (1-2 working days)

## 📋 Checklist
- [ ] Fix multisig.rs (33 → 0)
- [ ] Fix mnemonic.rs (32 → 0)
- [ ] Fix fork.rs (27 → 0)
- [ ] Fix lib.rs (24 → 0)
- [ ] Fix sighash.rs (21 → 0)
- [ ] Run `cargo test --all`
- [ ] Run `cargo clippy --all-targets -- -D warnings`
- [ ] Update `SECURITY_UNWRAP_ELIMINATION_PROGRESS.md`
- [ ] Commit: `fix(security): eliminate 137 unwraps in critical paths`

## 🎯 Success Criteria
- **Phase 1 Complete**: 343 → 206 unwraps (-137)
- **Security Score**: 65 → 75 (+10 points)
- **All tests passing**
- **Zero clippy warnings**

---
**Started**: $(date +%Y-%m-%d)
**Target Completion**: 2 days
