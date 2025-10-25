# Consensus Economics & PQC Cost Model (Phase 3 Draft)

This note captures the initial production assumptions for Proof-of-Work security, post-quantum signature costs, and fee/weight policy. It complements `docs/architecture/overview.md` and the transaction/block data specification.

## 1. PQC Performance & Fee Policy
- **Default signature scheme:** CRYSTALS-Dilithium level 3 (public key ≈ 1.5 KB, signature ≈ 2.7 KB). Falcon512 and SPHINCS+ remain optional under `SigAlgorithm` but are not enabled by default.
- **Weight accounting:** The consensus weight formula continues to follow `weight = raw_bytes + α × (#pq_signatures)` with `α = 384` weight units per signature. This keeps a Dilithium-signed input roughly equivalent to a legacy ECDSA input once witness separation (Phase 4) is live.
- **Witness separation:** PQ signatures will migrate into a witness structure (expanded `Transaction::signatures`) while legacy script paths move into `witnesses[]`. Block relay protocols can then omit witness data for historical blocks, cutting bandwidth.
- **Batch verification:** Dilithium batch verify is mandatory for miners / validators. The consensus module exposes RNG-derived digest placeholders for now; Phase 4 replaces these with canonical sighash constructions.
- **Fee market:** Nodes prioritise `fee_per_weight` (sat/weight). Heavy witness data pushes a transaction toward fee auctions; wallet tooling should guide users toward L2 rollups once on-chain fees exceed target thresholds.
- **L2/Rollups:** Contract-style or high-volume applications are steered to anchored rollups. Base layer transactions focus on settlement, HTLC, bridge checkpoints, and rollup fraud proofs.

## 2. Proof-of-Work Security Model
- **Target block time:** 600 seconds.
- **Difficulty retarget:** ASERT (per-block) with a half-life of 86,400 seconds (~1 day). LWMA parameters remain an alternative for simulations but ASERT is the production default.
- **Tail emission:** Once halvings reduce the subsidy below **0.5 BQ** (50,000,000 satoshis-equivalent), the protocol locks a constant tail reward to preserve miner incentives (~0.5–1% annualized depending on fee share).
- **Block reward schedule:**
  - Initial subsidy: **50 BQ** (5,000,000,000 minimal units).
  - Halving interval: **210,000 blocks** (~4 years).
  - Tail emission per block: **0.5 BQ**.
- **Timestamp sanity:** Median-Time-Past (MTP) remains mandatory; block timestamps must be greater than the median of the previous 11 blocks and cannot drift more than +2 hours.
- **Pool decentralisation:**
  - Adopt Stratum V2 with job negotiation to reduce custodial control.
  - Encourage non-custodial pooling through BQIP proposals (e.g., payout scripts with multisig timelocks).
  - Integrate compact blocks + erasure-coded relay for low bandwidth regions; Phase 4 introduces gossip-level changes.

## 3. Operational Guidance
- **Reward schedule API:** `RewardSchedule::subsidy_at_height(height)` now returns the correct subsidy including tail emission. The `ConsensusEngine` exposes this via the block validation report.
- **Difficulty metadata:** `ConsensusParams` tracks `target_block_time` and `difficulty_half_life`. Consensus code still uses placeholder digest generation—full difficulty/retarget logic will land alongside Phase 4 validation.
- **ASERT helper:** `consensus::asert_next_target(anchor_target, height_delta, time_delta, params)` exposes the prototype retarget calculation (clamped to the Bitcoin-style max target constant).
- **Security testing:** PQC benchmarks must verify Dilithium verify throughput ≥5k sig/sec on reference hardware (8 cores) and confirm batch verification scaling with >1k signatures per block.
- **Next steps:**
  1. Finalise witness layout and rollup commitment format (Phase 4).
  2. Implement ASERT difficulty context and integrate with chainstate (Phase 5).
  3. Define PSBT extensions for PQC with explicit weight budgeting, aligning wallet estimators with the fee schedule above.
