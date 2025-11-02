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

### ✅ Safe Logging Patterns

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

