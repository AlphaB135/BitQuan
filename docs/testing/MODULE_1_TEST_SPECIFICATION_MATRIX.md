# BitQuan Layer-1 Blockchain — Comprehensive Test Specification Matrix

**Document Version:** 1.0.0  
**Date:** 2026-08-14  
**Author:** Principal L1 Blockchain Architect & Head of Core Engineering  
**Status:** Pre-Testnet Phase 1 — Quality Assurance Framework  

---

## Executive Summary

This document provides an exhaustive, structured test specification matrix detailing every critical test required before launching BitQuan Public Testnet Phase 1. Each test case includes precise execution commands, adversarial threat models, expected results, and failure thresholds calibrated to institutional-grade blockchain security standards.

**Coverage Areas:**
1. Consensus & Fork Choice (ASERT, Reorg, Uncle/GHOST)
2. Post-Quantum Cryptographic Assurance (Dilithium5)
3. Mempool & Anti-Spam Stress Testing (BQIP-0002)
4. P2P Networking & Eclipse/Sybil Resistance
5. RPC & API Security (JWT, Rate Limiting)
6. Storage Integrity (RocksDB Recovery)

**Testing Philosophy:** Every test is adversarial-first — we assume malicious actors will probe every attack surface identified during the 2026 internal security audit (C1-C7 vulnerabilities).

---

## 1. CONSENSUS & FORK CHOICE TESTING

### Test Suite: CON — Consensus Rule Enforcement

#### Test Case CON-001: ASERT Difficulty Adjustment Under Hashpower Surge

**Subsystem:** `crates/consensus/src/asert.rs`  
**Objective:** Verify ASERT (Absolutely Scheduled Exponentially Rising Targets) correctly adjusts difficulty when network hashpower increases by 500% over 10 blocks.

**Adversarial Threat Model:**  
- **Attack Vector:** Malicious mining pool acquires 5x normal hashrate, attempts to mine blocks at 12s intervals (target: 120s) to trigger chain instability.
- **Economic Impact:** Rapid block production → premature halvings → inflation schedule disruption.

**Prerequisites:**
- Clean devnet environment (genesis block only)
- Difficulty anchor: `height=0, bits=0x207fffff, timestamp=1700000000`
- ASERT parameters: `target_block_time=120s, half_life=14400s`

**Execution Commands:**
```bash
# 1. Initialize test chain
cd /home/ubuntu/bitquan-audit
cargo build --release --bin bitquan-node

# 2. Start node with devnet config
./target/release/bitquan-node run \
  --config config/devnet.toml \
  --datadir /tmp/asert-test-1 \
  --network devnet &
NODE_PID=$!

# 3. Mine 10 blocks with 12s intervals (5x faster than target 120s)
for i in {1..10}; do
  ./target/release/bitquan-node mine \
    --pow mock \
    --count 1 \
    --interval 12 \
    --datadir /tmp/asert-test-1
  sleep 12
done

# 4. Query difficulty progression via RPC
curl -s http://localhost:19443/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}' \
  | jq '.result.difficulty'

# 5. Verify ASERT formula enforcement
./target/release/bitquan-node debug asert \
  --anchor-height 0 \
  --anchor-bits 0x207fffff \
  --anchor-time 1700000000 \
  --next-height 10 \
  --next-time $((1700000000 + 120)) \
  --half-life 14400

# 6. Cleanup
kill $NODE_PID
rm -rf /tmp/asert-test-1
```

**Expected Result:**
- Block 10 difficulty `bits` must be **lower** than genesis `0x207fffff` (higher difficulty).
- ASERT formula validation:
  ```
  exponent = (actual_time - expected_time) / half_life
  actual_time = 10 * 12s = 120s
  expected_time = 10 * 120s = 1200s
  exponent = (120 - 1200) / 14400 = -1080 / 14400 = -0.075

  new_target = anchor_target * 2^(-0.075)
  new_target < anchor_target  (difficulty increased)
  ```

**Assertion Criteria:**
- `block_10.header.bits < 0x207fffff` (compact target decreased)
- Difficulty progression monotonically increases for blocks 1-10
- No overflow/underflow in fixed-point arithmetic (32.32 format)

**Failure Threshold:** **CRITICAL**  
If ASERT fails to adjust difficulty, the chain is vulnerable to:**  
- 51% attack with sustained hashpower injection
- Block reward manipulation via rapid mining
- Testnet becomes unusable for stress testing

**Reference:** `crates/consensus/src/asert.rs:200-250`, `FP_SCALE = 2^32`

---

#### Test Case CON-002: ASERT Difficulty Adjustment Under Hashpower Collapse

**Subsystem:** `crates/consensus/src/asert.rs`  
**Objective:** Verify ASERT correctly **decreases** difficulty when 90% of hashpower vanishes, preventing chain stall.

**Adversarial Threat Model:**  
- **Attack Vector:** Coordinated miner exodus (e.g., mining pool shutdown, regulatory crackdown).
- **Impact:** Chain stalls if difficulty remains high — no blocks mined for hours/days.

**Prerequisites:**
- Chain at height 100, stable 120s block time
- Simulated hashpower drop: blocks 101-110 mined at 1200s intervals (10x slower)

**Execution Commands:**
```bash
# 1. Setup: Pre-mine 100 blocks at target 120s
./scripts/test-helpers/mine-chain.sh \
  --blocks 100 \
  --interval 120 \
  --output /tmp/asert-collapse

# 2. Simulate hashpower collapse: mine 10 blocks at 1200s intervals
for i in {101..110}; do
  ./target/release/bitquan-node mine \
    --pow hashcash \
    --difficulty-override $((0x207fffff + 0x00100000)) \
    --datadir /tmp/asert-collapse
  sleep 1200  # 20 minutes per block
done

# 3. Verify difficulty decreased
DIFF_100=$(./scripts/get-block-difficulty.sh 100 /tmp/asert-collapse)
DIFF_110=$(./scripts/get-block-difficulty.sh 110 /tmp/asert-collapse)

if [ "$DIFF_110" -gt "$DIFF_110" ]; then
  echo "✅ PASS: Difficulty decreased from $DIFF_100 to $DIFF_110"
else
  echo "❌ FAIL: Difficulty did not adjust downward"
fi
```

**Expected Result:**
- Block 110 `bits` > Block 100 `bits` (target increased → easier mining)
- Chain does NOT stall (block 111 mined within 10 minutes)

**Assertion Criteria:**
- `target_110 / target_100 >= 2.0` (difficulty halved minimum)
- No integer overflow in ASERT exponent calculation
- Burst guard does NOT trigger (collapse ≠ burst)

**Failure Threshold:** **CRITICAL**  
Chain stall = testnet death. Must self-heal within 2x target block time.

---

#### Test Case CON-003: Deep Chain Reorg (50 Blocks)

**Subsystem:** `crates/consensus/src/fork.rs`  
**Objective:** Verify fork choice engine correctly handles a 50-block reorganization when a competing chain overtakes the main chain.

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker mines a secret chain with higher cumulative work, broadcasts at height H+50 to trigger reorg.
- **Impact:** Double-spend attack if reorg succeeds without proper UTXO rollback.

**Prerequisites:**
- Main chain at height 100 (public)
- Attacker chain forked at height 50, mined in secret to height 101 with higher difficulty

**Execution Commands:**
```bash
# 1. Setup main chain (100 blocks, normal difficulty)
./scripts/test-cluster.sh start --nodes 3 --network testnet
sleep 30  # Wait for sync

# 2. Fork at block 50: create isolated attacker node
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/attacker-node \
  --p2p-bind 127.0.0.1:29444 \
  --isolated &  # No peers = secret chain
ATTACKER_PID=$!

# 3. Mine attacker chain from block 50 to 101 (higher work via lower bits)
./target/release/bitquan-node mine \
  --datadir /tmp/attacker-node \
  --start-height 50 \
  --count 51 \
  --difficulty-mul 0.9  # 10% higher difficulty → more work

# 4. Broadcast attacker chain to main network (trigger reorg)
./target/release/bitquan-node connect \
  --datadir /tmp/attacker-node \
  --peer 127.0.0.1:19444  # Connect to seed node

# 5. Monitor reorg via logs
tail -f /tmp/bitquan-testnet/node-seed/logs/node.log | grep "REORG"

# 6. Verify fork choice switched to attacker chain
MAIN_TIP=$(curl -s http://localhost:19443/rpc -d '{"method":"getbestblockhash"}' | jq -r '.result')
ATTACKER_TIP=$(./scripts/get-block-hash.sh 101 /tmp/attacker-node)

if [ "$MAIN_TIP" == "$ATTACKER_TIP" ]; then
  echo "✅ PASS: Reorg successful, fork choice selected higher-work chain"
else
  echo "❌ FAIL: Fork choice did not switch chains"
fi
```

**Expected Result:**
- Fork choice engine detects competing chain at height 101 with higher cumulative work
- Reorg executed: disconnect blocks 51-100 (main), connect blocks 51-101 (attacker)
- UTXO set rolled back correctly (no ghost coins)
- All 3 nodes converge on attacker chain within 60 seconds

**Assertion Criteria:**
- `ForkChoice::last_reorg_depth == 50`
- No `ForkError::ReorgTooDeep` (50 < MAX_REORG_DEPTH=100)
- RocksDB UTXO index matches block 101 state exactly
- Mempool purged of transactions spending UTXOs from disconnected blocks

**Failure Threshold:** **HIGH**  
Reorg failures enable double-spend attacks and chain split scenarios.

**Reference:** `crates/consensus/src/fork.rs:140`, `DEFAULT_MAX_REORG = 100`

---

#### Test Case CON-004: Reorg Exceeding Depth Cap (101 Blocks) — MUST REJECT

**Subsystem:** `crates/consensus/src/fork.rs`  
**Objective:** Verify that reorganizations deeper than `MAX_REORG_DEPTH=100` are **rejected** to prevent 51% deep-reorg attacks.

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker with sustained 51% hashpower mines a 101-block secret chain, attempts to rewrite 101 blocks of history.
- **Defense:** Reject reorg, treat as invalid chain to preserve finality guarantees.

**Prerequisites:**
- Main chain at height 200
- Attacker chain forked at height 99, mined to height 200 (101 blocks deep)

**Execution Commands:**
```bash
# 1. Setup: main chain at height 200
./scripts/mine-chain.sh --blocks 200 --output /tmp/reorg-cap-test

# 2. Create attacker chain forked at height 99
cp -r /tmp/reorg-cap-test /tmp/attacker-deep-reorg
cd /tmp/attacker-deep-reorg
./target/release/bitquan-node rollback --to-height 99

# 3. Mine attacker chain to height 200 (101 new blocks)
./target/release/bitquan-node mine \
  --datadir /tmp/attacker-deep-reorg \
  --count 101 \
  --difficulty-mul 0.95  # Slightly higher work

# 4. Attempt to broadcast to main network
./target/release/bitquan-node connect \
  --datadir /tmp/attacker-deep-reorg \
  --peer 127.0.0.1:19444 &

# 5. Monitor rejection
tail -f /tmp/reorg-cap-test/node.log | grep "ReorgTooDeep"

# Expected log output:
# [ERROR] Fork choice: ReorgTooDeep(101, 100) - rejecting competing chain
```

**Expected Result:**
- Node **rejects** attacker chain with `ForkError::ReorgTooDeep(101, 100)`
- Main chain tip remains at original height 200 block
- Attacker peer **banned** for 24 hours (malicious behavior)

**Assertion Criteria:**
```rust
// From crates/consensus/src/fork.rs
assert_eq!(fork_choice.last_reorg_depth, 0);  // No reorg occurred
assert!(ban_manager.is_banned(&attacker_peer_id));
```

**Failure Threshold:** **CRITICAL**  
If deep reorgs are allowed, attackers can rewrite weeks of history, destroying chain integrity.

---

#### Test Case CON-005: Uncle Block Rejection (Deprecated Feature)

**Subsystem:** `crates/consensus/src/lib.rs:607-611`  
**Objective:** Verify that blocks containing uncle/ommer blocks are **rejected** per BitQuan consensus rules.

**Adversarial Threat Model:**  
- **Attack Vector:** Miner includes uncle blocks to claim extra rewards (Ethereum GHOST-style).
- **BitQuan Policy:** Uncle blocks are **deprecated** for 120s block time chains (unnecessary complexity).

**Prerequisites:**
- Node running with consensus engine initialized

**Execution Commands:**
```bash
# 1. Create block with uncle
cat > /tmp/block-with-uncle.json <<EOF
{
  "header": { "version": 1, "prev_block": "0x00...00", "bits": 0x207fffff, ... },
  "transactions": [ ... ],
  "uncles": [
    { "version": 1, "prev_block": "0x00...01", "bits": 0x207fffff, ... }
  ]
}
EOF

# 2. Submit via RPC
RESPONSE=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"submitblock\",\"params\":[$(cat /tmp/block-with-uncle.json)]}")

echo "$RESPONSE" | jq '.error.message'
# Expected: "Uncle blocks are deprecated and not supported"
```

**Expected Result:**
- Block validation fails with `ConsensusError::InvalidUncle`
- Error message: `"Uncle blocks are deprecated and not supported"`
- Block rejected before signature verification (fail-fast)

**Assertion Criteria:**
```rust
// From crates/consensus/src/lib.rs:607
if !block.uncles.is_empty() {
    return Err(ConsensusError::InvalidUncle(...));
}
```

**Failure Threshold:** **MEDIUM**  
Uncle block acceptance would break reward economics and enable unfair miner advantages.

---

## 2. POST-QUANTUM CRYPTOGRAPHIC ASSURANCE

### Test Suite: PQC — Dilithium5 Signature Verification

#### Test Case PQC-001: Dilithium5 Signature Malleability Attack

**Subsystem:** `crates/crypto/src/lib.rs`, `crates/pqc-dilithium-seeded/`  
**Objective:** Verify that modified Dilithium5 signatures are rejected (no malleability).

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker intercepts valid transaction, flips bits in signature, rebroadcasts.
- **Expected Defense:** Signature verification fails, transaction rejected from mempool.

**Prerequisites:**
- Valid signed transaction with Dilithium5 signature (4,595 bytes)

**Execution Commands:**
```bash
# 1. Create valid transaction
TX_HEX=$(./target/release/bitquan-cli create-tx \
  --from bq1q... \
  --to bq1q... \
  --amount 10.0 \
  --sign wallet.keystore)

# 2. Extract signature bytes (offset 100-4695)
SIG_ORIGINAL=$(echo "$TX_HEX" | xxd -p -l 4595 -s 100)

# 3. Flip one bit in signature (byte 1000, bit 3)
SIG_MODIFIED=$(echo "$SIG_ORIGINAL" | sed 's/\(.\{2000\}\)./\1X/')

# 4. Reconstruct transaction with modified signature
TX_MALLEATED=$(echo "$TX_HEX" | sed "s/$SIG_ORIGINAL/$SIG_MODIFIED/")

# 5. Submit to mempool via RPC
RESPONSE=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$TX_MALLEATED\"]}")

echo "$RESPONSE" | jq '.error.message'
# Expected: "signature verification failed"
```

**Expected Result:**
- Modified transaction **rejected** by mempool with `CryptoError::Malformed("signature verification failed")`
- Original transaction still valid, can be submitted successfully

**Assertion Criteria:**
```rust
// From crates/crypto/src/lib.rs:111-137
assert!(dilithium::crypto_sign_verify(&sig_bytes, message, &pk_bytes).is_err());
```

**Failure Threshold:** **CRITICAL**  
Signature malleability → transaction ID manipulation → double-spend attacks.

**Reference:** NIST FIPS 205 — CRYSTALS-Dilithium specification §4.2

---

#### Test Case PQC-002: Zero-Length Signature Rejection

**Subsystem:** `crates/crypto/src/lib.rs:116`  
**Objective:** Verify that transactions with empty or truncated signatures are rejected **before** cryptographic verification (fail-fast).

**Execution Commands:**
```bash
# Create transaction with empty signature field
cat > /tmp/tx-empty-sig.json <<EOF
{
  "version": 2,
  "network": "devnet",
  "inputs": [...],
  "outputs": [...],
  "witnesses": [{
    "signatures": [{
      "signer_index": 0,
      "signature": "",  // EMPTY
      "public_key": "0x..."
    }]
  }]
}
EOF

# Submit
RESPONSE=$(./target/release/bitquan-cli submit-tx /tmp/tx-empty-sig.json)
echo "$RESPONSE" | grep "invalid signature length"
```

**Expected Result:**
- Immediate rejection with `CryptoError::Malformed("invalid signature length")`
- CPU not wasted on Dilithium5 verification (4,595-byte empty signature check fails first)

**Assertion Criteria:**
```rust
if payload.signature.len() != SIGNBYTES {  // SIGNBYTES = 4595
    return Err(CryptoError::Malformed("invalid signature length"));
}
```

**Failure Threshold:** **MEDIUM** (DoS vector if not caught early)

---

#### Test Case PQC-003: Cross-Network Replay Protection

**Subsystem:** `crates/types/src/transaction.rs`, `crates/consensus/src/sighash.rs`  
**Objective:** Verify that a transaction signed for Testnet cannot be replayed on Mainnet (domain separation).

**Adversarial Threat Model:**  
- **Attack Vector:** User sends 1000 BQ on Testnet → Attacker replays exact transaction on Mainnet after launch.
- **Defense:** Network ID and genesis hash included in sighash → signature invalid on different network.

**Execution Commands:**
```bash
# 1. Create transaction on Testnet
TX_TESTNET=$(./target/release/bitquan-cli create-tx \
  --network testnet \
  --from bq1q... \
  --to bq1q... \
  --amount 1000.0 \
  --keystore wallet.keystore)

# 2. Extract raw transaction bytes
TX_RAW=$(echo "$TX_TESTNET" | jq -r '.hex')

# 3. Submit to Mainnet RPC
curl -s http://mainnet-node:8443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$TX_RAW\"]}" \
  | jq '.error.message'

# Expected: "signature verification failed" (sighash mismatch)
```

**Expected Result:**
- Transaction rejected on Mainnet with signature verification failure
- Sighash computation includes:
  ```rust
  TxContext {
      network_id: NetworkId::Testnet,      // Different on mainnet
      genesis_hash: TESTNET_GENESIS_HASH,  // Different on mainnet
  }
  ```

**Assertion Criteria:**
```rust
// From crates/consensus/src/sighash.rs
let ctx_testnet = TxContext::new(NetworkId::Testnet, TESTNET_GENESIS);
let ctx_mainnet = TxContext::new(NetworkId::Mainnet, MAINNET_GENESIS);
assert_ne!(
    transaction_sighash(&tx, &ctx_testnet),
    transaction_sighash(&tx, &ctx_mainnet)
);
```

**Failure Threshold:** **CRITICAL**  
Cross-network replay → users lose real funds via testnet transaction replay.

**Reference:** `crates/types/src/context.rs`, BQIP-0001 Domain Separation

---

#### Test Case PQC-004: Dilithium5 Key Derivation Entropy Validation

**Subsystem:** `crates/crypto/src/wallet.rs`, `crates/crypto/src/rng.rs`  
**Objective:** Verify that wallet key generation uses cryptographically secure entropy (no weak RNG).

**Threat Model:**  
- **Attack Vector:** Weak RNG → predictable private keys → attacker pre-computes key space, steals funds.

**Execution Commands:**
```bash
# 1. Generate 1000 wallets, extract public keys
for i in {1..1000}; do
  ./target/release/bitquan-node wallet-gen \
    --output /tmp/wallet-$i.keystore \
    --password "test$i"
  
  PK=$(./target/release/bitquan-node wallet-pubkey \
    --keystore /tmp/wallet-$i.keystore \
    --password "test$i")
  
  echo "$PK" >> /tmp/pubkeys.txt
done

# 2. Statistical entropy test (NIST SP 800-22)
./scripts/entropy-test.py /tmp/pubkeys.txt

# Expected output:
# ✅ Frequency test: PASS (p-value > 0.01)
# ✅ Runs test: PASS (p-value > 0.01)
# ✅ Longest run: PASS (no patterns detected)
```

**Expected Result:**
- All 1000 public keys unique (no collisions)
- NIST SP 800-22 randomness tests pass with p-value > 0.01
- Entropy source: `/dev/urandom` (Linux) or `CryptGenRandom` (Windows)

**Assertion Criteria:**
```rust
// From crates/crypto/src/rng.rs
let mut seed = [0u8; 32];
getrandom::getrandom(&mut seed).expect("RNG failure");
assert!(seed.iter().any(|&b| b != 0));  // Not all zeros
```

**Failure Threshold:** **CRITICAL**  
Weak entropy = predictable keys = total loss of funds.

**Reference:** NIST SP 800-90A — Recommendation for Random Number Generation

---

## 3. MEMPOOL & ANTI-SPAM STRESS TESTING

### Test Suite: MEM — Transaction Pool Management

#### Test Case MEM-001: 10,000 TPS Transaction Flood (DoS Resilience)

**Subsystem:** `crates/mempool/src/lib.rs`  
**Objective:** Verify mempool remains bounded at 300 MB under sustained 10,000 tx/sec flood.

**Adversarial Threat Model:**  
- **Attack Vector:** Botnet floods node with 10,000 valid transactions per second.
- **Defense:** BQIP-0002 fee-density eviction policy activates, low-fee transactions evicted.

**Prerequisites:**
- Node running with default mempool config (300 MB max)
- Test wallet with 10,000 UTXOs pre-funded

**Execution Commands:**
```bash
# 1. Start node with mempool monitoring
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --enable-mempool-metrics &
NODE_PID=$!

# 2. Launch transaction flood (10k tx/sec for 60 seconds)
./scripts/stress/tx-flood.py \
  --rpc http://localhost:19443 \
  --rate 10000 \
  --duration 60 \
  --wallet /tmp/stress-wallet.keystore \
  --fee-range 1-100  # qbits/WU

# 3. Monitor mempool size
watch -n 1 "curl -s http://localhost:19443/metrics | grep mempool_size_bytes"

# 4. Verify bounded memory
MAX_SIZE=$(curl -s http://localhost:19443/metrics \
  | grep mempool_size_bytes \
  | awk '{print $2}' \
  | sort -n | tail -1)

if [ "$MAX_SIZE" -lt 315000000 ]; then  # 300MB + 5% tolerance
  echo "✅ PASS: Mempool bounded at $MAX_SIZE bytes"
else
  echo "❌ FAIL: Mempool exceeded 300MB limit"
fi

kill $NODE_PID
```

**Expected Result:**
- Mempool size **never exceeds** 315 MB (300 MB + 5% measurement tolerance)
- Low-fee transactions evicted automatically when limit approached
- Node remains responsive (RPC latency < 100ms p99)

**Assertion Criteria:**
```rust
// From crates/mempool/src/lib.rs:79
const DEFAULT_MAX_SIZE: usize = 300_000_000;  // 300 MB
assert!(mempool.size_bytes() <= DEFAULT_MAX_SIZE);
```

**Failure Threshold:** **HIGH**  
Unbounded mempool → OOM crash → node downtime → network partition.

**Reference:** BQIP-0002 — Mempool Fee Density Eviction Policy

---

#### Test Case MEM-002: Fee-Density Eviction (Low-Fee Purge)

**Subsystem:** `crates/mempool/src/lib.rs:295-320`  
**Objective:** Verify that when mempool reaches capacity, lowest fee-density transactions are evicted first.

**Execution Commands:**
```bash
# 1. Fill mempool to 290 MB (near capacity)
./scripts/stress/fill-mempool.sh --target-size 290000000

# 2. Submit high-fee transaction (50 qbits/WU)
HIGH_FEE_TX=$(./target/release/bitquan-cli create-tx \
  --from bq1q... \
  --to bq1q... \
  --amount 1.0 \
  --fee-rate 50)

TXID_HIGH=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$HIGH_FEE_TX\"]}" \
  | jq -r '.result')

# 3. Verify high-fee tx accepted, low-fee tx evicted
sleep 2

IN_MEMPOOL=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"getrawmempool\",\"params\":[]}" \
  | jq -r ".result[] | select(. == \"$TXID_HIGH\")")

if [ -n "$IN_MEMPOOL" ]; then
  echo "✅ PASS: High-fee transaction accepted"
else
  echo "❌ FAIL: High-fee transaction rejected"
fi
```

**Expected Result:**
- High-fee transaction (50 qbits/WU) accepted into full mempool
- Lowest fee-density transactions evicted to make room
- Protected transactions (≥10 qbits/WU) **never** evicted

**Assertion Criteria:**
```rust
// From crates/mempool/src/lib.rs:89
const PROTECTED_FEE_RATE: u64 = 10;
// Eviction only touches transactions with fee_per_weight < new_fee_rate
// AND fee_per_weight < PROTECTED_FEE_RATE
```

**Failure Threshold:** **MEDIUM**  
Incorrect eviction → high-fee transactions stuck, network congestion.

---

#### Test Case MEM-003: Double-Spend Prevention (Same Input Rejection)

**Subsystem:** `crates/mempool/src/lib.rs:258-274`  
**Objective:** Verify that mempool rejects transactions spending the same UTXO (double-spend prevention).

**Execution Commands:**
```bash
# 1. Create two transactions spending same input
UTXO_TXID="0x1234..."
UTXO_VOUT=0

TX1=$(./target/release/bitquan-cli create-tx \
  --input "$UTXO_TXID:$UTXO_VOUT" \
  --to bq1qalice... \
  --amount 5.0 \
  --sign wallet.keystore)

TX2=$(./target/release/bitquan-cli create-tx \
  --input "$UTXO_TXID:$UTXO_VOUT" \
  --to bq1qbob... \
  --amount 5.0 \
  --sign wallet.keystore)

# 2. Submit TX1 (should succeed)
TXID1=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$TX1\"]}" \
  | jq -r '.result')

# 3. Submit TX2 (should fail)
RESPONSE=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$TX2\"]}")

echo "$RESPONSE" | jq '.error.message'
# Expected: "Double spend detected: input prev_txid=... already spent in mempool"
```

**Expected Result:**
- TX1 accepted into mempool
- TX2 rejected with error: `"Double spend detected: input ... already spent in mempool"`
- `spent_outpoints` HashSet correctly tracks all mempool inputs

**Assertion Criteria:**
```rust
// From crates/mempool/src/lib.rs:262
for input in &entry.tx.inputs {
    let outpoint = OutPoint::new(input.prev_txid, input.prev_vout);
    if self.spent_outpoints.contains(&outpoint) {
        return Err(Error::Invalid("Double spend detected..."));
    }
}
```

**Failure Threshold:** **CRITICAL**  
Double-spend acceptance → race conditions → loss of funds.

---

#### Test Case MEM-004: Replace-By-Fee (RBF) Logic Validation

**Subsystem:** `crates/mempool/src/lib.rs` (future RBF support)  
**Objective:** Verify that higher-fee transactions can replace existing mempool transactions (BIP-125 style).

**Status:** ⚠️ **NOT YET IMPLEMENTED** — Placeholder for Phase 2 development.

**Expected Behavior:**
- Transaction with same inputs but 2x fee replaces existing transaction
- Original transaction removed from mempool
- New transaction broadcast to peers

**Implementation Reference:** Bitcoin BIP-125, BQSegWit fee calculation.

---

## 4. P2P NETWORKING & ECLIPSE/SYBIL RESISTANCE

### Test Suite: NET — Network Security & Synchronization

#### Test Case NET-001: Initial Block Download (IBD) Memory Boundedness (10,000 Blocks)

**Subsystem:** `crates/network/src/sync.rs`, `crates/network/src/async_sync.rs`  
**Objective:** Verify that IBD (Initial Block Download) processes 10,000 blocks without memory leaks, with backpressure queue bounded at 50 blocks in RAM.

**Adversarial Threat Model:**  
- **Attack Vector:** Malicious peer floods node with 10,000 block headers, attempts to exhaust RAM during sync.
- **Defense:** Backpressure queue limits in-flight blocks to 50, rest fetched on-demand.

**Prerequisites:**
- Seed node with 10,000 blocks pre-mined
- Fresh node starting IBD from genesis

**Execution Commands:**
```bash
# 1. Start seed node with 10k block chain
./scripts/test-cluster.sh start-seed --blocks 10000

# 2. Start syncing node with memory profiling
valgrind --tool=massif --massif-out-file=/tmp/ibd-mem.out \
  ./target/release/bitquan-node run \
    --config config/testnet.toml \
    --datadir /tmp/ibd-node \
    --peer 127.0.0.1:19444 &
IBD_PID=$!

# 3. Monitor sync progress
while true; do
  HEIGHT=$(curl -s http://localhost:19445/rpc \
    -d '{"method":"getblockcount"}' \
    | jq -r '.result')
  
  echo "Height: $HEIGHT / 10000"
  
  if [ "$HEIGHT" -ge 10000 ]; then
    break
  fi
  
  sleep 10
done

# 4. Analyze memory usage
ms_print /tmp/ibd-mem.out > /tmp/ibd-mem-report.txt
MAX_MEM=$(grep "peak" /tmp/ibd-mem-report.txt | awk '{print $2}')

echo "Peak memory usage: $MAX_MEM MB"

# 5. Verify bounded memory (< 2GB for 10k blocks)
if [ "$MAX_MEM" -lt 2048 ]; then
  echo "✅ PASS: IBD memory bounded"
else
  echo "❌ FAIL: Memory exceeded 2GB"
fi

kill $IBD_PID
```

**Expected Result:**
- Node syncs all 10,000 blocks successfully
- Peak memory usage < 2 GB (50 blocks × ~4 MB each + overhead)
- No memory leaks detected (memory returns to baseline after sync)
- Sync completes in < 10 minutes (network latency dependent)

**Assertion Criteria:**
```rust
// From crates/network/src/async_sync.rs
const MAX_BLOCKS_IN_FLIGHT: usize = 50;
assert!(sync_state.in_flight_blocks.len() <= MAX_BLOCKS_IN_FLIGHT);
```

**Failure Threshold:** **HIGH**  
Unbounded IBD memory → OOM crash during sync → nodes cannot join network.

**Reference:** Bitcoin Core IBD backpressure mechanism, `MAX_BLOCKS_IN_FLIGHT`

---

#### Test Case NET-002: Noise Protocol Handshake TOCTOU Race Condition

**Subsystem:** `crates/network/src/noise.rs`  
**Objective:** Verify that Noise Protocol handshake is atomic and immune to Time-of-Check-Time-of-Use (TOCTOU) attacks.

**Adversarial Threat Model:**  
- **Attack Vector:** Man-in-the-middle attacker intercepts handshake, replaces ephemeral keys during key exchange.
- **Defense:** Noise XX pattern with mutual authentication, handshake state machine enforced.

**Execution Commands:**
```bash
# 1. Start node with Noise encryption enabled
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --enable-encryption true &

# 2. Simulate MITM attack with modified handshake
./scripts/attack/mitm-noise-handshake.py \
  --target localhost:19444 \
  --modify-ephemeral-key

# 3. Monitor connection attempts
tail -f /tmp/bitquan-testnet/node.log | grep "Noise handshake"

# Expected log output:
# [WARN] Noise handshake failed: authentication failure
# [INFO] Peer banned: 127.0.0.1 (reason: invalid Noise handshake)
```

**Expected Result:**
- Handshake fails with authentication error
- MITM peer banned immediately (invalid handshake = malicious)
- No plaintext data transmitted (all traffic encrypted or rejected)

**Assertion Criteria:**
```rust
// From crates/network/src/noise.rs
match noise_transport.handshake(&mut stream) {
    Ok(_) => { /* authenticated */ },
    Err(NoiseError::Authentication) => {
        ban_manager.ban_peer(peer_id, "invalid Noise handshake");
    }
}
```

**Failure Threshold:** **CRITICAL**  
MITM vulnerability → traffic interception → transaction theft, double-spend coordination.

**Reference:** Noise Protocol Framework, XX pattern specification

---

#### Test Case NET-003: Inbound Peer Limit Enforcement (Sybil Resistance)

**Subsystem:** `crates/network/src/connection_manager.rs`  
**Objective:** Verify that node enforces maximum inbound peer limit (125 connections) to prevent Sybil attacks.

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker spawns 1000 fake nodes, connects to victim node, eclipses honest peers.
- **Defense:** Connection limit enforced, excess connections rejected.

**Execution Commands:**
```bash
# 1. Start target node (max_peers=125)
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --max-peers 125 &

# 2. Launch 200 connection attempts
for i in {1..200}; do
  (echo -e "HELLO\n" | nc localhost 19444 &)
done

# 3. Count active connections
CONN_COUNT=$(netstat -an | grep ":19444" | grep ESTABLISHED | wc -l)

echo "Active connections: $CONN_COUNT"

# 4. Verify limit enforced
if [ "$CONN_COUNT" -le 125 ]; then
  echo "✅ PASS: Peer limit enforced"
else
  echo "❌ FAIL: Peer limit exceeded"
fi
```

**Expected Result:**
- Maximum 125 connections accepted
- Connection attempts 126-200 rejected with TCP RST
- No resource exhaustion (file descriptors, memory)

**Assertion Criteria:**
```rust
// From crates/network/src/connection_manager.rs
if self.active_connections() >= self.config.max_peers {
    return Err(ConnectionError::MaxPeersReached);
}
```

**Failure Threshold:** **HIGH**  
Unbounded connections → Eclipse attack → node isolated from honest network.

**Reference:** Bitcoin Core connection management, `MAX_OUTBOUND_FULL_RELAY_CONNECTIONS`

---

#### Test Case NET-004: Slowloris Attack Protection (30-Second Total Timeout)

**Subsystem:** `crates/network/src/peer_async.rs`, `crates/network/src/dos_protection.rs`  
**Objective:** Verify that node disconnects peers who send partial messages slowly (Slowloris DoS attack).

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker opens 100 connections, sends 1 byte per second to keep connections alive while consuming resources.
- **Defense:** Tokio async I/O with 30-second total message timeout per connection.

**Execution Commands:**
```bash
# 1. Start node with async P2P
./target/release/bitquan-node run --config config/testnet.toml &

# 2. Launch Slowloris attack
./scripts/attack/slowloris.py \
  --target localhost:19444 \
  --connections 100 \
  --rate 1  # 1 byte per second

# 3. Monitor connection timeouts
tail -f /tmp/bitquan-testnet/node.log | grep "timeout"

# Expected: All 100 connections terminated within 30 seconds
```

**Expected Result:**
- All Slowloris connections terminated after 30 seconds
- Node remains responsive to legitimate peers
- No file descriptor exhaustion

**Assertion Criteria:**
```rust
// From crates/network/src/peer_async.rs
tokio::time::timeout(Duration::from_secs(30), read_frame(&mut stream))
    .await
    .map_err(|_| NetworkError::Timeout)?;
```

**Failure Threshold:** **HIGH**  
Slowloris success → resource exhaustion → node unavailable to honest peers.

---

#### Test Case NET-005: Malicious Peer Banning (IP + Peer ID)

**Subsystem:** `crates/network/src/ban_manager.rs`  
**Objective:** Verify that peers sending invalid blocks are banned for 24 hours (both IP and Peer ID).

**Execution Commands:**
```bash
# 1. Start node with ban management enabled
./target/release/bitquan-node run --config config/testnet.toml &

# 2. Send invalid block from test peer
./scripts/attack/send-invalid-block.sh \
  --target localhost:19444 \
  --peer-id "attacker-node-001" \
  --invalid-pow  # Block with insufficient PoW

# 3. Verify ban
curl -s http://localhost:19443/rpc \
  -d '{"method":"listbanned","params":[]}' \
  | jq -r '.result[] | select(.address == "127.0.0.1")'

# Expected: {address: "127.0.0.1", banned_until: <timestamp+86400>, reason: "invalid block"}
```

**Expected Result:**
- Attacker peer banned for 86,400 seconds (24 hours)
- Reconnection attempts rejected during ban period
- Ban persists across node restarts (stored in RocksDB)

**Assertion Criteria:**
```rust
// From crates/network/src/ban_manager.rs
assert!(ban_manager.is_banned(&peer_id));
assert!(ban_manager.is_ip_banned(&attacker_ip));
assert_eq!(ban_info.duration_secs, 86400);
```

**Failure Threshold:** **MEDIUM**  
Missing bans → attackers retry indefinitely → resource waste defending against same attacker.

---

## 5. RPC & API SECURITY

### Test Suite: RPC — JSON-RPC Interface Security

#### Test Case RPC-001: JWT Authentication Bypass Attempt

**Subsystem:** `crates/rpc/src/lib.rs`, JWT authentication middleware  
**Objective:** Verify that RPC endpoints reject requests without valid JWT tokens.

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker attempts to call privileged RPC methods (`generatetoaddress`, `stop`) without authentication.
- **Defense:** JWT middleware rejects unauthenticated requests with HTTP 401.

**Prerequisites:**
- Node running with JWT authentication enabled (default)
- JWT secret configured in `jwt.toml`

**Execution Commands:**
```bash
# 1. Start node with JWT auth
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --jwt-secret jwt.example.toml &

# 2. Attempt RPC call without JWT
curl -s http://localhost:19443/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"generatetoaddress","params":[1,"bq1q..."],"id":1}' \
  | jq '.error'

# Expected: {"code": -32600, "message": "Unauthorized: missing or invalid JWT"}

# 3. Attempt with invalid JWT
curl -s http://localhost:19443/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.INVALID.SIGNATURE" \
  -d '{"jsonrpc":"2.0","method":"generatetoaddress","params":[1,"bq1q..."],"id":1}' \
  | jq '.error'

# Expected: {"code": -32600, "message": "Unauthorized: invalid JWT signature"}

# 4. Verify with valid JWT
JWT=$(./scripts/generate-jwt.sh --secret jwt.example.toml --role admin)
curl -s http://localhost:19443/rpc \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
  | jq '.result'

# Expected: <block count number>
```

**Expected Result:**
- Requests without JWT: HTTP 401 Unauthorized
- Requests with invalid JWT: HTTP 401 with signature error
- Requests with valid JWT: HTTP 200 with result

**Assertion Criteria:**
```rust
// From crates/rpc/src/middleware.rs
if !request.headers().contains_key("authorization") {
    return Err(RpcError::Unauthorized("missing JWT"));
}
let token = validate_jwt(&jwt_string, &config.jwt_secret)?;
```

**Failure Threshold:** **CRITICAL**  
Authentication bypass → unauthorized mining control → network manipulation.

**Reference:** RFC 7519 (JWT), BQIP-0005 RPC Authentication

---

#### Test Case RPC-002: Role-Based Access Control (Miner vs Read-Only)

**Subsystem:** `crates/rpc/src/lib.rs`, JWT role claims  
**Objective:** Verify that `Miner` role cannot call `Admin` methods (e.g., `stop`, `invalidateblock`).

**Execution Commands:**
```bash
# 1. Generate JWT with Miner role
JWT_MINER=$(./scripts/generate-jwt.sh --secret jwt.example.toml --role miner)

# 2. Attempt Admin-only method
curl -s http://localhost:19443/rpc \
  -H "Authorization: Bearer $JWT_MINER" \
  -d '{"method":"stop","params":[],"id":1}' \
  | jq '.error.message'

# Expected: "Forbidden: insufficient permissions (requires: Admin, has: Miner)"

# 3. Attempt Miner-allowed method
curl -s http://localhost:19443/rpc \
  -H "Authorization: Bearer $JWT_MINER" \
  -d '{"method":"getblocktemplate","params":[],"id":1}' \
  | jq '.result'

# Expected: { ... block template data ... }
```

**Expected Result:**
- Miner role can call: `getblocktemplate`, `submitblock`, `getmininginfo`
- Miner role **cannot** call: `stop`, `invalidateblock`, `setban`
- Admin role can call all methods

**Assertion Criteria:**
```rust
// From crates/rpc/src/auth.rs
match (method, token.role) {
    ("stop", Role::Admin) => Ok(()),
    ("stop", _) => Err(RpcError::Forbidden),
    // ...
}
```

**Failure Threshold:** **HIGH**  
RBAC bypass → privilege escalation → miners can stop nodes, manipulate chain state.

---

#### Test Case RPC-003: Rate Limiting (100 Requests/Second Per IP)

**Subsystem:** `crates/rpc/src/rate_limiter.rs`  
**Objective:** Verify that RPC rate limiting blocks IPs exceeding 100 req/sec.

**Execution Commands:**
```bash
# 1. Flood RPC with 1000 requests in 1 second
for i in {1..1000}; do
  curl -s http://localhost:19443/rpc \
    -d '{"method":"getblockcount","id":'$i'}' &
done

wait

# 2. Count successful vs rate-limited responses
SUCCESS=$(grep -c '"result"' /tmp/rpc-responses.log)
RATE_LIMITED=$(grep -c '"code":-32005' /tmp/rpc-responses.log)

echo "Successful: $SUCCESS, Rate-limited: $RATE_LIMITED"

# Expected: SUCCESS ~100, RATE_LIMITED ~900
```

**Expected Result:**
- First ~100 requests succeed
- Remaining ~900 requests rejected with: `{"code": -32005, "message": "Rate limit exceeded"}`
- Rate limit resets after 1 second window

**Assertion Criteria:**
```rust
// From crates/rpc/src/rate_limiter.rs
const MAX_REQUESTS_PER_SECOND: u32 = 100;
if request_count > MAX_REQUESTS_PER_SECOND {
    return Err(RpcError::RateLimitExceeded);
}
```

**Failure Threshold:** **MEDIUM**  
Missing rate limiting → RPC DoS → node unresponsive to legitimate requests.

---

#### Test Case RPC-004: Computationally Heavy Endpoint DoS (`generatetoaddress`)

**Subsystem:** `crates/rpc/src/methods/mining.rs`  
**Objective:** Verify that `generatetoaddress` is rate-limited to prevent CPU exhaustion attacks.

**Adversarial Threat Model:**  
- **Attack Vector:** Attacker calls `generatetoaddress(1000000, ...)` to mine 1 million blocks.
- **Defense:** Block count capped at 1000 per request, admin-only access.

**Execution Commands:**
```bash
# Attempt to mine 1 million blocks
JWT_ADMIN=$(./scripts/generate-jwt.sh --secret jwt.example.toml --role admin)

curl -s http://localhost:19443/rpc \
  -H "Authorization: Bearer $JWT_ADMIN" \
  -d '{"method":"generatetoaddress","params":[1000000,"bq1q..."],"id":1}' \
  | jq '.error.message'

# Expected: "Block count exceeds maximum (1000)"
```

**Expected Result:**
- Request rejected with parameter validation error
- Maximum 1000 blocks per `generatetoaddress` call
- Even admin role subject to this limit (prevents accidental DoS)

**Assertion Criteria:**
```rust
// From crates/rpc/src/methods/mining.rs
const MAX_GENERATE_BLOCKS: u64 = 1000;
if block_count > MAX_GENERATE_BLOCKS {
    return Err(RpcError::InvalidParams("block count exceeds maximum"));
}
```

**Failure Threshold:** **MEDIUM**  
Unbounded generation → CPU saturation → node hangs for hours.

---

#### Test Case RPC-005: `getblock` DoS with Non-Existent Hash

**Subsystem:** `crates/rpc/src/methods/blockchain.rs`  
**Objective:** Verify that `getblock` with invalid hash fails fast without scanning entire database.

**Execution Commands:**
```bash
# Request non-existent block
time curl -s http://localhost:19443/rpc \
  -d '{"method":"getblock","params":["0x0000000000000000000000000000000000000000000000000000000000000000"],"id":1}' \
  | jq '.error'

# Expected: {"code": -5, "message": "Block not found"}, latency < 10ms
```

**Expected Result:**
- Request fails with "Block not found" error
- Latency < 10ms (indexed lookup, not full scan)
- No database lock held during error path

**Assertion Criteria:**
```rust
// From crates/rpc/src/methods/blockchain.rs
let block = storage.get_block(&hash)
    .ok_or(RpcError::BlockNotFound)?;  // Fail-fast
```

**Failure Threshold:** **LOW**  
Slow error paths → amplification attack (1 request = 1 second database scan).

---

#### Test Case RPC-006: JSON-RPC Batch Request DoS

**Subsystem:** `crates/rpc/src/server.rs`  
**Objective:** Verify that batch RPC requests are limited to prevent resource exhaustion.

**Execution Commands:**
```bash
# Send batch request with 10,000 calls
cat > /tmp/batch-request.json <<EOF
[
$(for i in {1..10000}; do echo '{"method":"getblockcount","id":'$i'},'; done | sed '$ s/,$//')
]
EOF

curl -s http://localhost:19443/rpc \
  -H "Content-Type: application/json" \
  --data @/tmp/batch-request.json \
  | jq '.error.message'

# Expected: "Batch size exceeds maximum (100)"
```

**Expected Result:**
- Batch requests limited to 100 calls per request
- Requests with >100 calls rejected before processing
- No partial batch processing (all-or-nothing)

**Assertion Criteria:**
```rust
// From crates/rpc/src/server.rs
const MAX_BATCH_SIZE: usize = 100;
if batch.len() > MAX_BATCH_SIZE {
    return Err(RpcError::BatchTooLarge);
}
```

**Failure Threshold:** **MEDIUM**  
Unbounded batch → 10,000 parallel database queries → node DoS.

---

## 6. STORAGE INTEGRITY

### Test Suite: STO — RocksDB Persistence & Recovery

#### Test Case STO-001: RocksDB Recovery After Abrupt Power Loss (SIGKILL)

**Subsystem:** `crates/storage/src/lib.rs`  
**Objective:** Verify that RocksDB storage recovers cleanly after node killed with SIGKILL (simulates power failure).

**Adversarial Threat Model:**  
- **Scenario:** Node mining blocks, suddenly loses power mid-write.
- **Risk:** Corrupted database → node cannot restart → data loss.

**Prerequisites:**
- Node running and actively mining blocks

**Execution Commands:**
```bash
# 1. Start node with mining
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/storage-recovery-test \
  --mine &
NODE_PID=$!

# 2. Wait for 50 blocks
while [ $(curl -s http://localhost:19443/rpc -d '{"method":"getblockcount"}' | jq '.result') -lt 50 ]; do
  sleep 5
done

# 3. Kill node abruptly (simulate power loss)
kill -9 $NODE_PID

# 4. Restart node
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/storage-recovery-test &

# 5. Verify recovery
sleep 10
HEIGHT=$(curl -s http://localhost:19443/rpc -d '{"method":"getblockcount"}' | jq '.result')

if [ "$HEIGHT" -ge 49 ]; then  # May lose 1 block
  echo "✅ PASS: RocksDB recovered, height = $HEIGHT"
else
  echo "❌ FAIL: Data loss, height = $HEIGHT (expected ~50)"
fi
```

**Expected Result:**
- Node restarts successfully
- Block height ≥ 49 (may lose last uncommitted block)
- No database corruption errors in logs
- UTXO index consistent with block tip

**Assertion Criteria:**
```rust
// From crates/storage/src/lib.rs
let db = DB::open(&options, path)?;  // Should succeed after recovery
assert!(db.get(b"block_height")?.is_some());
```

**Failure Threshold:** **CRITICAL**  
Corruption after crash → node cannot restart → network loses validators.

**Reference:** RocksDB WAL (Write-Ahead Log) recovery mechanism

---

#### Test Case STO-002: UTXO Index Consistency After Reorg

**Subsystem:** `crates/storage/src/utxo_index.rs`  
**Objective:** Verify that UTXO index correctly reverts after a 50-block reorg.

**Prerequisites:**
- Main chain at height 100
- Competing chain causes 50-block reorg

**Execution Commands:**
```bash
# 1. Mine main chain to height 100, record UTXO set
./scripts/mine-chain.sh --blocks 100 --output /tmp/utxo-test
UTXO_SET_100=$(./scripts/dump-utxo-set.sh /tmp/utxo-test | sha256sum)

# 2. Trigger 50-block reorg (see CON-003)
./scripts/trigger-reorg.sh --depth 50 --datadir /tmp/utxo-test

# 3. Dump UTXO set after reorg
UTXO_SET_POST_REORG=$(./scripts/dump-utxo-set.sh /tmp/utxo-test | sha256sum)

# 4. Verify UTXO set consistency
# Note: UTXO set will differ (different transactions), but must be valid for new tip
./target/release/bitquan-node debug validate-utxo \
  --datadir /tmp/utxo-test

# Expected: "UTXO index is consistent with block tip"
```

**Expected Result:**
- UTXO index updated to reflect new chain tip (height 101 on competing chain)
- Spent outputs from disconnected blocks (51-100) restored to UTXO set
- New outputs from competing blocks (51-101) added to UTXO set
- No orphaned UTXOs (all outputs reference valid blocks)

**Assertion Criteria:**
```rust
// Validate every UTXO references a block in the active chain
for (outpoint, entry) in utxo_set.iter() {
    let tx_block = storage.get_transaction_block(&outpoint.txid)?;
    assert!(is_in_active_chain(tx_block.hash));
}
```

**Failure Threshold:** **CRITICAL**  
UTXO inconsistency → double-spend vulnerability → loss of funds.

---

#### Test Case STO-003: Column Family Integrity (Block vs UTXO)

**Subsystem:** `crates/storage/src/lib.rs`  
**Objective:** Verify that RocksDB column families (blocks, headers, utxo) remain isolated and consistent.

**Execution Commands:**
```bash
# 1. Corrupt UTXO column family manually
./scripts/storage/corrupt-column-family.sh \
  --datadir /tmp/cf-test \
  --cf utxo \
  --corrupt-bytes 100

# 2. Attempt to start node
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/cf-test \
  2>&1 | grep "corruption"

# Expected: "RocksDB corruption detected in column family 'utxo'"

# 3. Verify block CF still readable
./scripts/storage/read-column-family.sh \
  --datadir /tmp/cf-test \
  --cf blocks \
  --key "block_0"

# Expected: Genesis block data returned
```

**Expected Result:**
- Node detects corruption and refuses to start
- Error message identifies corrupted column family
- Other column families (blocks, headers) remain readable
- Repair tool recommended in error message

**Assertion Criteria:**
```rust
// From crates/storage/src/lib.rs
let cf_utxo = db.cf_handle("utxo").expect("utxo CF exists");
match db.get_cf(cf_utxo, key) {
    Err(Error::Corruption(_)) => {
        log::error!("RocksDB corruption detected in column family 'utxo'");
        return Err(StorageError::Corruption);
    }
}
```

**Failure Threshold:** **HIGH**  
Silent corruption → incorrect validation → chain consensus failure.

---

#### Test Case STO-004: Storage Compaction Under Load (1M Blocks)

**Subsystem:** `crates/storage/src/lib.rs`, RocksDB compaction  
**Objective:** Verify that RocksDB compaction runs automatically and keeps storage bounded under sustained block production.

**Prerequisites:**
- Node mining continuously for 24 hours (target: 1,000,000 blocks at 60s interval)

**Execution Commands:**
```bash
# 1. Start node with storage metrics
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/compaction-test \
  --mine \
  --enable-storage-metrics &

# 2. Monitor storage size over 24 hours
for i in {1..1440}; do  # Every minute for 24 hours
  SIZE=$(du -sh /tmp/compaction-test/rocksdb | awk '{print $1}')
  echo "$(date),$(curl -s http://localhost:19443/rpc -d '{"method":"getblockcount"}' | jq '.result'),$SIZE" >> /tmp/storage-growth.csv
  sleep 60
done

# 3. Analyze growth rate
./scripts/analyze-storage-growth.py /tmp/storage-growth.csv

# Expected: Linear growth < 10 MB/hour, compaction events logged
```

**Expected Result:**
- Storage grows linearly (no unbounded growth from uncollected garbage)
- RocksDB compaction runs automatically (log events: "Compaction started/finished")
- Final storage size < 24 hours × 10 MB/hr = 240 MB for empty blocks

**Assertion Criteria:**
```rust
// RocksDB compaction should be enabled in options
let mut db_opts = Options::default();
db_opts.set_max_background_jobs(4);  // Enable background compaction
```

**Failure Threshold:** **MEDIUM**  
Missing compaction → storage grows unbounded → disk full → node crash.

---

## 7. CROSS-SUBSYSTEM INTEGRATION TESTS

### Test Suite: INT — End-to-End Integration

#### Test Case INT-001: Full Block Lifecycle (Mine → Propagate → Validate → Store)

**Objective:** Verify complete block flow from mining to persistent storage across 3-node network.

**Execution Commands:**
```bash
# 1. Start 3-node cluster
docker compose -f docker-compose.cluster.yml up -d

# 2. Mine block on node-miner-1
BLOCK_HASH=$(curl -s http://localhost:19445/rpc \
  -d '{"method":"generatetoaddress","params":[1,"bq1qminer..."]}' \
  | jq -r '.result[0]')

# 3. Verify propagation to all nodes within 10 seconds
sleep 10

for PORT in 19443 19445 19447; do
  HEIGHT=$(curl -s http://localhost:$PORT/rpc \
    -d '{"method":"getblockcount"}' | jq '.result')
  
  HASH=$(curl -s http://localhost:$PORT/rpc \
    -d '{"method":"getbestblockhash"}' | jq -r '.result')
  
  echo "Node $PORT: height=$HEIGHT, hash=$HASH"
  
  if [ "$HASH" != "$BLOCK_HASH" ]; then
    echo "❌ FAIL: Node $PORT did not receive block"
    exit 1
  fi
done

echo "✅ PASS: Block propagated to all nodes"
```

**Expected Result:**
- Block mined on node-miner-1 within 30 seconds
- Block propagates to node-seed and node-relay-2 within 10 seconds
- All nodes agree on best block hash
- Block stored in RocksDB on all nodes

**Failure Threshold:** **CRITICAL**  
Propagation failure → network partition → chain split.

---

#### Test Case INT-002: Transaction Lifecycle (Create → Sign → Broadcast → Mine → Confirm)

**Objective:** Verify full transaction flow from wallet creation to blockchain confirmation.

**Execution Commands:**
```bash
# 1. Generate two wallets
./target/release/bitquan-node wallet-gen --output alice.keystore --password alice123
./target/release/bitquan-node wallet-gen --output bob.keystore --password bob123

ALICE_ADDR=$(./target/release/bitquan-node wallet-address --keystore alice.keystore --password alice123)
BOB_ADDR=$(./target/release/bitquan-node wallet-address --keystore bob.keystore --password bob123)

# 2. Fund Alice with 100 BQ via mining
./target/release/bitquan-node mine \
  --coinbase-address "$ALICE_ADDR" \
  --blocks 10 \
  --network devnet

# 3. Create and sign transaction: Alice → Bob (10 BQ)
TX_HEX=$(./target/release/bitquan-cli create-tx \
  --from "$ALICE_ADDR" \
  --to "$BOB_ADDR" \
  --amount 10.0 \
  --keystore alice.keystore \
  --password alice123)

# 4. Broadcast transaction
TXID=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$TX_HEX\"]}" \
  | jq -r '.result')

echo "Transaction broadcast: $TXID"

# 5. Verify in mempool
MEMPOOL=$(curl -s http://localhost:19443/rpc \
  -d '{"method":"getrawmempool"}' | jq -r ".result[] | select(. == \"$TXID\")")

if [ -z "$MEMPOOL" ]; then
  echo "❌ FAIL: Transaction not in mempool"
  exit 1
fi

# 6. Mine block to confirm transaction
./target/release/bitquan-node mine --blocks 1

# 7. Verify confirmation
CONFIRMATIONS=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"gettransaction\",\"params\":[\"$TXID\"]}" \
  | jq '.result.confirmations')

if [ "$CONFIRMATIONS" -ge 1 ]; then
  echo "✅ PASS: Transaction confirmed with $CONFIRMATIONS confirmations"
else
  echo "❌ FAIL: Transaction not confirmed"
fi

# 8. Verify Bob's balance
BOB_BALANCE=$(./target/release/bitquan-cli getbalance --address "$BOB_ADDR")

if [ "$BOB_BALANCE" == "10.0" ]; then
  echo "✅ PASS: Bob received 10 BQ"
else
  echo "❌ FAIL: Bob balance = $BOB_BALANCE (expected 10.0)"
fi
```

**Expected Result:**
- Transaction accepted into mempool with valid Dilithium5 signature
- Transaction mined into block within 120 seconds
- Bob's balance increases by exactly 10 BQ
- Alice's balance decreases by 10 BQ + fee

**Failure Threshold:** **CRITICAL**  
Transaction flow failure → blockchain unusable for value transfer.

---

#### Test Case INT-003: Wallet Recovery from BIP39 Mnemonic

**Objective:** Verify that HD wallet can be recovered from 12-word BIP39 mnemonic phrase.

**Execution Commands:**
```bash
# 1. Generate wallet with mnemonic
MNEMONIC=$(./target/release/bitquan-node wallet-gen-mnemonic)
echo "Mnemonic: $MNEMONIC"

ADDRESS_ORIGINAL=$(./target/release/bitquan-node wallet-from-mnemonic \
  --phrase "$MNEMONIC" \
  --output wallet-original.keystore \
  --password test123 && \
  ./target/release/bitquan-node wallet-address \
  --keystore wallet-original.keystore \
  --password test123)

# 2. Delete wallet, recover from mnemonic
rm wallet-original.keystore

ADDRESS_RECOVERED=$(./target/release/bitquan-node wallet-from-mnemonic \
  --phrase "$MNEMONIC" \
  --output wallet-recovered.keystore \
  --password test123 && \
  ./target/release/bitquan-node wallet-address \
  --keystore wallet-recovered.keystore \
  --password test123)

# 3. Verify addresses match
if [ "$ADDRESS_ORIGINAL" == "$ADDRESS_RECOVERED" ]; then
  echo "✅ PASS: Wallet recovered successfully"
else
  echo "❌ FAIL: Address mismatch"
  echo "Original:  $ADDRESS_ORIGINAL"
  echo "Recovered: $ADDRESS_RECOVERED"
fi
```

**Expected Result:**
- Original and recovered wallets produce identical addresses
- Private keys derived deterministically from mnemonic (BIP39 + BIP32 HD derivation)
- Mnemonic phrase checksum validated during recovery

**Failure Threshold:** **CRITICAL**  
Non-deterministic recovery → users cannot restore wallets → permanent loss of funds.

**Reference:** BIP39 (Mnemonic), BIP32 (HD Wallets), BQIP-0003 Wallet Standards

---

#### Test Case INT-004: Multi-Node Consensus Under Network Partition (Split-Brain)

**Objective:** Verify that when network partitions into two groups, nodes reconverge after partition heals.

**Execution Commands:**
```bash
# 1. Start 5-node cluster
./scripts/test-cluster.sh start --nodes 5

# 2. Partition network: [Node1, Node2] vs [Node3, Node4, Node5]
iptables -A INPUT -s 172.28.0.4 -j DROP  # Block Node3
iptables -A INPUT -s 172.28.0.5 -j DROP  # Block Node4
iptables -A INPUT -s 172.28.0.6 -j DROP  # Block Node5

# 3. Mine on both partitions
# Partition A (Node1, Node2)
curl -s http://localhost:19443/rpc \
  -d '{"method":"generatetoaddress","params":[10,"bq1qpartA..."]}' &

# Partition B (Node3, Node4, Node5) — majority
curl -s http://localhost:19447/rpc \
  -d '{"method":"generatetoaddress","params":[15,"bq1qpartB..."]}' &

wait
sleep 30

# 4. Heal partition
iptables -F  # Clear all firewall rules

sleep 60  # Allow sync

# 5. Verify all nodes converge on Partition B chain (higher work)
for PORT in 19443 19445 19447 19449 19451; do
  HEIGHT=$(curl -s http://localhost:$PORT/rpc -d '{"method":"getblockcount"}' | jq '.result')
  HASH=$(curl -s http://localhost:$PORT/rpc -d '{"method":"getbestblockhash"}' | jq -r '.result')
  echo "Node $PORT: height=$HEIGHT, hash=${HASH:0:16}..."
done

# Expected: All nodes at height 15 with same best block hash (Partition B wins)
```

**Expected Result:**
- Partition A mines 10 blocks, Partition B mines 15 blocks
- After partition heals, Partition A nodes reorg to Partition B chain (higher work)
- All 5 nodes converge on identical chain tip within 2 minutes
- Transactions unique to Partition A blocks return to mempool

**Failure Threshold:** **CRITICAL**  
Failed reconvergence → permanent chain split → two incompatible BitQuan networks.

**Reference:** Bitcoin's longest-chain rule, fork choice algorithm

---

#### Test Case INT-005: Mempool Synchronization Across Peers

**Objective:** Verify that transactions broadcast to one node propagate to all connected peers.

**Execution Commands:**
```bash
# 1. Start 3-node cluster
docker compose -f docker-compose.cluster.yml up -d

# 2. Submit transaction to node-seed only
TX=$(./target/release/bitquan-cli create-tx \
  --from bq1q... --to bq1q... --amount 1.0 \
  --keystore wallet.keystore --password test123)

TXID=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"sendrawtransaction\",\"params\":[\"$TX\"]}" \
  | jq -r '.result')

# 3. Verify propagation to peers within 5 seconds
sleep 5

for PORT in 19445 19447; do
  MEMPOOL=$(curl -s http://localhost:$PORT/rpc \
    -d '{"method":"getrawmempool"}' \
    | jq -r ".result[] | select(. == \"$TXID\")")
  
  if [ -n "$MEMPOOL" ]; then
    echo "✅ Node $PORT has transaction"
  else
    echo "❌ Node $PORT missing transaction"
    exit 1
  fi
done

echo "✅ PASS: Transaction propagated to all peers"
```

**Expected Result:**
- Transaction propagates from node-seed to node-miner-1 and node-relay-2 within 5 seconds
- All nodes have identical mempool contents
- No duplicate transaction broadcasts (seen filter working)

**Failure Threshold:** **HIGH**  
Failed propagation → incomplete mempool → miner misses fee-paying transactions.

---

#### Test Case INT-006: RPC JWT Token Expiration (1-Hour TTL)

**Objective:** Verify that JWT tokens expire after configured TTL (default: 1 hour).

**Execution Commands:**
```bash
# 1. Generate JWT with 5-second expiration (for testing)
JWT=$(./scripts/generate-jwt.sh \
  --secret jwt.example.toml \
  --role admin \
  --ttl 5)

# 2. Immediate use (should succeed)
curl -s http://localhost:19443/rpc \
  -H "Authorization: Bearer $JWT" \
  -d '{"method":"getblockcount"}' \
  | jq '.result'

# Expected: Block count returned

# 3. Wait 6 seconds, retry (should fail)
sleep 6

curl -s http://localhost:19443/rpc \
  -H "Authorization: Bearer $JWT" \
  -d '{"method":"getblockcount"}' \
  | jq '.error.message'

# Expected: "Unauthorized: JWT expired"
```

**Expected Result:**
- Fresh JWT works immediately
- Expired JWT rejected with "JWT expired" error
- No grace period (strict expiration enforcement)

**Failure Threshold:** **MEDIUM**  
Missing expiration → leaked tokens valid forever → security breach persistence.

---

#### Test Case INT-007: Faucet Drip Rate Limiting (10 BQ per Address per Day)

**Subsystem:** `crates/faucet/src/lib.rs`  
**Objective:** Verify testnet faucet enforces rate limits to prevent abuse.

**Execution Commands:**
```bash
# 1. Request faucet drip
ADDRESS="bq1qtest123..."
curl -s http://localhost:5000/faucet/drip \
  -d "{\"address\":\"$ADDRESS\"}" \
  | jq '.txid'

# Expected: Transaction ID returned

# 2. Immediate second request (should fail)
curl -s http://localhost:5000/faucet/drip \
  -d "{\"address\":\"$ADDRESS\"}" \
  | jq '.error'

# Expected: "Rate limit: address already funded in last 24 hours"

# 3. Verify balance
BALANCE=$(curl -s http://localhost:19443/rpc \
  -d "{\"method\":\"getbalance\",\"params\":[\"$ADDRESS\"]}" \
  | jq '.result')

if [ "$BALANCE" == "10.0" ]; then
  echo "✅ PASS: Faucet delivered 10 BQ, rate limit enforced"
else
  echo "❌ FAIL: Unexpected balance: $BALANCE"
fi
```

**Expected Result:**
- First request delivers 10 BQ to address
- Subsequent requests within 24 hours rejected
- Rate limit persists across faucet restarts (stored in Redis/database)

**Failure Threshold:** **LOW** (testnet only)  
Missing rate limits → faucet drained by abusers → testnet unusable.

---

#### Test Case INT-008: Node Graceful Shutdown (Save State, Close Connections)

**Objective:** Verify that node shutdown with SIGTERM saves all state cleanly.

**Execution Commands:**
```bash
# 1. Start node, mine 50 blocks
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/shutdown-test \
  --mine &
NODE_PID=$!

# Wait for 50 blocks
while [ $(curl -s http://localhost:19443/rpc -d '{"method":"getblockcount"}' | jq '.result') -lt 50 ]; do
  sleep 5
done

# 2. Graceful shutdown with SIGTERM
kill -TERM $NODE_PID

# Wait for shutdown
wait $NODE_PID

# 3. Verify logs show clean shutdown
grep "Shutting down gracefully" /tmp/shutdown-test/node.log

# 4. Restart and verify state
./target/release/bitquan-node run \
  --config config/testnet.toml \
  --datadir /tmp/shutdown-test &

sleep 10

HEIGHT=$(curl -s http://localhost:19443/rpc -d '{"method":"getblockcount"}' | jq '.result')

if [ "$HEIGHT" -eq 50 ]; then
  echo "✅ PASS: State preserved across shutdown"
else
  echo "❌ FAIL: Block height mismatch (expected 50, got $HEIGHT)"
fi
```

**Expected Result:**
- Node receives SIGTERM, begins graceful shutdown
- All P2P connections closed cleanly
- RocksDB flushed and closed
- Restart recovers exact state (height 50)

**Failure Threshold:** **MEDIUM**  
Unclean shutdown → data loss, corruption risk.

---

## APPENDIX A: Test Execution Matrix

| Test ID | Subsystem | Priority | Execution Time | Dependencies | Automation Status |
|---------|-----------|----------|----------------|--------------|-------------------|
| CON-001 | Consensus | CRITICAL | 5 min | None | ✅ Automated |
| CON-002 | Consensus | CRITICAL | 30 min | None | ✅ Automated |
| CON-003 | Fork Choice | CRITICAL | 10 min | Multi-node | ✅ Automated |
| CON-004 | Fork Choice | CRITICAL | 15 min | Multi-node | ✅ Automated |
| CON-005 | Consensus | MEDIUM | 2 min | None | ✅ Automated |
| PQC-001 | Crypto | CRITICAL | 3 min | None | ✅ Automated |
| PQC-002 | Crypto | MEDIUM | 1 min | None | ✅ Automated |
| PQC-003 | Crypto | CRITICAL | 5 min | Mainnet node | ⚠️ Manual (pre-mainnet) |
| PQC-004 | Crypto | CRITICAL | 20 min | NIST test suite | ✅ Automated |
| MEM-001 | Mempool | HIGH | 2 min | Load generator | ✅ Automated |
| MEM-002 | Mempool | MEDIUM | 3 min | None | ✅ Automated |
| MEM-003 | Mempool | CRITICAL | 2 min | None | ✅ Automated |
| MEM-004 | Mempool | LOW | N/A | Future feature | ❌ Not Implemented |
| NET-001 | Network | HIGH | 15 min | Seed node | ✅ Automated |
| NET-002 | Network | CRITICAL | 5 min | Attack scripts | ✅ Automated |
| NET-003 | Network | HIGH | 2 min | None | ✅ Automated |
| NET-004 | Network | HIGH | 3 min | Slowloris script | ✅ Automated |
| NET-005 | Network | MEDIUM | 2 min | None | ✅ Automated |
| RPC-001 | RPC | CRITICAL | 2 min | JWT config | ✅ Automated |
| RPC-002 | RPC | HIGH | 2 min | JWT config | ✅ Automated |
| RPC-003 | RPC | MEDIUM | 2 min | None | ✅ Automated |
| RPC-004 | RPC | MEDIUM | 3 min | Admin JWT | ✅ Automated |
| RPC-005 | RPC | LOW | 1 min | None | ✅ Automated |
| RPC-006 | RPC | MEDIUM | 2 min | None | ✅ Automated |
| STO-001 | Storage | CRITICAL | 5 min | None | ✅ Automated |
| STO-002 | Storage | CRITICAL | 15 min | Reorg setup | ✅ Automated |
| STO-003 | Storage | HIGH | 3 min | Corruption tool | ✅ Automated |
| STO-004 | Storage | MEDIUM | 24 hours | Long-running | ⚠️ CI Nightly only |
| INT-001 | Integration | CRITICAL | 5 min | Docker | ✅ Automated |
| INT-002 | Integration | CRITICAL | 10 min | Wallet setup | ✅ Automated |
| INT-003 | Integration | CRITICAL | 3 min | None | ✅ Automated |
| INT-004 | Integration | CRITICAL | 20 min | Multi-node + iptables | ⚠️ Requires sudo |
| INT-005 | Integration | HIGH | 5 min | Docker | ✅ Automated |
| INT-006 | Integration | MEDIUM | 10 sec | JWT config | ✅ Automated |
| INT-007 | Integration | LOW | 2 min | Faucet service | ✅ Automated |
| INT-008 | Integration | MEDIUM | 5 min | None | ✅ Automated |

**Total Test Cases:** 36  
**Critical Priority:** 18  
**High Priority:** 8  
**Medium Priority:** 9  
**Low Priority:** 1  

**Estimated Total Execution Time (Parallel):** ~45 minutes  
**Estimated Total Execution Time (Sequential):** ~28 hours

---

## APPENDIX B: Failure Severity Classification

### CRITICAL (18 tests)
**Definition:** Failure enables catastrophic attacks or complete system failure.

**Impact Examples:**
- Consensus divergence (chain split)
- Cryptographic bypass (double-spend, signature forgery)
- Data loss (corruption, non-deterministic recovery)
- Network partition (unable to sync, propagate blocks)

**Response:** BLOCK TESTNET LAUNCH until resolved.

### HIGH (8 tests)
**Definition:** Failure enables significant attacks or major functionality breakdown.

**Impact Examples:**
- DoS attacks (resource exhaustion, network flooding)
- Security degradation (bypassed rate limits, weak bans)
- Data inconsistency (UTXO mismatch, reorg failures)

**Response:** Fix within 48 hours before testnet launch.

### MEDIUM (9 tests)
**Definition:** Failure degrades performance or enables minor exploits.

**Impact Examples:**
- Slow error paths (amplification potential)
- Missing optimizations (compaction, batch limits)
- Testnet-only features (faucet rate limits)

**Response:** Fix before mainnet, can proceed with testnet if documented.

### LOW (1 test)
**Definition:** Nice-to-have functionality, no security impact.

**Impact Examples:**
- Future features not yet implemented (RBF)
- Non-essential tooling (advanced debugging)

**Response:** Backlog, no launch blocker.

---

## Test Execution Recommendations

### Phase 1: Pre-Testnet (Current)
**Execute:** All CRITICAL + HIGH tests (26 tests)  
**Target:** 100% pass rate  
**Timeline:** 2 weeks before testnet launch  

### Phase 2: Testnet (Public)
**Execute:** Full suite daily via CI  
**Monitor:** Real-world attack patterns, add adversarial tests as needed  
**Timeline:** Ongoing during testnet operation  

### Phase 3: Pre-Mainnet
**Execute:** Full suite + fuzzing + external audit  
**Target:** 100% pass rate + zero known vulnerabilities  
**Timeline:** Q4 2026 before mainnet launch  

---

**Document Status:** ✅ Complete — Ready for Engineering Review  
**Next Step:** Module 2 (Test Runbooks & Automated CLI Scripts)

---

**Signature:**  
*Principal L1 Blockchain Architect*  
*Date: 2026-08-14*
