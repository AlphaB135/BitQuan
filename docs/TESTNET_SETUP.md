# 🧪 BitQuan Testnet Setup Guide

Complete guide to set up and run BitQuan testnet for public testing.

## 📋 Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Server Setup](#server-setup)
4. [Node Configuration](#node-configuration)
5. [Mining Pool Setup](#mining-pool-setup)
6. [Wallet & Faucet](#wallet--faucet)
7. [Monitoring](#monitoring)
8. [For Testers](#for-testers)

---

## 🔧 Prerequisites

### System Requirements
- **OS**: Linux (Ubuntu 22.04+ recommended)
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 50GB SSD
- **Network**: Static IP, ports 8333, 8334, 3333 open

### Software
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y build-essential curl git pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

## 🚀 Quick Start

### 1. Clone Repository
```bash
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
git checkout v1.0.0
```

### 2. Build Node
```bash
cargo build --release --bin bitquan-node
```

### 3. Initialize Testnet
```bash
# Create testnet directory
mkdir -p ~/.bitquan/testnet

# Copy testnet genesis
cp genesis/testnet.json ~/.bitquan/testnet/genesis.json

# Copy testnet config
cp config/testnet.toml ~/.bitquan/testnet/config.toml
```

### 4. Start Node
```bash
./target/release/bitquan-node \
  --network testnet \
  --data-dir ~/.bitquan/testnet \
  --rpc-port 8334 \
  --p2p-port 8333
```

---

## 🖥️ Server Setup

### Create Service User
```bash
sudo useradd -m -s /bin/bash bitquan
sudo mkdir -p /opt/bitquan
sudo chown bitquan:bitquan /opt/bitquan
```

### Install Binary
```bash
# Copy built binary
sudo cp target/release/bitquan-node /opt/bitquan/bin/
sudo chmod +x /opt/bitquan/bin/bitquan-node

# Set ownership
sudo chown -R bitquan:bitquan /opt/bitquan
```

### Create Systemd Service
```bash
sudo tee /etc/systemd/system/bitquan-testnet.service > /dev/null << 'EOF'
[Unit]
Description=BitQuan Testnet Node
After=network.target

[Service]
Type=simple
User=bitquan
WorkingDirectory=/opt/bitquan
ExecStart=/opt/bitquan/bin/bitquan-node \
  --network testnet \
  --data-dir /opt/bitquan/data/testnet \
  --rpc-port 8334 \
  --p2p-port 8333 \
  --rpc-bind 0.0.0.0 \
  --mining-address tBQ1... \
  --enable-stratum \
  --stratum-port 3333

Restart=always
RestartSec=10

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/bitquan/data

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable bitquan-testnet
sudo systemctl start bitquan-testnet
```

### Check Status
```bash
# View logs
sudo journalctl -u bitquan-testnet -f

# Check status
sudo systemctl status bitquan-testnet

# Check ports
sudo ss -tulpn | grep bitquan
```

---

## ⚙️ Node Configuration

### Edit Config File
```bash
sudo -u bitquan nano /opt/bitquan/data/testnet/config.toml
```

### Example Testnet Config
```toml
[network]
network_id = "testnet"
p2p_port = 8333
max_peers = 50

# Seed nodes (add your bootstrap nodes)
bootstrap_nodes = [
    "testnet-seed1.bitquan.io:8333",
    "testnet-seed2.bitquan.io:8333",
]

[rpc]
enabled = true
bind = "0.0.0.0"
port = 8334
# Generate JWT secret: openssl rand -hex 32
jwt_secret = "your-secret-here"
max_connections = 100
require_auth = true

[mining]
enabled = false
# Set your testnet mining address
address = "tBQ1xxxxxxxxxxxxxxxxxxxxxxxxxxxx"

[pool]
enabled = true
bind = "0.0.0.0"
stratum_port = 3333
difficulty = 1000
vardiff_min = 100
vardiff_max = 10000

[consensus]
# Testnet uses lower difficulty
min_difficulty_bits = 0x1d00ffff

[storage]
db_path = "/opt/bitquan/data/testnet/chaindata"
cache_size_mb = 512
```

---

## ⛏️ Mining Pool Setup

### Enable Stratum Server
```bash
# Create pool wallet
./target/release/bitquan-node wallet create \
  --network testnet \
  --output /opt/bitquan/pool-wallet.keystore

# Get pool address
./target/release/bitquan-node wallet address \
  --keystore /opt/bitquan/pool-wallet.keystore
```

### Pool Configuration
Add to config.toml:
```toml
[pool]
enabled = true
name = "BitQuan Public Testnet Pool"
bind = "0.0.0.0"
stratum_port = 3333

# Pool wallet
address = "tBQ1_YOUR_POOL_ADDRESS"

# Difficulty settings
start_difficulty = 1000
vardiff_enabled = true
vardiff_min = 100
vardiff_max = 100000
vardiff_target_time = 15

# Payout settings
min_payout = 1000000  # 0.01 BQ
payout_interval = 3600  # 1 hour

# Dashboard
dashboard_enabled = true
dashboard_port = 8080
```

### Restart Node
```bash
sudo systemctl restart bitquan-testnet
```

### Check Pool Status
```bash
# Pool dashboard
curl http://localhost:8080/pool/stats

# Active miners
curl http://localhost:8080/pool/miners
```

---

## 💰 Wallet & Faucet

### Create Testnet Wallet
```bash
# For testers
./target/release/bitquan-node wallet create \
  --network testnet \
  --output my-testnet-wallet.keystore
```

### Get Wallet Address
```bash
./target/release/bitquan-node wallet address \
  --keystore my-testnet-wallet.keystore
```

### Setup Faucet (for distributing testnet coins)

Create faucet service:
```bash
sudo tee /etc/systemd/system/bitquan-faucet.service > /dev/null << 'EOF'
[Unit]
Description=BitQuan Testnet Faucet
After=network.target bitquan-testnet.service

[Service]
Type=simple
User=bitquan
WorkingDirectory=/opt/bitquan
ExecStart=/usr/bin/python3 /opt/bitquan/tools/testnet_faucet.py

Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
```

Update faucet config:
```python
# In tools/testnet_faucet.py
FAUCET_AMOUNT = 100_0000_0000  # 100 testnet BQ
COOLDOWN_HOURS = 24
RPC_URL = "http://localhost:8334"
FAUCET_WALLET = "/opt/bitquan/faucet-wallet.keystore"
```

---

## 📊 Monitoring

### Install Monitoring Tools
```bash
# Prometheus
sudo apt install -y prometheus

# Grafana
sudo apt install -y grafana
```

### Node Metrics Endpoint
```bash
# Enable metrics in config.toml
[metrics]
enabled = true
bind = "127.0.0.1"
port = 9090
```

### Check Metrics
```bash
curl http://localhost:9090/metrics
```

### Grafana Dashboard
1. Access: http://your-server:3000
2. Add Prometheus data source: http://localhost:9090
3. Import BitQuan dashboard from `monitoring/grafana/`

---

## 👥 For Testers

### Public Information to Share

#### **Testnet Node Info**
```
Network: BitQuan Testnet
RPC Endpoint: http://testnet.bitquan.io:8334
Mining Pool: stratum+tcp://testnet.bitquan.io:3333
Faucet: http://testnet.bitquan.io:5000
Explorer: http://explorer.testnet.bitquan.io
```

#### **Getting Started (for testers)**

1. **Download BitQuan Client**
   ```bash
   wget https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-linux-x86_64
   chmod +x bitquan-linux-x86_64
   ```

2. **Create Wallet**
   ```bash
   ./bitquan-linux-x86_64 wallet create --network testnet
   ```

3. **Get Testnet Coins**
   - Visit faucet: http://testnet.bitquan.io:5000
   - Enter your testnet address
   - Receive 100 testnet BQ

4. **Start Mining**
   ```bash
   # CPU Mining
   ./bitquan-linux-x86_64 mine \
     --pool stratum+tcp://testnet.bitquan.io:3333 \
     --address YOUR_TESTNET_ADDRESS \
     --threads 4
   ```

5. **Send Transaction**
   ```bash
   ./bitquan-linux-x86_64 send \
     --from YOUR_WALLET.keystore \
     --to RECIPIENT_ADDRESS \
     --amount 10.5
   ```

6. **Check Balance**
   ```bash
   ./bitquan-linux-x86_64 balance \
     --address YOUR_ADDRESS \
     --rpc http://testnet.bitquan.io:8334
   ```

### What to Test

✅ **Wallet Operations**
- Create/restore wallets
- Send/receive transactions
- HD wallet derivation
- Keystore encryption

✅ **Mining**
- Solo mining
- Pool mining
- Different algorithms (SHA256d, RandomX)
- Vardiff adjustment

✅ **Network**
- Peer discovery
- Block propagation
- Transaction relay
- Reorg handling

✅ **Smart Features**
- Multi-signature wallets
- Time-locked transactions
- Post-quantum signatures

✅ **Performance**
- Transaction throughput
- Block sync speed
- Memory usage
- CPU usage

### Report Issues
- GitHub Issues: https://github.com/AlphaB135/BitQuan/issues
- Discord: https://discord.gg/bitquan
- Telegram: https://t.me/bitquan_testnet

---

## 🔒 Security Notes

### Testnet Warnings
⚠️ **Testnet coins have NO value**
⚠️ **Network may be reset anytime**
⚠️ **Do NOT use mainnet keys on testnet**
⚠️ **Test in isolated environment first**

### Best Practices
- Use firewall rules
- Keep software updated
- Monitor logs regularly
- Backup wallet keystores
- Use strong passwords

---

## 🆘 Troubleshooting

### Node Won't Start
```bash
# Check logs
sudo journalctl -u bitquan-testnet -n 100

# Check ports
sudo netstat -tulpn | grep -E '8333|8334|3333'

# Check permissions
ls -la /opt/bitquan/data/testnet/
```

### Can't Connect to Peers
```bash
# Check bootstrap nodes
curl https://testnet-seeds.bitquan.io/peers.json

# Manually add peers
./bitquan-node --add-node testnet-seed1.bitquan.io:8333
```

### Mining Not Working
```bash
# Check pool connection
telnet testnet.bitquan.io 3333

# Check wallet address format
# Should start with 'tBQ1' for testnet

# Verify mining algorithm
# Use --algo sha256d or --algo randomx
```

### Database Corruption
```bash
# Stop node
sudo systemctl stop bitquan-testnet

# Rebuild from backup
sudo -u bitquan /opt/bitquan/bin/bitquan-node \
  --network testnet \
  --reindex

# Restart
sudo systemctl start bitquan-testnet
```

---

## 📞 Support

### Resources
- 📖 Documentation: https://docs.bitquan.io
- 💬 Discord: https://discord.gg/bitquan
- 🐦 Twitter: https://twitter.com/bitquan
- 📧 Email: testnet@bitquan.io

### Contribution
Want to help improve testnet?
1. Fork the repository
2. Create feature branch
3. Submit pull request
4. Join our Discord

---

## 🎯 Testnet Phases

### Phase 1: Core Testing (2 weeks)
- Node stability
- Basic transactions
- Wallet operations

### Phase 2: Mining Testing (2 weeks)
- Pool mining
- Solo mining
- Algorithm testing

### Phase 3: Stress Testing (2 weeks)
- High transaction volume
- Network partitions
- Edge cases

### Phase 4: Feature Testing (Ongoing)
- Multi-sig wallets
- Smart contracts (future)
- Cross-chain bridges (future)

---

**Ready to launch testnet? Let's go! 🚀**
