# Reproducible Builds

BitQuan supports deterministic, reproducible builds to ensure binary transparency and verifiability.

## Requirements

- Rust toolchain: **1.82.0** (stable)
- Cargo version: matches Rust 1.82.0
- Target: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`
- `SOURCE_DATE_EPOCH=1700000000` (fixed timestamp for reproducibility)

## Build Process

### Standard Reproducible Build

```bash
export SOURCE_DATE_EPOCH=1700000000
cargo build --release --locked
```

### Cross-Platform Builds

Linux:
```bash
SOURCE_DATE_EPOCH=1700000000 cargo build --release --locked --target x86_64-unknown-linux-gnu
```

macOS:
```bash
SOURCE_DATE_EPOCH=1700000000 cargo build --release --locked --target x86_64-apple-darwin
```

Windows:
```bash
set SOURCE_DATE_EPOCH=1700000000
cargo build --release --locked --target x86_64-pc-windows-msvc
```

## Verification

Generate checksums:
```bash
sha256sum target/release/bitquan-node > checksums-sha256.txt
sha512sum target/release/bitquan-node > checksums-sha512.txt
```

Compare with official release checksums to verify build integrity.

## Deterministic Flags

BitQuan builds use:
- `--locked` to pin exact dependency versions from Cargo.lock
- `SOURCE_DATE_EPOCH` for timestamp normalization
- Release mode optimizations (`--release`)

## Notes

- Builds must use the **exact** Rust version specified
- The `Cargo.lock` file must not be modified
- Different host OS or toolchain versions may produce different binaries
- For maximum reproducibility, use the same OS as the official build environment

## Official Build Environment

Official releases are built on:
- Ubuntu 22.04 LTS (Linux)
- macOS 13 (Ventura)
- Windows Server 2022

See `.github/workflows/release.yml` for exact CI configuration.

## Further Reading

- [docs/security/REPRODUCIBILITY.md](docs/security/REPRODUCIBILITY.md) - Detailed security notes
- [RELEASE.md](RELEASE.md) - Release process documentation
