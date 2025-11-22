# BitQuan Mainnet Installation Guide

## Overview

BitQuan is a post-quantum cryptocurrency with lattice-based cryptography (Dilithium3) and hybrid consensus. This guide covers mainnet deployment for production use.

## System Requirements

### Minimum Requirements
- **CPU**: 4 cores, 2.4GHz+ (x86_64 or ARM64)
- **Memory**: 8GB RAM
- **Storage**: 100GB SSD (NVMe recommended)
- **Network**: 10Mbps+ broadband with stable connection

### Recommended Requirements
- **CPU**: 8 cores, 3.0GHz+ (modern x86_64)
- **Memory**: 16GB RAM
- **Storage**: 500GB NVMe SSD
- **Network**: 100Mbps+ fiber connection

## Installation

### Method 1: Binary Release (Recommended)

```bash
# Download latest mainnet release
wget https://github.com/bitquan/bitquan/releases/latest/download/bitquan-mainnet-linux-x86_64.tar.gz

# Verify signature
wget https://github.com/bitquan/bitquan/releases/latest/download/bitquan-mainnet-linux-x86_64.tar.gz.asc
gpg --verify bitquan-mainnet-linux-x86_64.tar.gz.asc

# Extract and install
tar -xzf bitquan-mainnet-linux-x86_64.tar.gz
sudo cp bitquan-node /usr/local/bin/
sudo cp bitquan-wallet /usr/local/bin/
```

### Method 2: Build from Source

```bash
# Install Rust 1.79+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/bitquan/bitquan.git
cd bitquan
git checkout v1.0.0  # Mainnet tag
cargo build --release

# Install binaries
sudo cp target/release/bitquan-node /usr/local/bin/
sudo cp target/release/bitquan-wallet /usr/local/bin/
```

## Configuration

### Mainnet Configuration

Create `/etc/bitquan/mainnet.toml`:

```toml
[network]
magic = 0xe8f3e1e3  # Mainnet magic bytes
name = "mainnet"

[consensus]
algorithm = "randomx"
difficulty_adjustment_period = 2016  # ~2 weeks
max_block_size = 2_000_000  # 2MB
min_tx_fee = 1000  # 0.00001 BQ

[rpc]
enable = true
bind = "0.0.0.0:8332"
username = "your_rpc_user"
password = "secure_rpc_password"

[mining]
enable = false  # Set to true for mining
threads = 0  # 0 = auto-detect

[storage]
data_dir = "/var/lib/bitquan"
cache_size = "256MB"
```

### Firewall Configuration

```bash
# Open P2P port
sudo ufw allow 8333/tcp

# Open RPC port (if remote access needed)
sudo ufw allow 8332/tcp

# Open Stratum port (if mining)
sudo ufw allow 3333/tcp
```

## Service Setup

### Systemd Service

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

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/bitquan

[Install]
WantedBy=multi-user.target
```

```bash
# Create user and directories
sudo useradd -r -s /bin/false bitquan
sudo mkdir -p /var/lib/bitquan
sudo chown bitquan:bitquan /var/lib/bitquan

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable bitquan
sudo systemctl start bitquan
```

## Initial Sync

### Bootstrap Peers

Add trusted peers to speed up initial sync:

```bash
# Edit configuration
sudo nano /etc/bitquan/mainnet.toml

# Add bootstrap nodes
[network.p2p]
bootstrap_peers = [
    "mainnet1.bitquan.org:8333",
    "mainnet2.bitquan.org:8333",
    "mainnet3.bitquan.org:8333"
]
```

### Monitor Sync Progress

```bash
# Check sync status
bitquan-cli getblockchaininfo

# Monitor logs
sudo journalctl -u bitquan -f
```

Initial sync typically takes 6-12 hours depending on network speed and hardware.

## Security Hardening

### 1. GPG Verification

Always verify releases with GPG:

```bash
# Import BitQuan signing key
gpg --keyserver hkps://keys.openpgp.org --recv-keys 2B8F1E9C

# Verify release signature
gpg --verify bitquan-mainnet-linux-x86_64.tar.gz.asc
```

### 2. Network Security

```bash
# Configure fail2ban
sudo apt install fail2ban
sudo systemctl enable fail2ban

# Rate limit RPC requests
sudo ufw limit 8332/tcp
```

### 3. File Permissions

```bash
# Secure configuration files
sudo chmod 600 /etc/bitquan/mainnet.toml
sudo chown bitquan:bitquan /etc/bitquan/mainnet.toml

# Secure data directory
sudo chmod 700 /var/lib/bitquan
sudo chown bitquan:bitquan /var/lib/bitquan
```

## Monitoring

### Health Checks

```bash
# Node status
bitquan-cli getnetworkinfo

# Block height
bitquan-cli getblockcount

# Peer connections
bitquan-cli getpeerinfo

# Mempool size
bitquan-cli getmempoolinfo
```

### Metrics Endpoint

Enable Prometheus metrics:

```toml
[metrics]
enable = true
bind = "0.0.0.0:9100"
```

Access metrics at `http://your-node:9100/metrics`

## Troubleshooting

### Common Issues

1. **Sync Stuck**
   ```bash
   # Check peer connections
   bitquan-cli getpeerinfo | jq '.length'

   # Add more peers if needed
   bitquan-cli addnode "peer.ip:8333" "add"
   ```

2. **High Memory Usage**
   ```bash
   # Reduce cache size in config
   [storage]
   cache_size = "128MB"
   ```

3. **RPC Connection Failed**
   ```bash
   # Check service status
   sudo systemctl status bitquan

   # Verify RPC credentials
   bitquan-cli -rpcuser=user -rpcpassword=pass getblockchaininfo
   ```

### Log Analysis

```bash
# View recent errors
sudo journalctl -u bitquan --since "1 hour ago" | grep ERROR

# Monitor performance
sudo journalctl -u bitquan --since "1 hour ago" | grep -E "(sync|block|tx)"
```

## Backup and Recovery

### Wallet Backup

```bash
# Backup wallet
bitquan-wallet backup /path/to/backup/wallet.dat

# Export keys for cold storage
bitquan-wallet dumpwallet /path/to/backup/keys.txt
```

### Node Data Backup

```bash
# Stop node
sudo systemctl stop bitquan

# Backup blockchain data
sudo tar -czf /backup/bitquan-$(date +%Y%m%d).tar.gz /var/lib/bitquan

# Restart node
sudo systemctl start bitquan
```

## Upgrades

### Upgrade Procedure

1. **Stop node**: `sudo systemctl stop bitquan`
2. **Backup data**: See backup section above
3. **Install new version**: Follow installation steps
4. **Verify configuration**: Check for new config options
5. **Start node**: `sudo systemctl start bitquan`
6. **Monitor**: Check logs for any issues

## Support

- **Documentation**: https://docs.bitquan.org
- **GitHub Issues**: https://github.com/bitquan/bitquan/issues
- **Community**: https://discord.gg/bitquan
- **Security**: security@bitquan.org

## Next Steps

After installation, consider:
- Setting up monitoring alerts
- Configuring backup automation
- Joining mining pools if mining
- Setting up additional nodes for redundancy
