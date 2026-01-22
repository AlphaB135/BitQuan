# Troubleshooting Guide

This section helps you diagnose and fix common issues when running BitQuan nodes, wallets, or miners.

## Quick Diagnostic Checklist

Before diving into specific issues, run through this checklist:

1. **Network Configuration**
   - [ ] Verify you're on the right network (devnet/testnet/mainnet)
   - [ ] Check your config file points to correct ports
   - [ ] Ensure firewall allows P2P port (default: 18444)

2. **System Resources**
   - [ ] Check available disk space (need 10GB+ for full sync)
   - [ ] Verify RocksDB is accessible (datadir permissions)
   - [ ] Monitor memory usage (need 2GB+ RAM minimum)

3. **Logs**
   - [ ] Check node logs for error messages
   - [ ] Look for "connection refused" or "timeout" errors
   - [ ] Verify log level is appropriate (info/debug)

4. **Dependencies**
   - [ ] Rust version 1.82+ installed
   - [ ] All Cargo dependencies built successfully
   - [ ] System libraries available (RocksDB, etc.)

## When to Ask for Help

### Self-Serve (Check these first)
- [Specific troubleshooting guides below](#troubleshooting-guides)
- [FAQ](faq.md) for common questions
- [Documentation index](../README.md)

### Community Support
- Open a [GitHub Discussion](https://github.com/AlphaB135/BitQuan/discussions)
- Check existing [Issues](https://github.com/AlphaB135/BitQuan/issues)

### Bug Reports
If you've found a bug, file an [Issue](https://github.com/AlphaB135/BitQuan/issues/new) with:
- Your BitQuan version (`bitquan-node --version`)
- Full error messages and logs
- Steps to reproduce
- System information (OS, Rust version)

## Troubleshooting Guides

### Sync Issues
[sync-issues.md](sync-issues.md) - Chain not syncing? Stuck at certain block height?

**Common symptoms:**
- Block height not increasing
- "Failed to download block" errors
- Slow sync speed

### Network Issues
[network-issues.md](network-issues.md) - Can't connect to peers? Connection timeouts?

**Common symptoms:**
- "Connection refused" errors
- Zero peer connections
- Handshake failures

### Wallet Issues
[wallet-issues.md](wallet-issues.md) - Keystore or mnemonic problems? Can't access funds?

**Common symptoms:**
- "Invalid password" errors
- Can't decrypt keystore
- Mnemonic not generating correct address
- Balance showing 0 after mining

### Mining Issues
[mining-issues.md](mining-issues.md) - Mining not working? Low hash rate? Rejected shares?

**Common symptoms:**
- "Failed to submit block" errors
- Zero blocks found
- Low hash rate
- Stale blocks

## FAQ

[faq.md](faq.md) - Frequently asked questions about BitQuan.

**Quick answers to:**
- Is mainnet live?
- Why is my balance 0?
- How do I connect to testnet?
- What's the difference between hashcash and RandomX?

## Emergency Procedures

For critical issues (security breaches, active attacks), see:
- [Security Emergency Procedures](../security/EMERGENCY_PROCEDURES.md)
- [Emergency Quick Reference](../security/EMERGENCY_QUICK_REFERENCE.md)

## Getting Help Flowchart

```
          Issue occurs
              |
              v
        Check logs above
              |
              v
      Issue resolved? ──No──> Check specific guide below
              |                     |
             Yes                   No
              |                     |
              v                     v
           Done              Check FAQ ──No──> Open GitHub Discussion
                                    |                    |
                                   Yes                  No
                                    |                    |
                                    v                    v
                                 Done           File GitHub Issue
```

## Useful Commands for Diagnostics

```bash
# Check node status
./target/release/bitquan-node --version

# View current block height
./target/release/bitquan-node info --datadir ./data/chainstate

# Check wallet address
./target/release/bitquan-node wallet-address --keystore my-wallet.keystore

# Test network connectivity
telnet <peer-ip> 18444

# View logs (if using file logging)
tail -f bitquan.log

# Check disk usage
du -sh ./data/chainstate

# Check Rust version
rustc --version
```

## Common Error Messages

| Error | Meaning | See Also |
|-------|---------|----------|
| `Connection refused` | Firewall or port issue | [Network Issues](network-issues.md) |
| `Invalid password` | Wrong keystore password | [Wallet Issues](wallet-issues.md) |
| `Failed to decode block` | Corrupted data or version mismatch | [Sync Issues](sync-issues.md) |
| `Insufficient funds` | Coinbase not mature or no UTXOs | [FAQ](faq.md) |
| `Handshake timeout` | Peer incompatible or network issue | [Network Issues](network-issues.md) |
