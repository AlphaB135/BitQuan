# Development Documentation

**Last Updated: 2025-01-07**

This section contains guides for building, testing, and contributing to BitQuan.

## 🛠️ Building from Source

### Prerequisites

- Rust 1.75+ (MSRV)
- Cargo
- Git
- OpenSSL development headers
- (Optional) RandomX dependencies for hybrid mining

### Quick Build

```bash
# Clone repository
git clone https://github.com/your-org/BitQuan.git
cd BitQuan

# Build release binary
cargo build --release

# Run tests
cargo test --all

# Binary location
./target/release/bitquan-node --version
```

### Feature Flags

```bash
# Build with RandomX support (testnet hybrid mining)
cargo build --release --features randomx

# Build without RPC (minimal)
cargo build --release --no-default-features

# All features
cargo build --release --all-features
```

## 🧪 Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test consensus::
cargo test wallet::

# With output
cargo test -- --nocapture

# Parallel with more threads
cargo test -- --test-threads=8
```

### Integration Tests

```bash
# Run integration tests
cargo test --test '*'

# Specific integration test
cargo test --test node_integration
```

### Fuzzing

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run fuzzer
cd fuzz
cargo fuzz run transaction_parser

# With timeout
cargo fuzz run block_validator -- -max_total_time=300
```

See [../fuzz/](../../fuzz/) for fuzzing targets.

### Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# View report
open coverage/index.html
```

See [COVERAGE.md](./COVERAGE.md) for coverage targets.

## 🔍 Code Quality

### Linting

```bash
# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Auto-fix
cargo clippy --fix --all-targets

# Strict mode
cargo clippy -- -W clippy::all -W clippy::pedantic
```

### Formatting

```bash
# Check formatting
cargo fmt -- --check

# Auto-format
cargo fmt

# Format specific file
rustfmt src/consensus/mod.rs
```

### Security Audit

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit

# Fix vulnerabilities
cargo audit fix
```

### Deny

```bash
# Install cargo-deny
cargo install cargo-deny

# Check licenses, bans, advisories
cargo deny check

# Update deny configuration
vi deny.toml
```

## 🏗️ Project Structure

```
BitQuan/
├── crates/           # Library crates
│   ├── consensus/    # Consensus logic
│   ├── crypto/       # PQC signatures
│   ├── network/      # P2P protocol
│   ├── storage/      # Database layer
│   └── wallet/       # Wallet operations
├── src/              # Main binary
├── tests/            # Integration tests
├── fuzz/             # Fuzzing targets
├── docs/             # Documentation
├── scripts/          # Build/deploy scripts
└── config/           # Configuration examples
```

## 📝 Code Style

### Conventions

- **Naming**: `snake_case` for functions/variables, `PascalCase` for types
- **Line length**: 100 characters max
- **Comments**: Use `///` for doc comments, `//` for inline
- **Errors**: Use `thiserror` for error types
- **Logging**: Use `tracing` crate (info!, debug!, warn!, error!)

### Panic Safety

See [PANIC_SAFETY.md](./PANIC_SAFETY.md) for guidelines.

**Rules**:
- ❌ No panics in production code paths
- ✅ Use `Result<T, E>` for fallible operations
- ✅ Use checked arithmetic (`checked_add`, `saturating_mul`, etc.)
- ✅ Validate inputs at boundaries
- ✅ Document panic conditions in `# Panics` section

### Logging Policy

See [LOGGING_POLICY.md](./LOGGING_POLICY.md) for log level guidelines.

```rust
use tracing::{debug, info, warn, error};

// ERROR: Critical failures requiring immediate attention
error!("Failed to connect to database: {}", err);

// WARN: Recoverable issues or degraded performance
warn!("Peer connection timeout, retrying...");

// INFO: Normal operations and milestones
info!("Block {} mined successfully", height);

// DEBUG: Detailed diagnostic information
debug!("Transaction validation took {}ms", duration);

// TRACE: Very verbose, development only
trace!("Processing input {}: {:?}", idx, input);
```

## 🔄 Git Workflow

### Branching

- `main` - Production-ready code
- `develop` - Integration branch for features
- `feature/xyz` - New features
- `fix/xyz` - Bug fixes
- `release/vX.Y.Z` - Release preparation

### Commit Messages

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`

**Examples**:
```
feat(wallet): add BIP39 mnemonic support

Implements 12/24 word mnemonic generation and recovery
using the BIP39 standard wordlist.

Closes #123
```

### Pre-Commit Hooks

```bash
# Install hooks
cp scripts/git-hooks/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit

# Hooks run automatically on commit
git commit -m "feat: add new feature"
```

Hooks check:
- ✅ Code formatting (`cargo fmt --check`)
- ✅ Clippy warnings (`cargo clippy`)
- ✅ Tests pass (`cargo test`)
- ✅ No debug prints or TODOs in staged files

## 🚀 Release Process

See [RELEASE.md](../guides/RELEASE.md) for complete release procedures.

### Version Bumping

```bash
# Update version in Cargo.toml
vi Cargo.toml

# Update CHANGELOG.md
vi CHANGELOG.md

# Commit
git commit -am "chore: bump version to v1.2.0"

# Tag
git tag -s v1.2.0 -m "Release v1.2.0"

# Push
git push origin main --tags
```

### Reproducible Builds

See [REPRODUCIBILITY.md](../security/REPRODUCIBILITY.md)

```bash
# Build reproducibly
./scripts/build_reproducible.sh v1.2.0

# Verify
./scripts/verify_release.sh v1.2.0
```

## 🤝 Contributing

See [CONTRIBUTING.md](../guides/CONTRIBUTING.md) for contribution guidelines.

### Quick Checklist

- [ ] Fork repository and create feature branch
- [ ] Write tests for new functionality
- [ ] Run `cargo test`, `cargo clippy`, `cargo fmt`
- [ ] Update documentation if needed
- [ ] Add entry to CHANGELOG.md (Unreleased section)
- [ ] Submit pull request with clear description
- [ ] Respond to review feedback

## 📚 Related Documentation

- [CLI Tools](../cli/) - Command reference
- [Operations](../ops/) - Deployment guides
- [Security](../security/) - Security policies

---

*Updated on: 2025-01-07*
