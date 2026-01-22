# Fuzzing Targets

This directory contains fuzzing targets for BitQuan using cargo-fuzz and libFuzzer.

## Prerequisites

```bash
# Install nightly toolchain
rustup install nightly

# Install cargo-fuzz
cargo install cargo-fuzz
```

**Important**: Fuzzing requires nightly Rust. If your project uses `rust-toolchain.toml` to pin stable, you may need to:
1. Temporarily rename `rust-toolchain.toml` before building fuzzers
2. Use `cargo +nightly` explicitly for all fuzz commands
3. Set a local override: `cd fuzz && rustup override set nightly`

## Available Targets

- **fuzz_transaction** - Fuzzes transaction parsing, validation, and weight calculation
- **fuzz_block** - Fuzzes block header parsing, merkle root calculation, and block validation
- **fuzz_script** - Fuzzes script interpreter execution with arbitrary bytecode
- **fuzz_mempool** - Fuzzes mempool operations (insert, eviction, fee sorting)
- **fuzz_keystore_json** - Fuzzes keystore JSON parsing and serialization (CRITICAL - handles private keys)
- **fuzz_consensus** - Fuzzes consensus validation logic
- **fuzz_crypto** - Fuzzes cryptographic operations
- **fuzz_network** - Fuzzes P2P network message parsing
- **fuzz_pow** - Fuzzes proof-of-work validation
- **fuzz_wire** - Fuzzes wire protocol serialization
- **fuzz_asert** - Fuzzes ASERT (difficulty adjustment) calculation
- **fuzz_address** - Fuzzes address parsing and validation

## Running Fuzz Tests

```bash
# List all available fuzz targets
cargo fuzz list

# Run a specific target (runs indefinitely until crash or Ctrl+C)
cargo +nightly fuzz run fuzz_transaction

# Run with limited runs
cargo +nightly fuzz run fuzz_transaction -- -runs=10000

# Run with time limit (in seconds)
cargo +nightly fuzz run fuzz_transaction -- -max_total_time=60

# Run with multiple jobs (parallel fuzzing)
cargo +nightly fuzz run fuzz_transaction -- -jobs=4

# Run with specific seed corpus
cargo +nightly fuzz run fuzz_transaction fuzz/corpus/fuzz_transaction
```

## Continuous Integration

Fuzz targets are built (but not run) in CI to ensure they compile correctly:

```yaml
- name: Build fuzz targets
  run: cargo +nightly fuzz build
```

## Finding and Reproducing Crashes

If a crash is found, the input is saved to `fuzz/artifacts/<target>/`:

```bash
# Reproduce a crash
cargo +nightly fuzz run fuzz_transaction fuzz/artifacts/fuzz_transaction/crash-<hash>

# Debug a crash
cargo +nightly fuzz run fuzz_transaction --debug-assertions fuzz/artifacts/fuzz_transaction/crash-<hash>
```

## Coverage

Generate coverage report for fuzzing:

```bash
cargo fuzz coverage fuzz_transaction
cargo cov -- show target/*/release/fuzz_transaction \
    --format=html \
    --instr-profile=fuzz/coverage/fuzz_transaction/coverage.profdata \
    > coverage.html
```

## Best Practices

1. **Run regularly**: Integrate into your development workflow
2. **Use corpus**: Maintain seed inputs in `fuzz/corpus/` for better coverage
3. **Time limits**: Use `-max_total_time` for quick sanity checks
4. **Parallel jobs**: Use `-jobs` to utilize all CPU cores
5. **Minimize inputs**: Use `cargo fuzz tmin` to reduce crash inputs to minimal size

## Critical Security Targets

The following fuzzers handle security-sensitive data and should be prioritized:

1. **fuzz_keystore_json** (CRITICAL)
   - Handles private keys and wallet data
   - Tests JSON parsing from untrusted sources
   - Validates serialization round-trips don't leak data
   - Run command: `cargo +nightly fuzz run fuzz_keystore_json -- -runs=100000`

2. **fuzz_crypto** (HIGH)
   - Tests cryptographic operations
   - Validates key handling and signature operations
   - Ensures no panic on malformed inputs

3. **fuzz_consensus** (HIGH)
   - Tests blockchain consensus logic
   - Critical for preventing consensus bugs
   - Validates block and transaction acceptance rules

## Resources

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [Fuzzing in Rust](https://rust-fuzz.github.io/book/)
