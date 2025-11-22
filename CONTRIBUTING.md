# Contributing to BitQuan

We welcome contributions that help deliver a post-quantum secure blockchain. Before opening a pull request, please review the high-level workflow below and the detailed guidelines in [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md).

## Getting Started
1. Fork and clone the repository.
2. Install Rust stable (`rustup default stable`).
3. Run `cargo test` to ensure the workspace builds on your machine.
4. Configure reproducible builds per [`docs/REPRODUCIBILITY.md`](docs/REPRODUCIBILITY.md).

## Development Workflow
- Create feature branches from `main`.
- Follow the coding standards and security checklist described in `docs/CONTRIBUTING.md`.
- Ensure commits are GPG-signed (`git commit -S`).
- Include tests and documentation for new features.
- Run `scripts/pre-commit.sh` before opening a pull request to catch formatting, lint, and security issues early.

## Pull Requests
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before submitting.
- Reference related issues and provide context for reviewers.
- Expect at least two maintainer approvals before merging.

Thanks for helping secure BitQuan for the next 50+ years.

## Logging Security

**CRITICAL**: Never log sensitive data!

### ❌ Forbidden
- Private keys, passwords, mnemonics
- JWT tokens, API keys
- Any secret material

### [DONE] Safe Logging Patterns

```rust
// Use fingerprints for debugging
use crate::logging::fingerprint;
println!("Key loaded: {}", fingerprint(&key));

// Sanitize user input
use crate::logging::sanitize_for_log;
println!("User: {}", sanitize_for_log(&username));

// Mask secrets if needed
use crate::logging::mask_secret;
println!("Token: {}", mask_secret(&token, 4));
```

### Audit Before Commit

```bash
./scripts/audit-logs.sh
```

See [LOGGING_POLICY.md](docs/LOGGING_POLICY.md) for details.

## Code Structure & Naming Conventions

### File Naming

**[DONE] DO:**
- Use `snake_case.rs` for all Rust files
  - Examples: `transaction.rs`, `block_index.rs`, `tx_builder.rs`
- Use `lib.rs` for crate entry points
- Use `mod.rs` only for multi-file modules (prefer `module_name.rs` when possible)
- Use `*_tests.rs` for integration tests in `tests/` directory
  - Examples: `transaction_lifecycle_tests.rs`, `overflow_protection_tests.rs`

**❌ DON'T:**
- `CamelCase.rs` (incorrect)
- `kebab-case.rs` (incorrect for Rust)
- `test_*.rs` (use `*_tests.rs` instead)

### Module Organization

**Standard order in `lib.rs` or module files:**

```rust
// 1. Module declarations
mod transaction;
mod block;
mod error;

// 2. Re-exports (public API)
pub use transaction::{Transaction, TxIn, TxOut};
pub use block::Block;
pub use error::Error;

// 3. Internal modules (prefer pub(crate) for internals)
pub(crate) mod internal_utils;

// 4. Tests (at the end)
#[cfg(test)]
mod tests;
```

### Visibility Guidelines

**Use `pub(crate)` for internal APIs:**

```rust
// [DONE] Good - internal helper hidden from external users
pub(crate) fn internal_validation_helper(...) -> Result<()> { ... }

// [DONE] Good - public stable API
pub fn validate_transaction(...) -> Result<()> { ... }

// ❌ Bad - exposes internal details
pub fn internal_validation_helper(...) -> Result<()> { ... }
```

**Why?**
- Prevents users from depending on internal implementation details
- Easier to refactor without breaking external code
- Clearer separation between stable API and internals

### Crate Organization

Each crate should have a **single, clear responsibility**:

- `bitquan-types` - Core data structures only
- `bitquan-crypto` - Cryptographic primitives
- `bitquan-consensus` - Consensus rules and validation
- `bitquan-storage` - Database backend
- `bitquan-network` - P2P networking
- `bitquan-rpc` - JSON-RPC server
- `bitquan-mempool` - Transaction pool
- `bitquan-wallet` - Wallet operations
- `bitquan-node` - Main binary (orchestrator)

**Dependency Flow:** Always unidirectional (no circular dependencies)
```
types ← crypto ← consensus ← mempool ← node
types ← storage ← consensus ← rpc ← node
```

### API Stability

When adding new `pub` items, consult [docs/API_STABILITY.md](docs/API_STABILITY.md):
- Use `pub(crate)` for internal helpers within a crate
- Use `#[doc(hidden)]` for workspace-internal APIs
- Only mark items `pub` if they're part of the stable external API
- Document stability guarantees

For more details, see the Code Structure audit report in `docs/audit/`.
