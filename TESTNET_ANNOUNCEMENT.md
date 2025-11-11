# 🧪 BitQuan Testnet - Open for Public Testing!

[![CI Status](https://github.com/AlphaB135/BitQuan/workflows/CI/badge.svg)](https://github.com/AlphaB135/BitQuan/actions)
[![Version](https://img.shields.io/badge/version-1.0.0-blue)](https://github.com/AlphaB135/BitQuan/releases)
[![Network](https://img.shields.io/badge/network-testnet-orange)](http://testnet.bitquan.io)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](LICENSE)

**BitQuan testnet is now OPEN for public testing! Help us test the first post-quantum secure blockchain.**

---

## 🎯 Quick Links

- 🚰 **Faucet**: [Get free testnet coins](http://faucet.bitquan.io)
- 🔍 **Explorer**: [View blockchain](http://explorer.bitquan.io)
- ⛏️ **Mining Pool**: stratum+tcp://pool.bitquan.io:3333
- 📊 **Pool Dashboard**: [http://pool.bitquan.io:8080](http://pool.bitquan.io:8080)
- 📖 **Docs**: [docs.bitquan.io](https://docs.bitquan.io)
- 💬 **Discord**: [Join Community](https://discord.gg/bitquan)

---

## 🚀 Get Started in 3 Steps

### 1️⃣ Download Client
```bash
wget https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-linux-x86_64
chmod +x bitquan-linux-x86_64
./bitquan-linux-x86_64 --version
```

### 2️⃣ Create Wallet
```bash
./bitquan-linux-x86_64 wallet create --network testnet
```
Save your mnemonic phrase! Your address will start with `tBQ1`

### 3️⃣ Get Testnet Coins
Visit: **http://faucet.bitquan.io** and enter your address

You'll receive **100 testnet BQ** for testing!

---

## 🧪 What We Need You to Test

### 🔰 Beginner Level
- ✅ Create and restore wallets
- ✅ Send and receive transactions  
- ✅ Check balances
- ✅ Use the faucet

### 🔧 Intermediate Level
- ⛏️ Solo mining
- ⛏️ Pool mining
- 🔐 Multi-signature wallets
- ⏰ Time-locked transactions

### 🚀 Advanced Level
- 📡 Run your own node
- 🌐 P2P networking
- 🔬 Stress testing
- 🐛 Edge case hunting

**📋 Full Testing Guide**: [docs/TESTER_GUIDE.md](docs/TESTER_GUIDE.md)

---

## 🎁 Bug Bounty Program

Find bugs and earn rewards on mainnet launch!

| Severity | Reward | Examples |
|----------|---------|----------|
| 🔴 Critical | 1000-5000 BQ | Network halt, fund loss |
| 🟠 High | 500-1000 BQ | Security vulnerabilities |
| 🟡 Medium | 100-500 BQ | Functionality bugs |
| 🟢 Low | 50-100 BQ | UI/UX issues |

**Report bugs**: [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)

---

## 📊 Testnet Infrastructure

### Public Nodes
```
RPC:     http://testnet.bitquan.io:8334
P2P:     testnet.bitquan.io:8333
Pool:    stratum+tcp://pool.bitquan.io:3333
Faucet:  http://faucet.bitquan.io:5000
```

### Mining Pool Stats
- **Algorithm**: SHA256d / RandomX
- **Starting Difficulty**: 1000
- **VarDiff**: Enabled (100-10000)
- **Pool Fee**: 0% (testnet)
- **Payout**: Every 1 hour
- **Min Payout**: 0.01 BQ

### Network Stats
- **Block Time**: ~10 minutes (target)
- **Block Reward**: 50 BQ (halving every 210,000 blocks)
- **Difficulty Adjustment**: ASERT (real-time)
- **Consensus**: Proof of Work
- **Signatures**: Post-Quantum (Dilithium3)

---

## 🔧 Run Your Own Node

### Option 1: Quick Setup Script
```bash
curl -fsSL https://raw.githubusercontent.com/AlphaB135/BitQuan/main/scripts/setup-testnet.sh | sudo bash
```

### Option 2: Docker Compose
```bash
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
docker-compose -f docker-compose.testnet.yml up -d
```

### Option 3: Manual Setup
See detailed guide: [docs/TESTNET_SETUP.md](docs/TESTNET_SETUP.md)

---

## 🗺️ Testnet Roadmap

### Phase 1: Core Testing ✅ (Current)
**Duration**: 2 weeks  
**Focus**: Basic functionality
- [x] Node stability
- [x] Wallet operations
- [x] Transaction sending/receiving
- [ ] 100+ active testers
- [ ] 1000+ transactions

### Phase 2: Mining Testing
**Duration**: 2 weeks  
**Focus**: Mining functionality
- [ ] Solo mining
- [ ] Pool mining  
- [ ] Algorithm testing
- [ ] 50+ miners
- [ ] 500+ blocks mined

### Phase 3: Stress Testing
**Duration**: 2 weeks
**Focus**: Performance & edge cases
- [ ] High transaction volume
- [ ] Network partitions
- [ ] Large reorgs
- [ ] 5000+ TPS test

### Phase 4: Security Audit
**Duration**: 4 weeks
**Focus**: Security review
- [ ] Professional audit
- [ ] Penetration testing
- [ ] Code review
- [ ] Fix all critical issues

### Mainnet Launch 🚀
**Target**: Q1 2026 (after successful testing)

---

## 📚 Documentation

- **[Tester Guide](docs/TESTER_GUIDE.md)** - Start here!
- **[Testnet Setup](docs/TESTNET_SETUP.md)** - Run your own node
- **[API Reference](docs/API.md)** - RPC API documentation
- **[Mining Guide](docs/MINING.md)** - How to mine
- **[Wallet Guide](docs/WALLET.md)** - Wallet operations

---

## 💬 Community & Support

### Get Help
- 💬 **Discord**: https://discord.gg/bitquan
- 📱 **Telegram**: https://t.me/bitquan_testnet  
- 🐦 **Twitter**: https://twitter.com/bitquan
- 📧 **Email**: testnet@bitquan.io

### Stay Updated
- 📰 **Blog**: https://blog.bitquan.io
- 📺 **YouTube**: https://youtube.com/@bitquan
- 📝 **Forum**: https://forum.bitquan.io

---

## ⚠️ Important Notes

### Testnet Warnings
- ⚠️ **Testnet coins have NO VALUE**
- ⚠️ **Network may be reset anytime**
- ⚠️ **Do NOT use mainnet keys**
- ⚠️ **Test in isolated environment**

### What Testnet is NOT
- ❌ Not for storing real value
- ❌ Not a preview of mainnet economics
- ❌ Not guaranteed to be stable
- ❌ Not for production use

### What Testnet IS
- ✅ For testing functionality
- ✅ For finding bugs
- ✅ For learning the system
- ✅ For having fun!

---

## 🏆 Hall of Fame

### Top Testers
*Coming soon - be the first!*

### Top Bug Hunters
*Coming soon - find bugs, get recognized!*

### Top Miners
*Coming soon - mine blocks, win prizes!*

---

## 📈 Current Stats

```
Network:        Testnet
Version:        v1.0.0
Block Height:   Pending (launching soon)
Total Tx:       0
Active Nodes:   Launching...
Hashrate:       Launching...
Difficulty:     1000 (starting)
```

**Live Stats**: http://explorer.bitquan.io/stats

---

## 🤝 Contributing

Want to contribute code?

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing`)
5. Open Pull Request

**See**: [CONTRIBUTING.md](CONTRIBUTING.md)

---

## 📜 License

Apache 2.0 - See [LICENSE](LICENSE) for details

---

## 🎉 Let's Test Together!

**Ready to start?**

1. ✅ [Download Client](https://github.com/AlphaB135/BitQuan/releases)
2. ✅ [Create Wallet](#2️⃣-create-wallet)
3. ✅ [Get Coins](http://faucet.bitquan.io)
4. ✅ [Start Testing!](docs/TESTER_GUIDE.md)

**Questions?** Join our [Discord](https://discord.gg/bitquan)

---

<p align="center">
  <strong>🚀 Help us build the future of post-quantum blockchain! 🚀</strong>
</p>

<p align="center">
  <sub>Built with ❤️ by the BitQuan team</sub>
</p>
