# Mining Issues Troubleshooting

Mining not working? Low hash rate? Rejected shares? This guide helps diagnose and fix mining-related problems.

## Symptoms

- Zero blocks found
- "Failed to submit block" errors
- Low hash rate
- "Stale block" warnings
- Rejected shares
- Mining process crashes

## Diagnostic Steps

### 1. Verify Mining is Running

```bash
# Check if mining process is active
ps aux | grep bitquan-node

# Should show bitquan-node process with "mine" command
```

### 2. Check Mining Logs

```bash
# Watch mining logs in real-time
tail -f bitquan.log | grep -i "mining\|block\|found"

# Should see messages like:
# "Mining started with hashcash algorithm"
# "Testing nonce: X"
# "FOUND Block #N"
```

### 3. Verify Network Connectivity

```bash
# Mining requires peer connection for block submission
grep "peer" bitquan.log | tail -10

# Should show active peer connections
```

## Common Issues and Solutions

### Issue: Zero Blocks Found

**Symptoms:**
- Mining running but no blocks found
- Hash rate showing 0 H/s

**Possible Causes:**

#### A. Wrong Algorithm

**What's happening:** Using incompatible PoW algorithm.

**Solution:**
```bash
# Check available algorithms
./target/release/bitquan-node mine --help

# Use hashcash (SHA-256d) for main compatibility
./target/release/bitquan-node mine \
  --pow hashcash \
  --datadir ./data/chainstate

# Or use mock for testing (instant blocks)
./target/release/bitquan-node mine \
  --pow mock \
  --datadir ./data/chainstate
```

#### B. Difficulty Too High

**What's happening:** Network difficulty exceeds your hashrate.

**Solution:**
```bash
# Mine on devnet (lower difficulty)
./target/release/bitquan-node mine \
  --network devnet \
  --datadir ./data/chainstate

# Or use mock PoW for testing
./target/release/bitquan-node mine \
  --pow mock \
  --datadir ./data/chainstate
```

#### C. Insufficient Threads

**What's happening:** Not using all CPU cores.

**Solution:**
```bash
# Use all CPU cores
./target/release/bitquan-node mine \
  --threads 0 \
  --datadir ./data/chainstate

# Or specify exact number
./target/release/bitquan-node mine \
  --threads $(nproc) \
  --datadir ./data/chainstate
```

### Issue: "Failed to Submit Block"

**Symptoms:**
- Block found but submission fails
- "Block rejected" or "Invalid block" error

**Possible Causes:**

#### A. Stale Block

**What's happening:** Someone else mined block while you were mining.

**Solution:**
- This is normal for competitive mining
- Continue mining, will find next block
- Improve hashrate to find blocks faster

#### B. Invalid Coinbase

**What's happening:** Coinbase transaction has invalid payout script.

**Solution:**
```bash
# Verify payout script is valid hex
./target/release/bitquan-node mine \
  --payout-script-hex 76a914<hash>88ac \
  --datadir ./data/chainstate

# Use wallet address to generate script
# (See wallet guide for getting address script)
```

#### C. Consensus Violation

**What's happening:** Block violates consensus rules.

**Solution:**
- Verify you're on latest BitQuan version
- Check logs for specific violation
- Report to [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)

### Issue: Low Hash Rate

**Symptoms:**
- Hash rate much lower than expected
- Blocks found very slowly

**Possible Causes:**

#### A. Single Threaded Mining

**What's happening:** Only using 1 CPU thread.

**Solution:**
```bash
# Use multiple threads
./target/release/bitquan-node mine \
  --threads 4 \
  --datadir ./data/chainstate

# Or use all available
./target/release/bitquan-node mine \
  --threads 0 \
  --datadir ./data/chainstate
```

#### B. Power Saving Mode

**What's happening:** CPU throttled due to power settings.

**Solution:**
```bash
# Linux: Set CPU governor to performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable CPU frequency scaling
# (See your OS documentation for details)
```

#### C. Background Processes

**What's happening:** Other processes using CPU.

**Solution:**
- Close unnecessary applications
- Stop background services
- Use dedicated mining machine if possible

### Issue: "Invalid Payout Script"

**Symptoms:**
- Mining won't start
- Error about invalid script_pubkey

**Solution:**
```bash
# Get correct script from wallet
# Method 1: Use wallet address directly (if supported)
./target/release/bitquan-node mine \
  --payout-address <your-address> \
  --datadir ./data/chainstate

# Method 2: Generate script manually
# Get address from wallet first
./target/release/bitquan-node wallet-address \
  --keystore my-wallet.keystore

# Use the script_pubkey hex shown
./target/release/bitquan-node mine \
  --payout-script-hex <script_hex> \
  --datadir ./data/chainstate

# Method 3: Use default (test/mining only)
./target/release/bitquan-node mine \
  --datadir ./data/chainstate
# Uses default script for testing
```

### Issue: Mining Process Crashes

**Symptoms:**
- Mining starts then process exits
- Segmentation fault or panic

**Possible Causes:**

#### A. Memory Issues

**What's happening:** Out of memory or corrupted memory.

**Solution:**
```bash
# Check available memory
free -h

# Close memory-intensive applications
# Add swap if needed

# Mine with single thread to reduce memory
./target/release/bitquan-node mine \
  --threads 1 \
  --datadir ./data/chainstate
```

#### B. Database Lock

**What's happening:** Multiple processes accessing database.

**Solution:**
```bash
# Stop all other bitquan-node processes
pkill bitquan-node

# Ensure only one mining process
./target/release/bitquan-node mine \
  --datadir ./data/chainstate
```

#### C. Bug in Mining Code

**What's happening:** Software bug causing crash.

**Solution:**
```bash
# Update to latest version
cd BitQuan
git pull origin main
cargo build --release

# File bug report with:
# - Backtrace (if segfault)
# - Log files
# - BitQuan version
# - OS and hardware info
```

## Mining Configuration Examples

### Basic CPU Mining

```bash
# Start mining with SHA-256d
./target/release/bitquan-node mine \
  --network devnet \
  --pow hashcash \
  --threads $(nproc) \
  --datadir ./data/chainstate
```

### Mining to Specific Address

```bash
# Mine to your wallet address
./target/release/bitquan-node mine \
  --payout-script-hex a820<your-pubkey-hash>87 \
  --datadir ./data/chainstate
```

### Test Mining (Instant Blocks)

```bash
# Use mock PoW for testing (instant blocks)
./target/release/bitquan-node mine \
  --pow mock \
  --datadir ./data/chainstate
```

### Limited Block Mining

```bash
# Mine only 10 blocks then stop
./target/release/bitquan-node mine \
  --pow mock \
  --limit-blocks 10 \
  --datadir ./data/chainstate
```

## Hash Rate Benchmarks

### Expected Hash Rates (SHA-256d)

| CPU | Hash Rate | Threads |
|-----|-----------|---------|
| Intel i3 (4 cores) | ~500 MH/s | 4 |
| Intel i5 (6 cores) | ~750 MH/s | 6 |
| Intel i7 (8 cores) | ~1 GH/s | 8 |
| AMD Ryzen 5 (6 cores) | ~800 MH/s | 6 |
| AMD Ryzen 7 (8 cores) | ~1.1 GH/s | 8 |

**Note:** These are estimates. Actual rates vary based on:
- CPU model and generation
- Background processes
- Power management settings
- Cooling (thermal throttling)

### RandomX Hash Rates

RandomX is CPU-friendly but generally slower than SHA-256d:

| CPU | Hash Rate | Notes |
|-----|-----------|-------|
| Modern CPU | ~10-50 KH/s | Depends on cache size |

**Recommendation:** Use SHA-256d (hashcash) for better performance.

## Mining Pool Issues

### "Pool Connection Failed"

**Symptoms:**
- Can't connect to mining pool
- "Connection refused" to pool

**Solution:**
```bash
# Verify pool is online
# Check pool status page or Discord

# Test pool connectivity
telnet <pool-host> <pool-port>

# Try different pool
# See mining pool documentation for server list
```

### "Share Rejected"

**Symptoms:**
- Pool rejecting your shares
- "Stale share" warnings

**Possible Causes:**
- Network latency to pool
- Pool difficulty too high
- Outdated mining software

**Solution:**
- Use pool geographically closer
- Adjust difficulty in pool settings
- Update to latest mining software

## Performance Tuning

### Optimize CPU Performance

```bash
# Set CPU governor to performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable CPU sleep states
# (Advanced, see your OS documentation)

# Set process priority
nice -n -5 ./target/release/bitquan-node mine \
  --datadir ./data/chainstate
```

### Monitor Mining Performance

```bash
# Watch log output
tail -f bitquan.log | grep --line-buffered "FOUND\|Hash rate"

# Count blocks found
grep "FOUND Block" bitquan.log | wc -l

# Calculate average hash rate
grep "Hash rate" bitquan.log | awk '{sum+=$3; count++} END {print sum/count}'
```

### Thermal Monitoring

```bash
# Check CPU temperature
# Linux
sensors | grep Core

# macOS
sudo powermetrics --samplers cpu_power

# Watch for thermal throttling
watch -n 5 sensors
```

## Prevention Tips

1. **Cooling:** Ensure adequate CPU cooling
2. **Power:** Use stable power supply
3. **Monitoring:** Log mining statistics
4. **Backups:** Backup wallet after each block found
5. **Updates:** Keep BitQuan updated

## Mining Profitability

**IMPORTANT: Solo mining is NOT profitable on mainnet unless you have:**

- Massive hash rate (ASICs or large GPU farm)
- Or extreme luck

**For CPU mining:**
- Consider joining mining pool
- Or mine on testnet for learning
- Or use mock PoW for testing

**DO NOT:** Expect profit from CPU solo mining on mainnet.

## Still Having Issues?

1. **Gather Diagnostic Info:**
   ```bash
   # Mining diagnostics
   ps aux | grep mine > mining-diag.txt
   grep "mining" bitquan.log | tail -50 >> mining-diag.txt
   ./target/release/bitquan-node --version >> mining-diag.txt

   # System info
   lscpu >> mining-diag.txt
   free -h >> mining-diag.txt
   ```

2. **Check for Known Issues:**
   - [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)

3. **Report Bugs:**
   - Include full diagnostic output
   - Attach relevant logs
   - Describe expected vs actual behavior

## Related Guides

- [Pool Operations](../POOL_OPERATIONS.md) - Mining pool setup
- [Stratum Guide](../guides/STRATUM.md) - Stratum protocol
- [FAQ](faq.md) - "Which algorithm should I use?"
