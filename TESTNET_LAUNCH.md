# 🚀 BitQuan Testnet Launch Guide

## 📋 Overview
BitQuan Testnet is now **production-ready** with complete PQC wallet system, transaction broadcasting, and network infrastructure!

## 🏗️ Infrastructure Components

### 1. Bootstrap Nodes 🌐
**Purpose**: Entry points for new nodes to join the network

**Configuration** (in `config/testnet.toml`):
```toml
[network]
p2p_port = 19444
rpc_port = 19443

bootstrap_nodes = [
    "bootstrap1.testnet.bitquan.org:19444",
    "bootstrap2.testnet.bitquan.org:19444", 
    "bootstrap3.testnet.bitquan.org:19444",
    "seed.testnet.bitquan.org:19444",
]
```

**Setup**:
```bash
# Deploy bootstrap node
sudo ./deploy/setup-bootstrap-node.sh

# Verify node is running
curl http://localhost:19443 -X POST \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'
```

### 2. Testnet Faucet 🚰
**Purpose**: Distribute testnet coins for development and testing

**Features**:
- 💰 1 BQ per request
- ⏰ 1 hour cooldown
- 📊 5 requests per IP per day
- 🛡️ Rate limiting and IP tracking
- 🌐 Web interface

**Setup**:
```bash
# Deploy faucet
sudo ./deploy/setup-faucet.sh

# Install dependencies
pip install -r tools/requirements-faucet.txt

# Run faucet manually (for testing)
cd tools && python testnet_faucet.py
```

**Faucet Address**: `bq1q9aplqkvkjghqdpxfkz8nx4hkv38q5kjqs92fztn9gd5vcuk0tt7wzz5aga`

## 🚀 Quick Start Guide

### For Users:

#### 1. Get Testnet Coins
```bash
# Visit faucet web interface
http://faucet.testnet.bitquan.org

# Or get your address first
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
```

#### 2. Create Wallet
```bash
./target/release/bitquan-node wallet-gen \
  --output my-wallet.keystore \
  --network testnet
```

#### 3. Send Transactions
```bash
./target/release/bitquan-node wallet-send \
  --keystore my-wallet.keystore \
  --to bq1q...recipient-address \
  --amount 100000000 \
  --fee-rate 1
```

#### 4. Run Node
```bash
./target/release/bitquan-node run --config config/testnet.toml
```

### For Developers:

#### 1. Connect to Testnet
```bash
# Clone and build
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
cargo build --release

# Run with testnet config
./target/release/bitquan-node run --config config/testnet.toml
```

#### 2. RPC Integration
```bash
# Check blockchain info
curl http://localhost:19443 -X POST \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}'

# Submit transaction
curl http://localhost:19443 -X POST \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"submittransaction","params":["tx_hex_here"],"id":1}'
```

## 🔧 Network Configuration

### Testnet Parameters:
- **Network ID**: `testnet`
- **P2P Port**: `19444`
- **RPC Port**: `19443`
- **Genesis Hash**: Auto-generated
- **Block Time**: 10 minutes (600 seconds)
- **Initial Difficulty**: Very easy for testing
- **Block Reward**: 50 BQ
- **Halving Interval**: 210,000 blocks

### Security Features:
- 🔐 **Post-Quantum Cryptography**: Dilithium3 signatures
- 🛡️ **NIST Standardized**: Quantum-resistant algorithms
- 🔑 **Encrypted Storage**: Argon2id key derivation
- 📡 **Secure RPC**: JSON-RPC 2.0 with optional auth
- 🌐 **P2P Network**: Peer discovery and message relay

## 📊 Monitoring & Tools

### 1. Block Explorer
- **URL**: `https://explorer.testnet.bitquan.org`
- **Features**: 
  - 📊 Real-time block tracking
  - 🔍 Transaction search
  - 📈 Network statistics
  - 🏠 Address details

### 2. Network Statistics
```bash
# Get network status
curl http://localhost:19443 -X POST \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getnetworkstatus","params":[],"id":1}'

# Get mining info
curl http://localhost:19443 -X POST \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getmininginfo","params":[],"id":1}'
```

### 3. Faucet Stats
```bash
# Check faucet statistics
curl http://faucet.testnet.bitquan.org/stats

# Health check
curl http://faucet.testnet.bitquan.org/health
```

## 🛠️ Development Tools

### 1. Wallet Operations
```bash
# Generate wallet
./target/release/bitquan-node wallet-gen --network testnet

# Get address
./target/release/bitquan-node wallet-address --keystore wallet.keystore

# Sign message
./target/release/bitquan-node wallet-sign --keystore wallet.keystore --message "Hello Testnet"

# Verify signature
./target/release/bitquan-node wallet-verify --pubkey <hex> --message "Hello Testnet" --signature <hex>
```

### 2. Transaction Building
```bash
# Build unsigned transaction
./target/release/bitquan-node build-tx \
  --prev-txid <txid> \
  --prev-vout 0 \
  --value 100000000 \
  --to-script <script_hex>
```

### 3. Multi-signature
```bash
# Generate multisig wallet
./target/release/bitquan-node wallet-gen-multisig --threshold 2 --participants 3

# Sign partial transaction
./target/release/bitquan-node tx-sign-partial \
  --tx <tx_json> \
  --multisig-config <config> \
  --keystore wallet.keystore

# Combine signatures
./target/release/bitquan-node tx-combine-signatures \
  --tx <tx_json> \
  --signatures <sig1>,<sig2>
```

## 🌍 Community Resources

### 1. Documentation
- **Main Docs**: `https://docs.bitquan.org`
- **API Reference**: `https://api.bitquan.org`
- **GitHub**: `https://github.com/AlphaB135/BitQuan`

### 2. Support
- **Discord**: `https://discord.gg/bitquan`
- **Telegram**: `https://t.me/bitquan`
- **Twitter**: `@BitQuanCrypto`

### 3. Contributing
- **Bug Reports**: GitHub Issues
- **Feature Requests**: GitHub Discussions
- **Development**: See `CONTRIBUTING.md`

## 🎯 Next Steps

### Phase 1: Testnet Launch ✅
- [x] Complete PQC wallet system
- [x] Transaction broadcasting
- [x] Bootstrap nodes
- [x] Testnet faucet
- [x] RPC interface

### Phase 2: Network Growth 🚀
- [ ] Deploy multiple bootstrap nodes
- [ ] Launch block explorer
- [ ] Community testing
- [ ] Performance optimization

### Phase 3: Mainnet Preparation 🏁
- [ ] Security audits
- [ ] Economic model finalization
- [ ] Exchange partnerships
- [ ] Mainnet launch

## 🔒 Security Considerations

### Testnet Security:
- ⚠️ **Testnet coins have no value**
- 🔐 **Never reuse testnet keys on mainnet**
- 🛡️ **Keep testnet and mainnet separate**
- 📡 **Use secure connections for RPC**

### Reporting Issues:
- 🐛 **Security bugs**: security@bitquan.org
- 📋 **General issues**: GitHub Issues
- 🚨 **Network problems**: Discord/Telegram

---

## 🎉 Ready to Launch!

BitQuan Testnet is **production-ready** with:
- ✅ **Post-Quantum Security** (Dilithium3)
- ✅ **Complete Wallet System** 
- ✅ **Transaction Broadcasting**
- ✅ **Network Infrastructure**
- ✅ **Developer Tools**

**🚀 Let's launch the first post-quantum testnet!**