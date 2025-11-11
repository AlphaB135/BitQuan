# BitQuan Mainnet Launch Announcement

## Release Information

**Version:** v1.0.0  
**Launch Date:** TBD (Post-Audit)  
**Network ID:** `mainnet`  
**Chain ID:** `bitquan-mainnet-v1`

---

## Genesis Parameters

**Genesis Hash:** `000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f`  
**Genesis Timestamp:** `1704067200` (January 1, 2024 00:00:00 UTC)  
**Genesis File:** [`genesis/mainnet.json`](../genesis/mainnet.json)

**Consensus:**
- **PoW Algorithm:** Hybrid mining
  - SHA-256d (ASIC-friendly) - Available from genesis
  - RandomX (CPU-friendly) - Available from block 10,000
  - Ethash (GPU-friendly) - Available from block 10,000
- **Target Block Time:** 600 seconds (10 minutes)
- **Difficulty Adjustment:** ASERT (aserti3-2d), 2016-block window
- **Initial Subsidy:** 50 BQ (5,000,000,000 satoshis)
- **Halving Interval:** 210,000 blocks (~4 years)
- **Total Supply:** 21,000,000 BQ (fixed cap)

**Security:**
- **Signatures:** CRYSTALS-Dilithium3 (NIST PQC standard)
- **Key Derivation:** BIP32-style with post-quantum extensions
- **Block Weight:** Accounts for large PQC signatures (2.5KB per input)

---

## Network Endpoints

### DNS Seeds

Bootstrap via DNS seeds (≥60% reachability required):

```
seed1.bitquan.network:8333
seed2.bitquan.network:8333
seed3.bitquan.network:8333
seed4.bitquan.network:8333
seed5.bitquan.network:8333
```

**Validation:** `bq-preflight dns-check --network mainnet --dns-seed-threshold 60`

### RPC Endpoint

**Default:** `http://localhost:8332`  
**Authentication:** JWT tokens (required for production)

Example RPC call:
```bash
curl -X POST http://localhost:8332 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getblockcount","params":[]}'
```

### Stratum Mining Pool

**Default:** `stratum+tcp://localhost:3333`  
**Supported Algorithms:** SHA-256d (block 0+), RandomX & Ethash (block 10,000+)

**Miner Connection:**
```bash
# cgminer example
cgminer --url stratum+tcp://pool.bitquan.org:3333 \
        --user bitquan_address \
        --pass x
```

---

## Release Artifacts

All release binaries are available on [GitHub Releases](https://github.com/AlphaB135/BitQuan/releases/tag/v1.0.0).

### Binaries

- **Linux x86_64:** `bitquan-node-linux-x86_64`
- **Linux ARM64:** `bitquan-node-linux-aarch64`

### Checksums

**File:** `SHA256SUMS`

Verify integrity:
```bash
sha256sum -c SHA256SUMS
```

Expected checksums (to be filled by release workflow):
```
<SHA256>  bitquan-node-linux-x86_64
<SHA256>  bitquan-node-linux-aarch64
```

### Attestation

**File:** `attestation.sig` (optional, if using cosign)

Verify with cosign (keyless OIDC):
```bash
cosign verify-blob \
  --certificate bitquan-node-linux-x86_64.cert \
  --signature bitquan-node-linux-x86_64.sig \
  bitquan-node-linux-x86_64
```

---

## Quick Start

### 1. Download & Verify

```bash
# Download binary
wget https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-node-linux-x86_64
wget https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/SHA256SUMS

# Verify checksum
sha256sum -c SHA256SUMS --ignore-missing

# Make executable
chmod +x bitquan-node-linux-x86_64
```

### 2. Initialize Configuration

```bash
# Generate default config
./bitquan-node-linux-x86_64 init --network mainnet

# Config written to: ~/.bitquan/mainnet.toml
```

### 3. Start Node

```bash
# Run node (sync from genesis)
./bitquan-node-linux-x86_64 run --config ~/.bitquan/mainnet.toml

# Or as systemd service (recommended)
sudo systemctl enable bitquan-node
sudo systemctl start bitquan-node
```

### 4. Check Status

```bash
# Query chain height
curl -X POST http://localhost:8332 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getblockcount","params":[]}'

# Expected: {"jsonrpc":"2.0","id":1,"result":0}  # Genesis
```

---

## Mining

### Solo Mining (Advanced)

Solo mining requires:
- SHA-256d ASIC miners (Bitcoin-compatible)
- High hash rate (>1 TH/s recommended for profitability)

**Note:** Mainnet difficulty adjusts dynamically. Solo mining may not be profitable initially.

### Pool Mining (Recommended)

Join a mining pool for consistent payouts:

**Official Pool:** `pool.bitquan.org:3333` (TBD)

Configure your miner:
```
URL: stratum+tcp://pool.bitquan.org:3333
Worker: <your_bitquan_address>
Password: x
```

---

## Explorer

**Mainnet Explorer:** `https://explorer.bitquan.org` (TBD)

View:
- Real-time chain height and latest blocks
- Transaction history
- Address balances
- Network hashrate

---

## Testnet & Faucet

For testing before mainnet:

**Testnet Faucet:** `https://faucet.bitquan.org`  
**Testnet Explorer:** `https://testnet-explorer.bitquan.org`

Testnet allows:
- RandomX mining (CPU-friendly)
- Free testnet coins from faucet
- Risk-free transaction testing

---

## Upgrade & Safety Notes

### Breaking Changes from Testnet

1. **PoW Algorithm:** Mainnet starts with SHA-256d only, enables hybrid mining (RandomX + Ethash) at block 10,000.
2. **Network ID:** Separate chain (mainnet txs incompatible with testnet).
3. **Genesis Block:** Different genesis hash and initial state.

### Security Best Practices

1. **Run your own node:** Don't trust third-party RPC providers for sensitive operations.
2. **Verify binaries:** Always check SHA256 checksums before running.
3. **Backup keys:** Store wallet seeds in secure, offline locations.
4. **Use JWT auth:** Enable JWT authentication for RPC endpoints.
5. **Monitor logs:** Watch for unusual activity (see [OBSERVABILITY.md](../ops/OBSERVABILITY.md)).

### Upgrade Path

For future releases:
1. Download new binary and verify checksums
2. Stop node gracefully: `sudo systemctl stop bitquan-node`
3. Replace binary: `sudo cp bitquan-node /usr/local/bin/`
4. Start node: `sudo systemctl start bitquan-node`
5. Verify sync: Check logs and RPC `getblockcount`

**No data migration needed:** Chain data persists across upgrades (backward-compatible).

---

## PGP Key Fingerprints

For verifying signed communications and releases:

**Release Signing Key:**  
`AB12 34CD 56EF 7890 1234  5678 90AB CDEF 0123 4567` (example)

Import key:
```bash
gpg --keyserver keys.openpgp.org --recv-keys 0x0123456789ABCDEF
```

Verify release signature:
```bash
gpg --verify bitquan-node-linux-x86_64.asc bitquan-node-linux-x86_64
```

---

## Support & Community

- **Discord:** https://discord.gg/bitquan
- **Telegram:** https://t.me/bitquan
- **Forum:** https://forum.bitquan.org
- **GitHub Issues:** https://github.com/AlphaB135/BitQuan/issues

**Security Issues:** See [SECURITY.md](../SECURITY.md) for responsible disclosure.

---

## License

Apache 2.0 - See [LICENSE](../LICENSE)

---

**Prepared by:** BitQuan Core Team  
**Last Updated:** 2025-11-06

**Ready for Launch:** ✅ (Pending final audit)
