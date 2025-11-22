# 🧪 BitQuan Testnet - Tester Quick Start Guide

Welcome to BitQuan testnet testing! This guide will help you get started in 5 minutes.

## 🎯 What is BitQuan?

BitQuan is a **post-quantum secure blockchain** with:
- ✅ Quantum-resistant signatures (Dilithium3)
- ✅ Bitcoin-like economics (halving, UTXO model)
- ✅ Mining pool with Stratum support
- ✅ Fast ASERT difficulty adjustment
- ✅ Secure wallet with HD derivation

**Testnet Purpose**: Test all features before mainnet launch. Testnet coins have NO VALUE.

---

## 🚀 Quick Start (5 Minutes)

### Option 1: Using Pre-built Binary (Recommended)

#### 1. Download Client
```bash
# Linux
wget https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-linux-x86_64
chmod +x bitquan-linux-x86_64
mv bitquan-linux-x86_64 bitquan

# macOS
wget https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-macos-x86_64
chmod +x bitquan-macos-x86_64
mv bitquan-macos-x86_64 bitquan

# Windows
# Download from: https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-windows-x86_64.exe
```

#### 2. Create Wallet
```bash
./bitquan wallet create --network testnet
```

This creates `wallet.keystore` and shows your address (starts with `tBQ1`)

#### 3. Get Testnet Coins
Visit faucet and enter your address:
```
🚰 Faucet: http://faucet.bitquan.io
```

You'll receive **100 testnet BQ** (worth $0, for testing only)

#### 4. Check Balance
```bash
./bitquan balance --address YOUR_ADDRESS
```

#### 5. Send Transaction
```bash
./bitquan send \
  --from wallet.keystore \
  --to RECIPIENT_ADDRESS \
  --amount 10.5
```

---

### Option 2: Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone repo
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
git checkout v1.0.0

# Build
cargo build --release

# Run
./target/release/bitquan-node --help
```

---

## ⛏️ Mining (Optional)

### Solo Mining
```bash
./bitquan mine \
  --address YOUR_ADDRESS \
  --threads 4 \
  --network testnet
```

### Pool Mining
```bash
./bitquan mine \
  --pool stratum+tcp://pool.bitquan.io:3333 \
  --address YOUR_ADDRESS \
  --threads 4 \
  --algo sha256d
```

**Pool Dashboard**: http://pool.bitquan.io:8080

---

## 🧪 What to Test

### ✅ **Basic Testing** (Everyone should do)

1. **Wallet Operations**
   - [ ] Create new wallet
   - [ ] Restore from mnemonic
   - [ ] Generate multiple addresses
   - [ ] Export private key

2. **Transactions**
   - [ ] Send coins
   - [ ] Receive coins
   - [ ] Check transaction confirmation
   - [ ] Verify in explorer

3. **Balance Checking**
   - [ ] Check balance via CLI
   - [ ] Check balance via RPC
   - [ ] Check unconfirmed balance

### 🔧 **Advanced Testing** (For technical users)

4. **Mining**
   - [ ] Solo mine a block
   - [ ] Join mining pool
   - [ ] Test different algorithms (SHA256d, RandomX)
   - [ ] Monitor hashrate

5. **Multi-signature Wallets**
   ```bash
   ./bitquan multisig create \
     --required 2 \
     --total 3 \
     --pubkey1 PUBKEY1 \
     --pubkey2 PUBKEY2 \
     --pubkey3 PUBKEY3
   ```

6. **Time-locked Transactions**
   ```bash
   ./bitquan send \
     --from wallet.keystore \
     --to ADDRESS \
     --amount 5.0 \
     --locktime 1000  # Block height
   ```

7. **Post-Quantum Signatures**
   - [ ] Sign message with Dilithium3
   - [ ] Verify PQ signature
   - [ ] Check signature size

8. **RPC Testing**
   ```bash
   # Get blockchain info
   curl -X POST http://testnet.bitquan.io:8334/rpc \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}'

   # Get block by height
   curl -X POST http://testnet.bitquan.io:8334/rpc \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"getblock","params":[100],"id":1}'
   ```

9. **Network Testing**
   - [ ] Connect to multiple peers
   - [ ] Test peer discovery
   - [ ] Monitor block propagation
   - [ ] Test during network partition

10. **Stress Testing**
    ```bash
    # Send 1000 transactions
    for i in {1..1000}; do
      ./bitquan send --from wallet.keystore --to ADDRESS --amount 0.001
      sleep 1
    done
    ```

---

## 🐛 Found a Bug?

### How to Report

1. **GitHub Issues**: https://github.com/AlphaB135/BitQuan/issues
2. **Include**:
   - What you were doing
   - What happened (error message, screenshot)
   - What you expected
   - System info (OS, version)
   - Logs (if possible)

### Example Bug Report
```
Title: Transaction fails when sending max amount

Steps to reproduce:
1. Create wallet
2. Receive 100 BQ from faucet
3. Try to send 100 BQ to another address
4. Get error: "insufficient funds"

Expected: Should send 99.999 BQ (minus fee)
Actual: Error message

Environment:
- OS: Ubuntu 22.04
- Version: v1.0.0
- Network: Testnet

Logs:
[paste relevant logs]
```

---

## 📊 Testnet Resources

### Public Infrastructure
- **RPC Node**: http://testnet.bitquan.io:8334
- **Mining Pool**: stratum+tcp://pool.bitquan.io:3333
- **Block Explorer**: http://explorer.bitquan.io
- **Faucet**: http://faucet.bitquan.io
- **Pool Dashboard**: http://pool.bitquan.io:8080

### Documentation
- **Full Docs**: https://docs.bitquan.io
- **API Reference**: https://docs.bitquan.io/api
- **Mining Guide**: https://docs.bitquan.io/mining
- **Wallet Guide**: https://docs.bitquan.io/wallet

### Community
- **Discord**: https://discord.gg/bitquan
- **Telegram**: https://t.me/bitquan_testnet
- **Twitter**: https://twitter.com/bitquan
- **Forum**: https://forum.bitquan.io

---

## ❓ FAQ

### Q: Are testnet coins worth anything?
**A**: No. Testnet coins have ZERO value. They are only for testing.

### Q: Can I mine testnet coins?
**A**: Yes! Solo mining or join our pool: stratum+tcp://pool.bitquan.io:3333

### Q: How long until mainnet?
**A**: After thorough testnet testing (estimated 2-3 months).

### Q: Can I keep my testnet wallet for mainnet?
**A**: NO! Use different wallets. Testnet may be reset anytime.

### Q: What if I lose testnet coins?
**A**: No problem! Get more from the faucet (limit: 100 BQ per 24h).

### Q: Why is my transaction pending?
**A**: Wait for block confirmation (avg 10 minutes). Check explorer.

### Q: Can I run my own node?
**A**: Yes! See: docs/TESTNET_SETUP.md

### Q: What are the system requirements?
**A**:
- RAM: 4GB minimum
- Storage: 50GB
- Network: Decent internet connection
- OS: Linux/macOS/Windows

### Q: Is it safe to test?
**A**: Yes, but:
- ⚠️ Don't use real passwords
- ⚠️ Don't use mainnet keys
- ⚠️ Test in isolated environment
- ⚠️ No financial value at stake

---

## 🎁 Rewards

### Bug Bounty
Find critical bugs? Get rewarded on mainnet launch!

**Severity Levels**:
- 🔴 **Critical**: Network halt, fund loss → 1000-5000 BQ
- 🟠 **High**: Security vulnerability → 500-1000 BQ
- 🟡 **Medium**: Functionality bug → 100-500 BQ
- 🟢 **Low**: UI/UX issue → 50-100 BQ

**Note**: Rewards paid after mainnet launch in mainnet BQ.

### Top Contributors
Most active testers get:
- Early access to mainnet
- Verified tester badge
- Mainnet airdrop
- Community recognition

---

## 📝 Testing Checklist

Print and check off as you test:

**Basic Operations**
- [ ] Create wallet
- [ ] Get testnet coins from faucet
- [ ] Check balance
- [ ] Send transaction
- [ ] Receive transaction
- [ ] Verify transaction in explorer

**Advanced Features**
- [ ] Solo mine a block
- [ ] Join mining pool
- [ ] Create multi-sig wallet
- [ ] Sign/verify message
- [ ] Time-locked transaction
- [ ] HD wallet derivation

**Edge Cases**
- [ ] Send max amount
- [ ] Send to invalid address
- [ ] Double spend attempt
- [ ] Zero-value transaction
- [ ] Very large transaction

**Performance**
- [ ] Send 100 transactions
- [ ] Mine for 1 hour
- [ ] Monitor resource usage
- [ ] Test under load

**Bugs Found**: _____________

**Overall Rating**: ⭐⭐⭐⭐⭐

**Comments**:
_________________________________
_________________________________
_________________________________

---

## 🎯 Ready to Test?

1. ✅ Download client
2. ✅ Create wallet
3. ✅ Get testnet coins
4. ✅ Start testing!

**Questions?** Join our Discord: https://discord.gg/bitquan

**Happy Testing! 🚀**

---

*Last Updated: 2025-11-11*
*Version: 1.0.0*
*Network: Testnet*
