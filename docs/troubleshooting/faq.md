# Frequently Asked Questions (FAQ)

Quick answers to common questions about BitQuan.

## General Questions

### Is BitQuan mainnet live?

**No.** BitQuan is currently in **pre-mainnet** development.

**Current Status:**
- Code: 100% complete
- Tests: All passing (185+ tests)
- Mainnet: NOT launched (pending security audit)
- Testnet: In development

**Do NOT send real funds.** Only use testnet/devnet for testing.

### What makes BitQuan different from Bitcoin?

| Feature | Bitcoin | BitQuan |
|---------|---------|---------|
| **Signatures** | ECDSA (vulnerable to quantum) | Dilithium5 (post-quantum secure) |
| **Precision** | 8 decimals (satoshi) | 18 decimals (qbits) |
| **Supply** | 21 million BTC | 21 million BQ |
| **Smart Contracts** | Yes (via Script) | No (simple value transfer only) |
| **Consensus** | Longest chain rule | Longest VALID chain rule |
| **Mining** | SHA-256d only | SHA-256d + RandomX (experimental) |

### What is "post-quantum secure"?

BitQuan uses **CRYSTALS-Dilithium5**, a cryptographic signature algorithm selected by NIST in 2022 as the standard for post-quantum digital signatures.

**Why it matters:**
- ECDSA (used by Bitcoin) is vulnerable to quantum computers
- Dilithium5 is secure against both classical and quantum attacks
- Designed for 50+ years of security

### How can I get BitQuan coins?

**Currently:** You can only mine coins on testnet/devnet (which have NO value).

**After mainnet launch:**
- Mine coins with CPU/GPU/ASIC
- Receive coins from others
- Buy on exchanges (if listed)

**DO NOT:** Buy "BitQuan" from anyone claiming to sell it before mainnet launch - these are scams.

## Getting Started

### How do I install BitQuan?

```bash
# Clone repository
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan

# Build from source
cargo build --release

# Verify installation
./target/release/bitquan-node --version
```

See [Getting Started](../getting-started/quick-start.md) for detailed guide.

### How do I create a wallet?

```bash
# Generate new wallet
./target/release/bitquan-node wallet-gen --output my-wallet.keystore

# Set password (8+ characters)
# SAVE YOUR MNEMONIC IF SHOWN!

# Get your address
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
```

See [Wallet Generation](../wallet/generation.md) for details.

### How do I connect to testnet?

```bash
# Start testnet node
./target/release/bitquan-node --network testnet --datadir ./data/testnet

# Or with config file
./target/release/bitquan-node --config config/testnet.toml
```

See [Testnet Guide](../TESTNET_README.md) for testnet-specific information.

## Mining

### Why is my balance 0 after mining?

**Coinbase maturity:** Mined coins require **100 blocks** to mature before they can be spent.

**Example:**
- You mine a block at height #50
- Coins unlock at height #150
- Before #150, balance shows those coins but they're unspendable

**Check maturity:**
```bash
# Check if coins are mature
./target/release/bitquan-node balance \
  --address <your-address> \
  --datadir ./data/chainstate
```

### Which mining algorithm should I use?

**For testing:** Use `--pow mock` for instant blocks.

**For production:**
- **SHA-256d (hashcash):** Best performance, compatible with ASICs
- **RandomX:** CPU-friendly, experimental, lower hash rate

**Recommendation:** Use SHA-256d (hashcash) for best results.

```bash
# SHA-256d (recommended)
./target/release/bitquan-node mine --pow hashcash

# RandomX (experimental)
./target/release/bitquan-node mine --pow randomx

# Mock (testing only)
./target/release/bitquan-node mine --pow mock
```

### Can I mine with my CPU?

**Yes, BUT:**

- **Solo mining:** Extremely unlikely to find block on mainnet
- **Testnet/devnet:** Perfectly fine for learning
- **Mainnet:** Join mining pool or expect zero income

**For profit:** Use ASICs or GPUs, not CPU.

## Wallet

### How do I backup my wallet?

**1. Backup Keystore File:**
```bash
cp my-wallet.keystore backup/wallet.keystore.backup
```

**2. Backup Mnemonic:**
- Write on paper (NEVER digital)
- Store in secure location (safe, fireproof box)
- Consider steel backup (fire/water resistant)

**3. Test Backup:**
- Restore from backup to verify it works
- Do this BEFORE relying on it

See [Wallet Backup](../wallet/backup.md) for complete guide.

### I forgot my wallet password, what do I do?

**Unfortunately, there is NO password recovery.**

BitQuan uses Argon2id encryption specifically designed to prevent brute-force attacks.

**Options:**
1. Try all password variations carefully
2. Check if you wrote it down somewhere
3. If you have mnemonic, restore wallet from mnemonic

**If all else fails:** Funds are permanently lost. This is the harsh reality of self-custody cryptocurrency.

### How do I restore from mnemonic?

```bash
# Restore wallet from BIP39 mnemonic
./target/release/bitquan-node wallet-from-mnemonic \
  --phrase "word1 word2 word3 ... word12" \
  --output restored-wallet.keystore

# Set new password
# Verify address matches original
./target/release/bitquan-node wallet-address \
  --keystore restored-wallet.keystore
```

See [Mnemonic Guide](../wallet/mnemonic.md) for details.

## Network

### How many peers do I need?

**Minimum:** 1 peer (for basic operation)

**Recommended:** 8-16 peers (for robust operation)

**Maximum:** No hard limit, but resource constraints apply

**Check peer count:**
```bash
grep "peer" bitquan.log | tail -20
```

### Why can't I connect to peers?

**Common causes:**
1. **Firewall blocking** - Open port 18444
2. **Wrong network** - Verify `--network` flag
3. **Peer offline** - Try different peers
4. **NAT issues** - Configure port forwarding

See [Network Issues](network-issues.md) for detailed troubleshooting.

### What is the P2P port?

**Default P2P port:** 18444

**Network-specific ports:**
- Mainnet: 18444
- Testnet: 19444
- Devnet: 18444 (can vary)

**RPC port:** 18443 (mainnet), 19443 (testnet)

## Transactions

### Why did my transaction fail?

**Common causes:**

1. **Insufficient funds**
   - Check account balance includes fees
   - Wait for coinbase maturity (100 blocks)

2. **Invalid signature**
   - Verify keystore password is correct
   - Check transaction was signed properly

3. **Double spend**
   - Can't spend same UTXO twice
   - Wait for previous transaction to confirm

4. **Network fee too low**
   - Increase fee rate
   - Wait for lower-fee period

### How long do transactions take to confirm?

**Testnet/devnet:** Seconds to minutes (depending on mining)

**Mainnet (when launched):**
- Average block time: 10 minutes (target)
- With 1+ confirmations: ~10 minutes
- With 6 confirmations: ~60 minutes (considered secure)

### What is the transaction fee?

**Fees depend on:**
- Transaction size (bytes)
- Network congestion
- Fee rate you set

**Typical fee:**
- Testnet/devnet: ~0.0001 BQ (minimal)
- Mainnet: Market rate (varies)

**Calculate fee:**
```
Fee = Transaction Size × Fee Rate
```

## Technical

### What programming language is BitQuan written in?

**Rust (Edition 2021)**

**Why Rust?**
- Memory safety (prevents entire classes of bugs)
- Performance (comparable to C/C++)
- Concurrency (safe multi-threading)
- Modern tooling (Cargo, rustfmt, clippy)

### What database does BitQuan use?

**RocksDB** (embedded key-value store)

**Why RocksDB?**
- High performance (optimized for flash storage)
- Reliable (battle-tested at scale)
- Embedded (no separate database server)
- Efficient compression

### How big is the blockchain?

**Current (devnet/testnet):**
- Depends on blocks mined
- Typically < 1GB for testing

**Mainnet estimate (after launch):**
- Similar to Bitcoin: ~500GB+ per year
- Depends on transaction volume

**Storage requirements:**
- Minimum: 20GB free space
- Recommended: 100GB+ for full node

### Can I run a full node?

**System requirements:**

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 4 cores | 8+ cores |
| **RAM** | 4GB | 8GB+ |
| **Storage** | 20GB SSD | 100GB+ SSD |
| **Network** | 10 Mbps | 100 Mbps |
| **OS** | Linux/macOS/Windows | Linux (Ubuntu 22.04+) |

**Yes!** BitQuan is designed to be runnable on consumer hardware.

## Security

### Is BitQuan secure?

**Current status:**
- Internal security audit: Complete
- External security audit: Pending
- E2E testing: Complete (all flows validated)
- Post-quantum crypto: Yes (Dilithium5)

**After external audit and mainnet launch:** Full production security

**DO NOT:** Use mainnet for significant funds until external audit complete.

### Has BitQuan been audited?

**Internal audits:** Complete
- Code review (PR #80)
- Security hardening
- Fuzzing strategy
- Dependency audit

**External audit:** Pending (scheduled before mainnet)

See [Security Audit Reports](../security/audits/) for details.

### What happens if quantum computers break Bitcoin?

BitQuan is **already quantum-resistant** due to Dilithium5 signatures.

**Timeline:**
- Now: BitQuan uses post-quantum crypto
- Bitcoin: Vulnerable, but can hard fork to quantum-safe algo
- BitQuan advantage: Already secure, no hard fork needed

## Development

### How can I contribute?

See [Contributing Guide](../guides/CONTRIBUTING.md).

**Quick start:**
1. Fork repository
2. Create feature branch
3. Make changes
4. Run tests (`cargo test`)
5. Run linting (`cargo clippy`)
6. Submit pull request

### Where can I get help?

- **Documentation:** [docs/](../README.md)
- **GitHub Issues:** [Report bugs](https://github.com/AlphaB135/BitQuan/issues)
- **GitHub Discussions:** [Ask questions](https://github.com/AlphaB135/BitQuan/discussions)
- **Troubleshooting:** [Troubleshooting Guide](README.md)

### Is BitQuan a scam?

**No.** BitQuan is:

- Open source (Apache 2.0, fully auditable)
- Spare-time solo project
- No pre-mine, no ICO, no token sale
- No promises of "get rich quick"

**What it is NOT:**
- Not a get-rich-quick scheme
- Not an investment opportunity
- Not asking for money
- Not making claims about price/moon

**Transparency:**
- All code public on GitHub
- Development reports published
- Security audits available
- Clear about pre-mainnet status

## Still Have Questions?

1. **Check Documentation:**
   - [Troubleshooting](README.md)
   - [Guides](../guides/)
   - [API Reference](../api/rpc/API_REFERENCE.md)

2. **Search Existing Issues:**
   - [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)

3. **Ask Community:**
   - [GitHub Discussions](https://github.com/AlphaB135/BitQuan/discussions)

4. **Read Source Code:**
   - [Repository](https://github.com/AlphaB135/BitQuan)
