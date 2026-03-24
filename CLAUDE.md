# BitQuan — Claude Code Guide

## Project Overview

BitQuan is a Rust blockchain implementation with C FFI dependencies (rocksdb, aws-lc-sys, ring).

## CI Architecture

### Workflows
- `ci.yml` — Main CI: clippy, format, tests, coverage, cargo-deny, fuzz build
- `full-matrix.yml` — Tests (3 OS), cross-compilation (musl/aarch64/wasm32), fuzz, audit
- `integration-tests.yml` — Multi-node, network, database, wallet, stress, security tests
- `security-scan.yml` — Optimized security scanning with coverage
- `rpc-tests.yml` — RPC endpoint tests
- `fast-pr.yml` — Quick PR validation

### Cross-Compilation Constraints

**This is critical. Do NOT add `cross` or QEMU for targets with C FFI deps.**

| Target | Tool | What works | What fails |
|--------|------|-----------|-------------|
| `x86_64-unknown-linux-musl` | `cross` | Pure Rust + rocksdb | — |
| `aarch64-unknown-linux-gnu` | `cargo build` (native) | Pure Rust + rocksdb | `cross` Docker (missing bindgen headers) |
| `wasm32-unknown-unknown` | `cargo build` | Pure Rust only | Any C FFI (rocksdb, aws-lc-sys, ring) |

**aarch64 env vars** (required for rocksdb bindgen):
```
CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
BINDGEN_EXTRA_CLANG_ARGS="--sysroot=/usr/aarch64-linux-gnu -I/usr/aarch64-linux-gnu/include"
```

**System packages needed**: `gcc-aarch64-linux-gnu g++-aarch64-linux-gnu clang libclang-dev`

### Crate FFI Dependency Map
```
bitquan-types      → pure Rust       → musl, aarch64, wasm32
bq-crypto          → pure Rust       → musl, aarch64, wasm32
bitquan-consensus  → rocksdb (C++)   → musl, aarch64  (NOT wasm32)
bitquan-mempool    → pure Rust       → musl, aarch64, wasm32
bitquan-network    → rocksdb (C++)   → musl, aarch64  (NOT wasm32)
bitquan-rpc        → rocksdb+ring+aws-lc-sys → musl, aarch64 (NOT wasm32)
wallet             → pure Rust       → musl, aarch64, wasm32
```

Run `cargo tree -p <crate> | grep -i "rocksdb\|aws-lc\|ring\|openssl-sys\|bindgen"` before attempting cross-compilation.

### Build Rules
- Always use separate `--target-dir` per target (musl/aarch64/wasm32) to prevent GLIBC contamination
- Never share `target/` between musl and gnu builds
- Multi-Platform Docker Build uses QEMU (2-4 hours, expected) — do not block on it

## Research References

When working on CI, cross-compilation, or build system issues, these learnings contain battle-tested solutions:

### Cross-Compilation
- `psi/memory/learnings/2026-03-24_rust-cross-compilation-guide.md` — Quick reference for env vars, sysroot setup, pre-build checklist
- `psi/memory/learnings/2026-03-24_aws-lc-rs-ci-techniques.md` — **Upstream C FFI reference**: 22 cross targets, wasm32 not supported by design
- `psi/memory/learnings/2026-03-24_polkadot-sdk-ci-techniques.md` — Polkadot uses custom CI image, no cross-compilation, artifact pipeline
- `psi/memory/learnings/2026-03-24_polkadot-sdk-ci-deep-dive.md` — **WASM runtime**: SRTOOL, SKIP_WASM_BUILD, try-runtime, profile.production
- `psi/memory/learnings/2026-03-24_agave-solana-ci-techniques.md` — Solana uses cargo-ndk for Android, native macOS for iOS, no QEMU
- `psi/memory/learnings/2026-03-24_solana-labs-ci-techniques.md` — Solana Labs release pipeline (S3 + GH Release)
- `psi/memory/learnings/2026-03-24_zcash-librustzcash-ci-techniques.md` — **Synthetic crate pattern** for wasm32/embedded targets
- `psi/memory/learnings/2026-03-24_rust-bitcoin-ci-techniques.md` — rust-bitcoin proves `cross` works for pure Rust, Kani/Miri/ASAN, cargo-semver-checks

### Enterprise CI
- `psi/memory/learnings/2026-03-24_aptos-labs-ci-techniques.md` — GCS Docker lock, Forge K8s testnet, 15+ label gates, custom runner specs

### Node & Database
- `psi/memory/learnings/2026-03-24_lighthouse-ci-techniques.md` — Reproducible builds, lockbud deadlock detection, Kurtosis testnet

### Key Insight from 9 Blockchain Projects
All 9 major Rust blockchain projects (Polkadot, Agave, Solana, Zcash, rust-bitcoin, Aptos, Lighthouse, aws-lc-rs) **do NOT use QEMU multi-platform Docker builds**. Projects with C FFI deps (Polkadot, Zcash) avoid cross-compilation entirely. rust-bitcoin uses `cross` successfully because it's pure Rust.

### wasm32 is NOT supported by aws-lc-sys (confirmed by upstream)
`bitquan-rpc` depends on `ring` -> `aws-lc-sys`. aws-lc-rs upstream CI has **zero** wasm32 targets. This is by design, not a bug. The correct approach: feature-gate crypto deps — use `ring`/`aws-lc-sys` for native targets, pure-Rust crypto for wasm32. See `psi/memory/learnings/2026-03-24_aws-lc-rs-ci-techniques.md`.

### aarch64: aws-lc-rs uses `cross-rs` successfully
aws-lc-sys upstream supports `aarch64-unknown-linux-gnu` via `cross test --target aarch64-unknown-linux-gnu` with no manual env vars. Our native toolchain approach also works but `cross-rs` is the upstream-preferred method.

### Enterprise CI Patterns (from Aptos Labs)
- **Label-based CI gates**: `CICD:run-e2e-tests`, `CICD:build-images` — granular control per PR
- **GCS Docker build lock**: deduplicate concurrent Docker builds
- **`CARGO_INCREMENTAL=0`**: disable incremental compilation in CI for correctness
- **`pull_request_target` + permission guard**: fork PRs access secrets safely

### Node & Database Patterns (from Lighthouse)
- **Reproducible build**: build binary twice, compare SHA256 — blocks release if different
- **lockbud deadlock detection**: MIRAI static analysis for async deadlocks
- **Kurtosis + Assertoor**: spin up full testnet in CI, run assertions against it

### WASM Runtime Patterns (from Polkadot SDK)
- **SRTOOL pipeline**: Docker with optimized Rust+WASM toolchain for on-chain runtime
- **`SKIP_WASM_BUILD=1`**: skip WASM compilation for clippy/check (saves 5-15 min)
- **`profile.production`**: `lto = true` + `codegen-units = 1` for WASM size optimization
- **try-runtime**: test migrations against live chain snapshots

### TOML Section Ordering
`[target.'cfg(...)'.dependencies]` absorbs ALL subsequent entries until the next section header. Always put `[dependencies]` BEFORE `[target.'cfg(...)'.dependencies]`.

### GitHub Actions Permissions
- `pull-requests: write` needed for `createComment` (separate from `issues: write`)
