# Sync Issues Troubleshooting

Chain not syncing? Stuck at certain block height? This guide helps diagnose and fix synchronization problems.

## Symptoms

- Block height not increasing
- "Failed to download block" errors
- Slow sync speed
- Chain stuck at specific height
- "Invalid block" errors

## Diagnostic Steps

### 1. Check Current Status

```bash
# Check current block height
./target/release/bitquan-node info --datadir ./data/chainstate

# Check if sync is active
# Look for "Syncing..." in logs or process status
```

### 2. Check Logs

```bash
# If running in foreground, check console output
# If using file logging:
tail -f bitquan.log | grep -i "sync\|block\|error"
```

### 3. Verify Database Integrity

```bash
# Check RocksDB files exist
ls -la ./data/chainstate/

# Check for corruption indicators
# (Corruption may show as "Bad block" or "Decode error")
```

## Common Issues and Solutions

### Issue: No Peers Connected

**Symptoms:**
- Sync not starting
- "No peers available" in logs

**Solution:**
1. Check network connectivity
2. Verify P2P port is open (default: 18444)
3. Add seed nodes manually with `--peers` flag
4. See [Network Issues](network-issues.md) for detailed troubleshooting

### Issue: Sync Stuck at Specific Height

**Symptoms:**
- Sync progresses then stops at same block
- "Invalid block" or "Consensus error" in logs

**Possible Causes:**

#### A. Fork/Reorg in Progress

**What's happening:** Network is reorganizing to a different chain tip.

**Solution:**
- Wait for reorg to complete (usually 5-10 minutes)
- Do not interrupt the process
- Logs will show "Reorg detected" or "Switching to chain"

#### B. Invalid Block in Database

**What's happening:** Corrupted block data in local database.

**Solution:**
```bash
# Stop the node
pkill bitquan-node

# Backup current chainstate
mv ./data/chainstate ./data/chainstate.backup

# Restart node (will resync from genesis)
./target/release/bitquan-node --network devnet --datadir ./data/chainstate
```

#### C. Consensus Rule Violation

**What's happening:** Node received invalid block (possible attack or bug).

**Solution:**
- Check if you're on latest version: `bitquan-node --version`
- Verify network matches peers (`--network` flag)
- Report to [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues) with block hash

### Issue: Slow Sync Speed

**Symptoms:**
- Sync progressing but very slowly (< 1 block/minute)

**Possible Causes:**

#### A. Slow Disk I/O

**Diagnosis:**
```bash
# Check disk speed
dd if=/dev/zero of=test.tmp bs=1M count=100
rm test.tmp
# Should be > 100 MB/s for SSD, > 30 MB/s for HDD
```

**Solution:**
- Use SSD instead of HDD if possible
- Close other disk-intensive applications
- Consider reducing log verbosity

#### B. Network Bandwidth Limited

**Diagnosis:**
```bash
# Test download speed
curl -o /dev/null http://speedtest.tele2.net/100MB.zip
```

**Solution:**
- Ensure sufficient bandwidth (min 10 Mbps recommended)
- Check for other bandwidth-heavy applications
- Verify ISP isn't throttling P2P traffic

#### C. Single Peer Connection

**Diagnosis:**
```bash
# Check peer count in logs
grep "peer" bitquan.log | tail -20
```

**Solution:**
- Add more peers with `--peers` flag
- Check firewall isn't blocking connections
- See [Network Issues](network-issues.md)

### Issue: "Failed to Decode Block"

**Symptoms:**
- Error: "Failed to decode block at height X"
- Sync stops at specific block

**Possible Causes:**

#### A. Version Mismatch

**What's happening:** Your node version is incompatible with network.

**Solution:**
```bash
# Check your version
./target/release/bitquan-node --version

# Rebuild from latest source
cd BitQuan
git pull origin main
cargo build --release
```

#### B. Corrupted Block Data

**What's happening:** Downloaded block data was corrupted or incomplete.

**Solution:**
```bash
# Stop node
pkill bitquan-node

# Delete corrupted block and re-sync
# (Note: This requires manual database surgery)
# Easier: Wipe chainstate and resync from genesis

mv ./data/chainstate ./data/chainstate.$(date +%s).corrupt
./target/release/bitquan-node --network devnet --datadir ./data/chainstate
```

### Issue: "Block Rejected" During Mining

**Symptoms:**
- You mine a block but it gets rejected
- "Invalid coinbase" or "Invalid signature" errors

**Solution:**
- Verify payout script is valid: `bitquan-node mine --help`
- Check coinbase maturity (100 blocks)
- Ensure you're mining on correct network
- See [Mining Issues](mining-issues.md) for more

## Advanced Diagnostics

### Check Block Validity

```bash
# Get block hash at specific height
./target/release/bitquan-node getblockhash <height> --datadir ./data/chainstate

# Get block details
./target/release/bitquan-node getblock <hash> --datadir ./data/chainstate
```

### Verify Chain Tip

```bash
# Check current best block
./target/release/bitquan-node getbestblock --datadir ./data/chainstate

# Compare with peers (ask in Discord/GitHub)
```

### Database Inspection

```bash
# Check RocksDB stats
# (Requires RocksDB CLI tools or custom script)

# Estimate database size
du -sh ./data/chainstate/
```

## Prevention Tips

1. **Regular Backups:** Backup `chainstate/` directory periodically
2. **Stable Power:** Use UPS to prevent corruption during power loss
3. **Sufficient Disk:** Keep 20GB+ free space
4. **Update Regularly:** Keep BitQuan updated to latest version
5. **Monitor Logs:** Check logs weekly for early warning signs

## When to Resync from Genesis

**Resync is recommended when:**
- Database is corrupted beyond repair
- Fork is too deep to recover
- Major version upgrade with breaking changes

**Backup before resyncing:**
```bash
# Archive old chainstate
tar -czf chainstate.backup.$(date +%Y%m%d).tar.gz ./data/chainstate/

# Start fresh
rm -rf ./data/chainstate/
./target/release/bitquan-node --network devnet --datadir ./data/chainstate
```

## Still Having Issues?

If none of the above solutions work:

1. **Gather Diagnostic Info:**
   ```bash
   bitquan-node --version > diagnostics.txt
   echo "Block height:" >> diagnostics.txt
   bitquan-node info --datadir ./data/chainstate >> diagnostics.txt
   echo "Recent errors:" >> diagnostics.txt
   tail -100 bitquan.log | grep -i error >> diagnostics.txt
   ```

2. **Search Existing Issues:**
   - [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)

3. **Open New Issue:**
   - Include `diagnostics.txt`
   - Describe what you were doing when issue started
   - Attach relevant log snippets

## Related Guides

- [Network Issues](network-issues.md) - Peer connection problems
- [Mining Issues](mining-issues.md) - Block submission problems
- [FAQ](faq.md) - "Why is sync slow?"
