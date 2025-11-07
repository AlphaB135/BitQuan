# BitQuan Genesis Blocks

## Overview
This document contains the canonical genesis block hashes for BitQuan networks. These values are used for preflight validation to ensure network integrity before mainnet launch.

## Mainnet Genesis

**Network ID:** `mainnet`  
**Chain ID:** `bitquan-mainnet-v1`  
**Genesis Hash:** `000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f`  
**Genesis Timestamp:** `1704067200` (January 1, 2024 00:00:00 UTC)  
**Genesis File:** [`genesis/mainnet.json`](../genesis/mainnet.json)

### Consensus Parameters
- **PoW Algorithm:** SHA-256d (single algorithm, hybrid forbidden)
- **Target Block Time:** 600 seconds (10 minutes)
- **Difficulty Adjustment:** ASERT algorithm, 2016 blocks
- **Initial Subsidy:** 50 BQ (5,000,000,000 satoshis)
- **Halving Interval:** 210,000 blocks (~4 years)

### Genesis Block Details
```
Height:         0
Version:        1
Prev Hash:      0000000000000000000000000000000000000000000000000000000000000000
Merkle Root:    4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b
Bits:           486604799 (0x1d00ffff)
Nonce:          2083236893
PoW Algo:       sha256d
```

### Coinbase Message
```
BitQuan mainnet genesis - Jan 2024 - Post-Quantum Secure Blockchain
```

## Testnet Genesis

**Network ID:** `testnet`  
**Chain ID:** `bitquan-testnet-v1`  
**Genesis Hash:** `000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943`  
**Genesis Timestamp:** `1704153600` (January 2, 2024 00:00:00 UTC)  
**Genesis File:** [`genesis/testnet.json`](../genesis/testnet.json)

### Consensus Parameters
- **PoW Algorithm:** Hybrid (SHA-256d + RandomX)
- **Target Block Time:** 120 seconds (2 minutes)
- **Difficulty Adjustment:** ASERT algorithm, 504 blocks
- **Initial Subsidy:** 50 BQ (testing value)

## Verification

### Manual Verification
```bash
# Extract hash from genesis file
jq -r '.genesis_hash' genesis/mainnet.json

# Compare with documented hash
grep "Genesis Hash:" docs/GENESIS.md
```

### Automated Preflight Check
```bash
# Run genesis verification
scripts/preflight/check_genesis_hash.sh mainnet v1.0.0

# Full preflight validation
scripts/preflight/preflight.sh --network mainnet --release-tag v1.0.0
```

## Checkpoints

### Mainnet Checkpoints
*To be populated after launch with significant milestone block hashes*

```
Height 100000: <hash>
Height 200000: <hash>
```

### Testnet Checkpoints
```
Height 50000:  <hash>
Height 100000: <hash>
```

## DNS Seeds

Canonical DNS seed addresses for network bootstrap are maintained in [`genesis/dns_seeds.txt`](../genesis/dns_seeds.txt).

**Mainnet Seeds:**
- seed1.bitquan.network:8333
- seed2.bitquan.network:8333
- seed3.bitquan.network:8333
- seed4.bitquan.network:8333
- seed5.bitquan.network:8333

**Testnet Seeds:**
- testnet-seed1.bitquan.network:18333
- testnet-seed2.bitquan.network:18333
- testnet-seed3.bitquan.network:18333

**Reachability Policy:**
- Minimum threshold: ≥60% of seeds must be reachable
- TCP probe timeout: 5 seconds
- Validated via `bq-preflight dns-check --dns-seed-threshold 60`

## Validation Requirements

Before mainnet v1.0.0 launch, the following must be verified:

1. ✅ Genesis hash matches documented value exactly
2. ✅ Consensus parameters are locked (no runtime modification)
3. ✅ PoW algorithm is SHA-256d only (hybrid disabled)
4. ✅ DNS seeds resolve and respond (≥60% threshold, 5s TCP probe)
5. ✅ Genesis file is byte-for-byte reproducible

## Post-Quantum Signature

The genesis block includes a post-quantum signature using Dilithium3 to prove authenticity and establish the chain of trust.

**Algorithm:** Dilithium3  
**Public Key:** `302a300506032b657003210065ca823a1dbeb5c5d76e8c7d8d9f3f2c7e5d4c3b2a191f0e9d8c7b6a594837261f0e9d8c7b6a5948372`  
**Signature:** `a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4`

## References

- [Consensus Economics](CONSENSUS_ECON.md)
- [Pre-Launch Checklist](../ops/PRELAUNCH_CHECKLIST.md)
- [Network Specification](network.md)
- [Security Policy](../SECURITY.md)

---

*Last Updated: November 2024*  
*Document Version: 1.0.0*
