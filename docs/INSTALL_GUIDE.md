# BitQuan v1.0.0 Installation Guide

## 🚀 Quick Start

```bash
# One-line installation (Linux x86_64)
curl --proto '=https' --tlsv1.2 -sSf https://install.bitquan.org | sh

# Start mainnet node
./bitquan-node --network mainnet --enable-stratum --dashboard-port 8080
```

---

## 📋 Prerequisites

### System Requirements
- **OS**: Linux (Ubuntu 20.04+, Debian 11+, CentOS 8+), macOS 10.15+, Windows 10+
- **CPU**: 4+ cores, 2.4GHz+ (x86_64 or ARM64)
- **Memory**: 8GB+ RAM (16GB+ recommended)
- **Storage**: 100GB+ SSD (500GB+ NVMe recommended)
- **Network**: 10Mbps+ broadband (100Mbps+ recommended)

### Required Software
- **Rust**: 1.79.0 or newer
- **OpenSSL**: 1.1.1+ (for cryptographic operations)
- **Git**: 2.0+ (for source builds)
- **CMake**: 3.10+ (for some dependencies)

---

## 🔧 Installation Methods

### Method 1: Binary Release (Recommended)

#### Linux
```bash
# Download latest release
wget https://github.com/bitquan/bitquan/releases/download/v1.0.0/bitquan-mainnet-linux-x86_64.tar.gz

# Verify GPG signature
wget https://github.com/bitquan/bitquan/releases/download/v1.0.0/bitquan-mainnet-linux-x86_64.tar.gz.asc
gpg --verify bitquan-mainnet-linux-x86_64.tar.gz.asc

# Extract and install
tar -xzf bitquan-mainnet-linux-x86_64.tar.gz
sudo cp bitquan-node bitquan-wallet /usr/local/bin/
sudo chmod +x /usr/local/bin/bitquan-*
```

#### macOS
```bash
# Download for macOS
wget https://github.com/bitquan/bitquan/releases/download/v1.0.0/bitquan-mainnet-darwin-x86_64.tar.gz

# Verify and extract
gpg --verify bitquan-mainnet-darwin-x86_64.tar.gz.asc
tar -xzf bitquan-mainnet-darwin-x86_64.tar.gz
sudo cp bitquan-node bitquan-wallet /usr/local/bin/
```

#### Windows
```powershell
# Download using PowerShell
Invoke-WebRequest -Uri "https://github.com/bitquan/bitquan/releases/download/v1.0.0/bitquan-mainnet-windows-x86_64.zip" -OutFile "bitquan-mainnet-windows-x86_64.zip"

# Extract (using built-in tar)
tar -xf bitquan-mainnet-windows-x86_64.zip

# Add to PATH or copy to desired location
copy bitquan-node.exe C:\Windows\System32\
copy bitquan-wallet.exe C:\Windows\System32\
```

### Method 2: Build from Source

#### Install Rust
```bash
# Install Rust 1.79.0+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify version
rustc --version  # Should be 1.79.0 or newer
```

#### Clone and Build
```bash
# Clone repository
git clone https://github.com/bitquan/bitquan.git
cd bitquan

# Checkout mainnet tag
git checkout v1.0.0

# Build release version
cargo build --release --locked

# Verify build
./target/release/bitquan-node --version
./target/release/bitquan-wallet --version
```

#### Install System Dependencies
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev cmake

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel cmake

# macOS (using Homebrew)
brew install openssl cmake

# Windows (using Chocolatey)
choco install openssl visualstudio2019buildtools
```

---

## ⚙️ Configuration

### Create Configuration File

Create `/etc/bitquan/mainnet.toml` (Linux/macOS) or `C:\ProgramData\BitQuan\mainnet.toml` (Windows):

```toml
[network]
magic = 0xe8f3e1e3  # Mainnet magic bytes
name = "mainnet"
data_dir = "/var/lib/bitquan"  # Linux
# data_dir = "/Users/Shared/BitQuan"  # macOS
# data_dir = "C:\ProgramData\BitQuan"  # Windows

[consensus]
algorithm = "randomx"
difficulty_adjustment_period = 2016  # ~2 weeks
max_block_size = 2_000_000  # 2MB
min_tx_fee = 1000  # 0.00001 BQ

[rpc]
enable = true
bind = "0.0.0.0:8332"
username = "your_rpc_user"
password = "secure_rpc_password_here"
cors_domains = ["*"]  # Adjust for security

[mining]
enable = false  # Set to true for mining
threads = 0  # 0 = auto-detect cores
stratum_port = 3333

[storage]
cache_size = "256MB"
max_open_files = 1000
compression = "lz4"

[metrics]
enable = true
bind = "0.0.0.0:9090"
namespace = "bitquan"

[logging]
level = "info"
file = "/var/log/bitquan/node.log"
max_size = "100MB"
max_files = 10
```

### Security Configuration

#### Firewall Setup
```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 8333/tcp  # P2P
sudo ufw allow 8332/tcp  # RPC (if needed)
sudo ufw allow 3333/tcp  # Stratum (if mining)
sudo ufw allow 9090/tcp  # Metrics (if needed)

# CentOS/RHEL (firewalld)
sudo firewall-cmd --permanent --add-port=8333/tcp
sudo firewall-cmd --permanent --add-port=8332/tcp
sudo firewall-cmd --permanent --add-port=3333/tcp
sudo firewall-cmd --reload
```

#### User and Permissions
```bash
# Create dedicated user
sudo useradd -r -s /bin/false bitquan
sudo mkdir -p /var/lib/bitquan /var/log/bitquan /etc/bitquan
sudo chown bitquan:bitquan /var/lib/bitquan /var/log/bitquan /etc/bitquan

# Secure configuration
sudo chmod 600 /etc/bitquan/mainnet.toml
sudo chmod 700 /var/lib/bitquan
```

---

## 🚀 Running BitQuan

### Command Line Options

```bash
# Show all options
./bitquan-node --help

# Start mainnet node
./bitquan-node --config /etc/bitquan/mainnet.toml

# Start with specific features
./bitquan-node --network mainnet --enable-stratum --dashboard-port 8080

# Run in background
nohup ./bitquan-node --config /etc/bitquan/mainnet.toml > /dev/null 2>&1 &

# Debug mode
./bitquan-node --config /etc/bitquan/mainnet.toml --log-level debug
```

### Systemd Service (Linux)

Create `/etc/systemd/system/bitquan.service`:

```ini
[Unit]
Description=BitQuan Mainnet Node
After=network.target

[Service]
User=bitquan
Group=bitquan
ExecStart=/usr/local/bin/bitquan-node --config /etc/bitquan/mainnet.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/bitquan /var/log/bitquan
MemoryDenyWriteExecute=true
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

[Install]
WantedBy=multi-user.target
```

Enable and start service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable bitquan
sudo systemctl start bitquan
sudo systemctl status bitquan
```

---

## 👛 Wallet Setup

### Generate New Wallet
```bash
# Generate new wallet
./bitquan-wallet generate --network mainnet

# Output will show:
# Address: bq1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
# Private key: [RECOVERY PHRASE WORDS]
# Write down recovery phrase and store securely!
```

### Restore Existing Wallet
```bash
# Restore from recovery phrase
./bitquan-wallet restore --network mainnet

# Input your 12/24 word recovery phrase when prompted
```

### Wallet Security Best Practices
```bash
# Encrypt wallet with passphrase
./bitquan-wallet encrypt --network mainnet

# Backup wallet file
cp ~/.bitquan/wallet.dat /secure/backup/wallet-$(date +%Y%m%d).dat

# Test backup on separate machine
./bitquan-wallet --test-backup /secure/backup/wallet-YYYYMMDD.dat
```

---

## ⛏️ Mining Configuration

### Solo Mining
```bash
# Start node with mining enabled
./bitquan-node --config /etc/bitquan/mainnet.toml --enable-mining --mining-threads 8

# Or enable in config:
[mining]
enable = true
threads = 8
```

### Pool Mining
```bash
# Configure mining pool in config
[mining.pool]
url = "stratum+tcp://pool.bitquan.org:3333"
username = "your_wallet_address"
password = "x"

# Start with pool mining
./bitquan-node --config /etc/bitquan/mainnet.toml --enable-mining --pool-mining
```

### Mining Software
```bash
# Using xmrig (RandomX compatible)
xmrig -o stratum+tcp://pool.bitquan.org:3333 -u your_wallet_address -p x --randomx

# Monitor mining performance
./bitquan-cli getmininginfo
```

---

## 🔒 Security Best Practices

### Memory Security
```bash
# Lock sensitive data in memory (requires root)
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.conf

# Configure mlock for wallet memory
ulimit -l unlimited  # Allow memory locking
```

### Network Security
```bash
# Use Tor for privacy (optional)
./bitquan-node --config /etc/bitquan/mainnet.toml --proxy socks5://127.0.0.1:9050

# Rate limit RPC requests
[rpc]
max_connections = 100
request_timeout = 30
rate_limit = 10  # requests per second
```

### Key Management
```bash
# Generate offline keys (air-gapped)
./bitquan-wallet generate --offline --network mainnet

# Store keys in hardware wallet
./bitquan-wallet export --hardware --network mainnet

# Use hardware security module (HSM)
./bitquan-node --config /etc/bitquan/mainnet.toml --hsm-device /dev/hsm0
```

---

## 📊 Monitoring and Maintenance

### Health Checks
```bash
# Check node status
curl http://localhost:9090/health

# Get blockchain info
./bitquan-cli getblockchaininfo

# Check peer connections
./bitquan-cli getpeerinfo

# Monitor logs
tail -f /var/log/bitquan/node.log
```

### Metrics Collection
```bash
# Prometheus metrics endpoint
curl http://localhost:9090/metrics

# Grafana dashboard setup
# Add Prometheus data source: http://localhost:9090
# Import BitQuan dashboard JSON
```

### Performance Tuning
```bash
# Optimize for high-performance
[storage]
cache_size = "1GB"  # Increase if RAM available
max_open_files = 10000  # Increase for high I/O

[network]
max_peers = 200  # Increase peer connections
outbound_connections = 8  # Optimize bandwidth
```

---

## 🔧 Troubleshooting

### Common Issues

#### Build Failures
```bash
# Clear cargo cache
cargo clean

# Update dependencies
cargo update

# Force rebuild
cargo build --release --locked
```

#### Runtime Issues
```bash
# Check configuration
./bitquan-node --config /etc/bitquan/mainnet.toml --check

# Verify permissions
ls -la /var/lib/bitquan/
ls -la /etc/bitquan/

# Check system resources
free -h
df -h /var/lib/bitquan
```

#### Network Issues
```bash
# Test connectivity
nc -zv peer.bitquan.org 8333

# Check DNS resolution
nslookup dns-seed.bitquan.org

# Verify firewall
sudo ufw status verbose
```

#### Sync Issues
```bash
# Reset sync state
./bitquan-cli reconsiderblock "latest_block_hash"

# Reindex blockchain
./bitquan-node --config /etc/bitquan/mainnet.toml --reindex

# Add bootstrap peers
./bitquan-cli addnode "bootstrap.bitquan.org:8333" "add"
```

### Log Analysis
```bash
# View recent errors
grep ERROR /var/log/bitquan/node.log | tail -20

# Monitor performance
grep "block\|tx\|peer" /var/log/bitquan/node.log | tail -10

# Check for security issues
grep "auth\|permission\|denied" /var/log/bitquan/node.log
```

---

## 📚 Additional Resources

### Documentation
- **[API Reference](rpc/API_REFERENCE.md)**: Complete RPC API documentation
- **[BQIPs](bqip/)**: BitQuan Improvement Proposals
- **[Security Guide](security/README.md)**: Security best practices
- **[Mining Guide](guides/STRATUM.md)**: Detailed mining information

### Tools
- **bitquan-cli**: Command-line interface for node management
- **bitquan-wallet**: Wallet management utility
- **bq-stress**: Load testing and benchmarking tool
- **bq-preflight**: Pre-launch validation tool

### Community
- **Discord**: https://discord.gg/bitquan
- **GitHub**: https://github.com/bitquan/bitquan
- **Documentation**: https://docs.bitquan.org
- **Explorer**: https://explorer.bitquan.org

---

## ✅ Verification

After installation, verify everything is working:

```bash
# 1. Check node version
./bitquan-node --version  # Should show v1.0.0

# 2. Verify configuration
./bitquan-node --config /etc/bitquan/mainnet.toml --check

# 3. Test RPC connectivity
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}' \
  http://localhost:8332/

# 4. Check wallet functionality
./bitquan-wallet getaddress --network mainnet

# 5. Verify network connectivity
./bitquan-cli getpeerinfo
```

---

## 🆘 Support

### Getting Help
- **Documentation**: https://docs.bitquan.org
- **GitHub Issues**: https://github.com/bitquan/bitquan/issues
- **Community**: https://discord.gg/bitquan
- **Security**: security@bitquan.org

### Reporting Issues
When reporting issues, include:
- BitQuan version (`./bitquan-node --version`)
- Operating system and architecture
- Configuration file (sanitized)
- Error logs
- Steps to reproduce

---

**🎉 Congratulations! You've successfully installed BitQuan v1.0.0 mainnet!**

Join the post-quantum cryptocurrency revolution and help secure the future of digital money.

**Welcome to BitQuan! 🚀**