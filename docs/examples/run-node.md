# Run a BitQuan Node

This example shows you how to start and operate a BitQuan node.

## Prerequisites

- BitQuan built from source
- 10 minutes

**Build if needed:**
```bash
cd BitQuan
cargo build --release
```

## Example 1: Start Node (Foreground)

### Step 1: Start Node

```bash
./target/release/bitquan-node --network devnet
```

### Expected Output

```
BitQuan Node v1.0-audit-20251122

Starting BitQuan node...
Network: devnet
P2P Port: 18444
RPC Port: 18443
Data Directory: ./data/chainstate

Loading blockchain...
Blockchain loaded: 0 blocks

Starting P2P server...
P2P server listening on 0.0.0.0:18444

Starting RPC server...
RPC server listening on 127.0.0.1:18443

Node ready! (Press Ctrl+C to stop)
```

### Step 2: Check Node Status

In another terminal:

```bash
./target/release/bitquan-node info --datadir ./data/chainstate
```

### Expected Output

```
=== BitQuan Node Info ===
Version: v1.0-audit-20251122
Network: devnet
Chain Height: 0
Best Block: (none)
Peers Connected: 0
Sync Status: Not syncing
```

### Step 3: Stop Node

Press `Ctrl+C` in the terminal running the node.

## Example 2: Start Node (Background)

### Step 1: Start Node in Background

```bash
./target/release/bitquan-node --network devnet > bitquan.log 2>&1 &
```

### Step 2: Check Process

```bash
ps aux | grep bitquan-node
```

### Expected Output

```
user  12345  bitquan-node --network devnet
```

### Step 3: Check Logs

```bash
tail -f bitquan.log
```

### Step 4: Stop Node

```bash
pkill bitquan-node
```

## Example 3: Start Node with Config File

### Step 1: Create Config File

```bash
cat > config/my-node.toml << EOF
[network]
magic = "devnet"

[p2p]
listen_address = "0.0.0.0:18444"
max_peers = 50

[rpc]
enabled = true
listen_address = "127.0.0.1:18443"

[storage]
path = "./data/chainstate"
EOF
```

### Step 2: Start with Config

```bash
./target/release/bitquan-node --config config/my-node.toml
```

## Example 4: Connect to Peers

### Step 1: Start with Specific Peers

```bash
./target/release/bitquan-node \
  --network devnet \
  --peers 127.0.0.1:18444 \
  --peers 192.168.1.100:18444
```

### Step 2: Check Peer Connections

```bash
grep "peer" bitquan.log | tail -10
```

### Expected Output

```
[INFO] Connected to peer: 127.0.0.1:18444
[INFO] Peer count: 1
[INFO] Handshake successful with 127.0.0.1:18444
```

## Example 5: Check Balance

### Step 1: Check Your Balance

```bash
./target/release/bitquan-node balance \
  --address bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q \
  --datadir ./data/chainstate
```

### Expected Output

```
=== BitQuan Balance ===
Chain height: 116
Decoded address: bq1q9ssgzwt99pazd7rskw05dfycepc2me6hwu8653fmztzrgm382q6wsms93q
Pubkey hash: 610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a7

Scanning blockchain for UTXOs...
 Block #6 TX ... vout=0 amount=50000000000000000000
 Block #7 TX ... vout=0 amount=50000000000000000000
...

UTXO count: 100
Balance: 5000000000000000000000 qbits
Balance: 50.000000000000000000 BQ
```

## Example 6: Node with Mining

### Step 1: Start Mining Node

```bash
./target/release/bitquan-node \
  --network devnet \
  --datadir ./data/chainstate \
  mine \
  --pow mock \
  --payout-script-hex a820610409cb2943d137c3859cfa3524c643856f3abbb87d5229d89621a3713a81a787
```

### Expected Output

```
Starting BitQuan node with mining...
Mining started with algorithm: mock

Mining block #1...
FOUND Block #1 | Nonce: 0 | Hash: abc123...

Mining block #2...
FOUND Block #2 | Nonce: 0 | Hash: def456...

Mining block #3...
...
```

## Example 7: Systemd Service (Linux)

### Step 1: Create Service File

```bash
sudo nano /etc/systemd/system/bitquan.service
```

### Step 2: Add Service Configuration

```ini
[Unit]
Description=BitQuan Node
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/home/your-username/BitQuan
ExecStart=/home/your-username/BitQuan/target/release/bitquan-node \
  --network devnet \
  --datadir /home/your-username/bitquan-data
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Step 3: Enable and Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable auto-start on boot
sudo systemctl enable bitquan

# Start service
sudo systemctl start bitquan

# Check status
sudo systemctl status bitquan

# View logs
sudo journalctl -u bitquan -f
```

## Node Operations

### Check Node Health

```bash
# General info
./target/release/bitquan-node info --datadir ./data/chainstate

# Block count
./target/release/bitquan-node getblockcount --datadir ./data/chainstate

# Best block hash
./target/release/bitquan-node getbestblock --datadir ./data/chainstate
```

### Monitor Logs

```bash
# Follow logs in real-time
tail -f bitquan.log

# Search for errors
grep -i error bitquan.log

# Search for peer connections
grep -i peer bitquan.log | tail -20

# Search for blocks
grep "FOUND Block" bitquan.log
```

### Backup Node Data

```bash
# Stop node first
pkill bitquan-node

# Backup chainstate
tar -czf chainstate-backup-$(date +%Y%m%d).tar.gz ./data/chainstate

# Restart node
./target/release/bitquan-node --network devnet
```

### Reset Node (CAUTION)

```bash
# Stop node
pkill bitquan-node

# WARNING: This deletes all blockchain data!
rm -rf ./data/chainstate

# Restart node (will start from genesis)
./target/release/bitquan-node --network devnet
```

## Network Configurations

| Network | Magic | P2P Port | RPC Port | Use |
|---------|-------|----------|----------|-----|
| Devnet | devnet | 18444 | 18443 | Development |
| Testnet | testnet | 19444 | 19443 | Testing |
| Mainnet | mainnet | 18444 | 18443 | Production |

**Start with specific network:**
```bash
./target/release/bitquan-node --network testnet
```

## Common Errors

### Error: Address Already in Use

```
Error: Os { code: 98, kind: AddrInUse, message: "Address already in use" }
```

**Cause:** Port already in use by another process.

**Solution:**
```bash
# Find process using port
lsof -i :18444

# Kill existing process
pkill bitquan-node

# Or use different port
./target/release/bitquan-node --p2p-port 18445
```

### Error: Permission Denied

```
Error: Permission denied (os error 13)
```

**Cause:** No write permission to data directory.

**Solution:**
```bash
# Fix permissions
chmod 755 ./data
chmod 755 ./data/chainstate

# Or use different directory
./target/release/bitquan-node --datadir /tmp/bitquan
```

### Error: Database Locked

```
Error: DatabaseLocked
```

**Cause:** Another bitquan-node process is using the database.

**Solution:**
```bash
# Kill all bitquan-node processes
pkill bitquan-node

# Wait a few seconds
sleep 3

# Restart
./target/release/bitquan-node --network devnet
```

## Monitoring Your Node

### Health Check Script

```bash
#!/bin/bash
# health-check.sh - Check node health

echo "BitQuan Node Health Check"
echo "========================="

# Check if process is running
if pgrep -x bitquan-node > /dev/null; then
    echo "Process: RUNNING"
else
    echo "Process: NOT RUNNING"
    exit 1
fi

# Check if responding
if ./target/release/bitquan-node info --datadir ./data/chainstate > /dev/null 2>&1; then
    echo "API: RESPONSIVE"
else
    echo "API: NOT RESPONDING"
    exit 1
fi

# Get block count
HEIGHT=$(./target/release/bitquan-node getblockcount --datadir ./data/chainstate)
echo "Block Height: $HEIGHT"

# Check recent logs
ERRORS=$(grep -i error bitquan.log | tail -5)
if [ -z "$ERRORS" ]; then
    echo "Recent Errors: None"
else
    echo "Recent Errors:"
    echo "$ERRORS"
fi

echo "========================="
echo "Health Check Complete"
```

### Resource Monitoring

```bash
# CPU usage
top -p $(pgrep bitquan-node)

# Memory usage
ps aux | grep bitquan-node

# Disk usage
du -sh ./data/chainstate

# Network connections
netstat -an | grep 18444
```

## Node Maintenance

### Regular Tasks

**Daily:**
- Check node is running
- Monitor disk space
- Review error logs

**Weekly:**
- Backup chainstate
- Update BitQuan (if new release)
- Review peer connections

**Monthly:**
- Full system backup
- Security audit
- Performance review

### Update Node

```bash
# Stop node
pkill bitquan-node

# Update code
cd BitQuan
git pull origin main
cargo build --release

# Restart node
./target/release/bitquan-node --network devnet
```

## Production Considerations

For production/mainnet deployment, see:
- [Mainnet Deployment](../operations/mainnet-deployment.md)
- [VPS Deployment](../operations/vps-deployment.md)
- [Pre-Launch Checklist](../ops/PRELAUNCH_CHECKLIST.md)

## What's Next?

- [Mine Blocks](mine-blocks.md) - Start mining
- [Send Transaction](send-transaction.md) - Send coins
- [RPC Calls](rpc-calls.md) - Use API
- [Troubleshooting](../troubleshooting/) - Fix problems

## Related Documentation

- [Operations Guide](../operations/README.md) - Node operations
- [Network Issues](../troubleshooting/network-issues.md) - P2P problems
- [Sync Issues](../troubleshooting/sync-issues.md) - Sync problems
