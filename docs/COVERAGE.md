# Code Coverage

## Running Coverage Reports

This project uses `cargo-llvm-cov` for code coverage metrics.

### Installation

```bash
cargo install cargo-llvm-cov
```

### Generate HTML Report

```bash
cargo llvm-cov --workspace --html
```

The report will be generated in `./target/llvm-cov/html/index.html`.

### Generate Summary

```bash
cargo llvm-cov --workspace
```

### Continuous Integration

Coverage is tracked in CI and reports are generated for all PRs.

Target coverage thresholds:
- Core crates (consensus, wallet, mempool): > 80%
- Network/RPC crates: > 70%
- Utilities: > 60%
