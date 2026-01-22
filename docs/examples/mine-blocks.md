# Mine Blocks

This example shows you how to mine BitQuan blocks and receive coinbase rewards.

## Prerequisites

- BitQuan built from source
- Wallet address (for receiving rewards)
- 5 minutes

## Example 1: Mine with Mock PoW (Instant)

### Step 1: Get Your Address

```bash
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
```

### Expected Output

```
Decoded address: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q
Script: a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787
```

### Step 2: Mine 10 Blocks

```bash
./target/release/bitquan-node mine \
  --pow mock \
  --payout-script-hex a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787 \
  --limit-blocks 10 \
  --datadir ./data/chainstate
```

### Expected Output

```
Mining started with algorithm: mock

Mining block #1...
FOUND Block #1 | Nonce: 0 | Hash: 0abc123... | 0.00s | 0 H/s

Mining block #2...
FOUND Block #2 | Nonce: 0 | Hash: 0def456... | 0.00s | 0 H/s

Mining block #3...
FOUND Block #3 | Nonce: 0 | Hash: 0fed789... | 0.00s | 0 H/s
...
FOUND Block #10 | Nonce: 0 | Hash: 0xyzabc... | 0.00s | 0 H/s

Mining complete. 10 blocks mined.
```

### Step 3: Check Rewards

```bash
./target/release/bitquan-node balance \
  --address bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q \
  --datadir ./data/chainstate
```

### Expected Output

```
=== BitQuan Balance ===
Chain height: 10
...
UTXO count: 10
Balance: 500000000000000000000 qbits
Balance: 50.000000000000000000 BQ
```

**Note:** Each block = 5 BQ reward (on devnet)

## Example 2: Mine with SHA-256d (Real PoW)

### Step 1: Mine Single Block

```bash
./target/release/bitquan-node mine \
  --pow hashcash \
  --threads 4 \
  --payout-script-hex a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787 \
  --limit-blocks 1 \
  --datadir ./data/chainstate
```

### Expected Output

```
Mining started with algorithm: hashcash
Using 4 threads

Mining block #1...
Testing nonce: 0
Testing nonce: 1000000
Testing nonce: 2000000
...
FOUND Block #1 | Nonce: 12345678 | Hash: 000abc123... | 45.23s | 2.5 MH/s
```

### Timing Comparison

| Algorithm | Block Time | Hash Rate |
|-----------|------------|-----------|
| mock | ~0s | Instant |
| hashcash (SHA-256d) | ~30-60s | 1-3 MH/s |
| randomx | ~2-5 min | ~10-50 KH/s |

## Example 3: Continuous Mining

### Step 1: Start Continuous Miner

```bash
./target/release/bitquan-node mine \
  --pow mock \
  --threads 4 \
  --payout-script-hex a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787 \
  --datadir ./data/chainstate
```

### Expected Output

```
Mining started with algorithm: mock
Mining continuously (press Ctrl+C to stop)

Mining block #1...
FOUND Block #1 | Nonce: 0 | Hash: 0abc123...

Mining block #2...
FOUND Block #2 | Nonce: 0 | Hash: 0def456...

Mining block #3...
FOUND Block #3 | Nonce: 0 | Hash: 0fed789...
...
```

### Step 2: Stop Mining

Press `Ctrl+C` to stop.

## Example 4: Mine to Address (Convenience)

### Get Payout Script from Address

```bash
# Option 1: Use wallet-address to get script
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore

# Look for "Script:" line
# Script: a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787

# Option 2: Use default script (for testing)
./target/release/bitquan-node mine \
  --pow mock \
  --datadir ./data/chainstate
```

## Example 5: Mine for Maturity Testing

### Mine 101 Blocks (for coinbase maturity)

```bash
#!/bin/bash
# mine-for-maturity.sh - Mine 101 blocks for testing

echo "Mining 101 blocks for coinbase maturity test..."

./target/release/bitquan-node mine \
  --pow mock \
  --payout-script-hex a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787 \
  --limit-blocks 101 \
  --datadir ./data/chainstate

echo "Mining complete!"
echo "First block (#1) coins are now mature at block #101"

# Check balance
./target/release/bitquan-node balance \
  --address bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q \
  --datadir ./data/chainstate
```

## Coinbase Maturity

**Important:** Mined coins require **100 blocks** to mature before spending.

### Maturity Timeline

```
Block #1:  Mined (coinbase created)
Block #2:  Immature (99 blocks remaining)
Block #3:  Immature (98 blocks remaining)
...
Block #100: Immature (1 block remaining)
Block #101: MATURE! (can spend now)
```

### Check Which UTXOs Are Mature

```bash
./target/release/bitquan-node balance \
  --address <your-address> \
  --datadir ./data/chainstate
```

Look for lines like:
```
Block #6 TX ... vout=0 amount=50000000000000000000  # Mature if height >= 106
```

## Mining Algorithms

### Available Algorithms

| Algorithm | Description | Use Case |
|-----------|-------------|----------|
| **hashcash** | SHA-256d (Bitcoin-style) | Production, ASIC-friendly |
| **randomx** | RandomX (CPU-friendly) | Experimental, CPU mining |
| **mock** | Instant blocks | Testing, development |

### Select Algorithm

```bash
# SHA-256d (default)
./target/release/bitquan-node mine --pow hashcash

# RandomX
./target/release/bitquan-node mine --pow randomx

# Mock (testing)
./target/release/bitquan-node mine --pow mock
```

## Mining Performance

### Hash Rate Benchmarks

| CPU | SHA-256d | RandomX |
|-----|----------|---------|
| Intel i3 (4 cores) | ~500 MH/s | ~10 KH/s |
| Intel i5 (6 cores) | ~750 MH/s | ~15 KH/s |
| Intel i7 (8 cores) | ~1 GH/s | ~20 KH/s |
| AMD Ryzen 5 | ~800 MH/s | ~18 KH/s |
| AMD Ryzen 7 | ~1.1 GH/s | ~25 KH/s |

**Note:** MH/s = million hashes per second, KH/s = thousand hashes per second

### Optimize Performance

```bash
# Use all CPU cores
./target/release/bitquan-node mine \
  --pow hashcash \
  --threads 0 \
  --datadir ./data/chainstate

# Or specify thread count
./target/release/bitquan-node mine \
  --pow hashcash \
  --threads $(nproc) \
  --datadir ./data/chainstate
```

## Mining Payout

### Coinbase Reward

Each block mined pays reward to coinbase output:

| Network | Reward per Block |
|---------|-----------------|
| Devnet | 5 BQ |
| Testnet | 5 BQ |
| Mainnet | 50 BQ (planned, with halving) |

### Track Your Mining

```bash
# Count blocks you mined
grep "FOUND Block" bitquan.log | wc -l

# Show recent blocks
grep "FOUND Block" bitquan.log | tail -10

# Calculate total earnings
# (Blocks × Reward)
```

## Common Errors

### Error: Invalid Payout Script

```
Error: Invalid("Invalid payout script")
```

**Solution:** Use correct script format from `wallet-address` output.

```bash
# Get correct script
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore
# Copy the "Script:" line
```

### Error: No Peers Connected

```
Warning: No peers connected, block not broadcast
```

**Normal for devnet:** Block stored locally, no broadcast needed.

**For testnet/mainnet:** Connect to peers first.

```bash
./target/release/bitquan-node \
  --network testnet \
  --peers seed.testnet.bitquan.org:19444
```

### Error: Database Corrupted

```
Error: DatabaseCorruption
```

**Solution:** Reset chainstate (CAUTION: loses data).

```bash
pkill bitquan-node
rm -rf ./data/chainstate
./target/release/bitquan-node mine --pow mock
```

## Mining Profitability

### Solo Mining Reality Check

**CPU solo mining on mainnet:**
- Profitability: Essentially zero
- Chance to find block: Very low
- Recommendation: Join mining pool or don't expect profit

**For testing:** Perfectly fine to CPU mine on devnet/testnet.

### For Profit

1. **Use ASIC hardware** (SHA-256d)
2. **Join mining pool** (combine hashrate)
3. **Consider electricity costs**
4. **Monitor market conditions**

**DO NOT:** Expect profit from CPU solo mining.

## Complete Mining Script

```bash
#!/bin/bash
# mine-and-check.sh - Mine blocks and verify rewards

set -e

# Configuration
SCRIPT="a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787"
ADDRESS="bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q"
DATADIR="./data/chainstate"
BLOCKS=10

echo "Mining $BLOCKS blocks..."

# Mine blocks
./target/release/bitquan-node mine \
  --pow mock \
  --payout-script-hex "$SCRIPT" \
  --limit-blocks "$BLOCKS" \
  --datadir "$DATADIR"

echo "Mining complete!"
echo ""

# Check balance
echo "Checking balance..."
./target/release/bitquan-node balance \
  --address "$ADDRESS" \
  --datadir "$DATADIR"

echo ""
echo "Total mined: $BLOCKS blocks"
echo "Expected reward: $((BLOCKS * 5)) BQ"
```

**Usage:**
```bash
chmod +x mine-and-check.sh
./mine-and-check.sh
```

## What's Next?

- [Create Wallet](create-wallet.md) - Generate wallet
- [Run Node](run-node.md) - Start your node
- [Send Transaction](send-transaction.md) - Spend your mined coins
- [Mining Issues](../troubleshooting/mining-issues.md) - Mining problems

## Related Documentation

- [Pool Operations](../POOL_OPERATIONS.md) - Mining pool setup
- [Stratum Protocol](../guides/STRATUM.md) - Pool mining protocol
- [Mining Issues](../troubleshooting/mining-issues.md) - Troubleshooting
- [FAQ](../troubleshooting/faq.md) - Mining questions
