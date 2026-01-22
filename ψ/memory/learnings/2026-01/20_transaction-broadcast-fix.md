# Transaction Broadcast Fix - Lesson Learned

**Date**: 2026-01-20
**Session Duration**: ~3 hours (06:34 - 09:28 GMT+7)
**Type**: Bug Fix + Feature Integration
**Impact**: High - Transaction broadcast system incomplete

## Problem Statement

Transaction broadcast system ถูก implement ใน `commands/mining.rs` แต่ `main.rs` mining loop ไม่ได้ใช้งาน ทำให้ wallet-send บันทึก transactions ลง `pending_transactions.jsonl` แต่ไม่เคยถูกรวมใน blocks

## Root Causes Identified

### 1. Duplicate Mining Implementations
- `main.rs:1701+` - Has its own mining loop WITHOUT pending transaction support
- `commands/mining.rs:717+` - Has full pending transaction support
- **Impact**: Feature complete architecturally but non-functional in practice

### 2. Database Path Hardcode
- `wallet-send` ใช้ hardcoded `data/chainstate`
- Mining ใช้ `data/devnet`
- **Fix**: Added `--datadir` CLI parameter

### 3. Script Format Mismatch
- Mining used P2PKH: `76a914{20-byte}88ac`
- Wallet generated OP_HASH256: `a820{32-byte}87`
- **Fix**: Reset blockchain with correct format

### 4. u128 JSON Overflow
- `u128` values (50 BQ = 5×10^19 qbits) exceed f64 precision (2^53)
- **Fix**: Serialize transaction → JSON string → embed in outer JSON
- **Dependency**: Added `serde_json "arbitrary_precision"` feature

## Lessons Learned

### Pattern: Code Duplication Detection
ก่อน implement feature ใหม่ ต้อง search codebase ว่ามี implementation ที่ similar อยู่แล้วหรือไม่

```bash
# Should have run this FIRST:
rg "fn.*mining\|pending_transaction" --type rust
```

### Pattern: u128 JSON Serialization
Blockchain amounts เป็น `u128` (up to 2^127) แต่ JSON number precision จำกัดที่ 2^53

**Solution**:
```rust
// Wrong (causes panic):
let json = serde_json::to_value(&transaction)?;

// Correct:
let tx_str = serde_json::to_string(&transaction)?;
let outer = serde_json::json!({ "tx": tx_str });
```

### Pattern: Architecture Discovery Order
1. **Search existing code** → มี pending transaction support แล้วหรือยัง?
2. **Trace call paths** → main.rs เรียก function ไหนจาก commands?
3. **THEN implement** → เชื่อมหรือ extend อย่างเดียว

**Anti-pattern**: Implement → Test → Discover duplicate → Refactor

### Discovery: Keypair Caching at Startup
`main.rs` mining loop generates keypair ตอน start และ cache ไว้ ทำให้:
- เปลี่ยน wallet file ไม่มีผลจนกว่าจะ restart
- ต้อง mine blockchain ใหม่ 3 ครั้งเพื่อ test payout script ใหม่

**Should add**: Documentation comment `// Keypair generated at startup, restart to update`

## Prevention Checklist

- [ ] Search codebase สำหรับ duplicate implementations ก่อน coding
- [ ] Trace call paths จาก main.rs → modules เพื่อ verify integration
- [ ] Test incremental (wallet → mining → end-to-end) อย่าง step-by-step
- [ ] Verify data types match serialization formats (u128 → JSON string)

## Technical Debt Created

1. **Duplicate mining logic** - `main.rs` vs `commands/mining.rs`
   - **Action**: Refactor to shared module
   - **Priority**: High - causes feature gaps

2. **Debug println! statements** - Added at mining.rs:103, 855-857
   - **Action**: Remove or replace with proper tracing
   - **Priority**: Low - cosmetic

## Next Steps

1. **CRITICAL**: Add `load_pending_transactions()` call to main.rs mining loop
2. **Test end-to-end**: wallet-send → mining → UTXO verification
3. **Refactor**: Consolidate mining implementations to single source
4. **Clean up**: Remove debug statements
5. **Document**: Add comments about keypair caching behavior

## Related Files

- `/crates/node/src/main.rs:1701` - Mining loop WITHOUT pending tx support
- `/crates/node/src/commands/mining.rs:717` - Mining loop WITH pending tx support
- `/crates/node/src/commands/wallet.rs` - Transaction serialization fix
- `/Cargo.toml` - serde_json arbitrary_precision feature

## Tags

`transaction-broadcast` `mining` `u128-serialization` `code-duplication` `debugging`
