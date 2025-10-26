# Installation Guide

This guide covers how to build and install BitQuan from source.

## Prerequisites

### System Requirements

- Operating System: Linux, macOS, or Windows (WSL2)
- RAM: 4GB minimum, 8GB recommended
- Disk Space: 10GB for build artifacts and blockchain data
- Network: Stable internet connection

### Required Software

- Rust 1.75 or later
- C/C++ compiler (gcc, clang, or MSVC)
- Git
- RocksDB development libraries

## Installing Dependencies

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    git \
    curl \
    librocksdb-dev \
    pkg-config \
    libssl-dev
```

### macOS

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install rocksdb pkg-config openssl
```

### Windows (WSL2)

```bash
# Install WSL2 Ubuntu
wsl --install -d Ubuntu

# Follow Ubuntu instructions above
```

## Installing Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version
```

## Building BitQuan

### Clone Repository

```bash
git clone https://github.com/alphab/BitQuan.git
cd BitQuan
```

### Build Release Binary

```bash
cargo build --release --features rocksdb-backend
```

Build artifacts will be in `target/release/bitquan-node`.

### Verify Installation

```bash
./target/release/bitquan-node --version
```

## Running Tests

```bash
# Run all tests
cargo test --all --features rocksdb-backend

# Run specific package tests
cargo test -p bitquan-consensus
cargo test -p bitquan-crypto
```

## Optional: Install System-Wide

```bash
# Copy binary to system path
sudo cp target/release/bitquan-node /usr/local/bin/

# Verify
bitquan-node --version
```

## Configuration

Create configuration directory:

```bash
mkdir -p ~/.bitquan
```

Default configuration file: `~/.bitquan/bitquan.conf`

## Troubleshooting

### RocksDB not found

If build fails with RocksDB errors:

```bash
# Ubuntu/Debian
sudo apt-get install librocksdb-dev

# macOS
brew install rocksdb

# Set PKG_CONFIG_PATH if needed
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH
```

### Linker errors

If you encounter linker errors:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

### Slow build times

Enable parallel compilation:

```bash
# Use all CPU cores
cargo build --release --features rocksdb-backend -j $(nproc)
```

## Next Steps

- [Quick Start Guide](QUICKSTART.md) - Start using BitQuan
- [Architecture Overview](../architecture/overview.md) - Understand the system
- [Contributing Guide](../../CONTRIBUTING.md) - Contribute to development
