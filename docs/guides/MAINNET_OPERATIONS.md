# BitQuan Mainnet Operations Guide

## Overview

This guide covers day-to-day operations of a BitQuan mainnet node, including monitoring, maintenance, and troubleshooting for production environments.

## Node Management

### Starting and Stopping

```bash
# Start node
sudo systemctl start bitquan

# Stop node gracefully
sudo systemctl stop bitquan

# Restart node
sudo systemctl restart bitquan

# Check status
sudo systemctl status bitquan
```

### Configuration Updates

```bash
# Edit configuration
sudo nano /etc/bitquan/mainnet.toml

# Validate configuration
bitquan-node --config /etc/bitquan/mainnet.toml --check

# Apply changes
sudo systemctl restart bitquan
```

## Monitoring

### Essential Metrics

Monitor these key metrics regularly:

#### Blockchain Status
```bash
# Current block height
bitquan-cli getblockcount

# Chain tip info
bitquan-cli getblockchaininfo

# Network difficulty
bitquan-cli getdifficulty
```

#### Network Health
```bash
# Peer connections
bitquan-cli getpeerinfo | jq '.length'

# Network traffic
bitquan-cli getnettotals

# Bandwidth usage
bitquan-cli getnetworkinfo
```

#### Mempool Status
```bash
# Mempool size
bitquan-cli getmempoolinfo

# Transaction count
bitquan-cli getrawmempool | jq '.length'

# Fee rates
bitquan-cli getmempoolinfo | jq '.fee_rate'
```

### Prometheus Monitoring

Configure metrics collection:

```toml
[metrics]
enable = true
bind = "0.0.0.0:9100"
namespace = "bitquan"
```

#### Key Metrics to Track

- `bitquan_block_height` - Current blockchain height
- `bitquan_peers_connected` - Number of connected peers
- `bitquan_mempool_size` - Transactions in mempool
- `bitquan_mempool_bytes` - Mempool size in bytes
- `bitquan_rpc_requests_total` - RPC request count
- `bitquan_cpu_usage_percent` - CPU utilization
- `bitquan_memory_usage_bytes` - Memory usage

#### Grafana Dashboard

Sample Grafana queries:

```promql
# Block sync progress
rate(bitquan_block_height[5m])

# Peer connections over time
bitquan_peers_connected

# Mempool size trend
bitquan_mempool_size

# RPC request rate
rate(bitquan_rpc_requests_total[1m])
```

## Maintenance

### Regular Tasks

#### Daily
```bash
# Check node health
bitquan-cli getblockchaininfo

# Review logs for errors
sudo journalctl -u bitquan --since "24 hours ago" | grep ERROR

# Monitor disk space
df -h /var/lib/bitquan
```

#### Weekly
```bash
# Check for updates
git fetch --tags
git describe --tags `git rev-list --tags --max-count=1`

# Review peer connections
bitquan-cli getpeerinfo | jq '.[] | {addr: .addr, version: .version, pingtime: .pingtime}'

# Backup wallet (if applicable)
bitquan-wallet backup /backup/wallet-$(date +%Y%m%d).dat
```

#### Monthly
```bash
# Full system backup
sudo systemctl stop bitquan
sudo tar -czf /backup/bitquan-full-$(date +%Y%m%d).tar.gz /var/lib/bitquan
sudo systemctl start bitquan

# Review performance metrics
bitquan-cli getmininginfo
bitquan-cli getnetworkinfo
```

### Database Maintenance

#### Compaction
```bash
# Trigger database compaction (monthly)
bitquan-cli compactdatabase

# Monitor during compaction
sudo journalctl -u bitquan -f
```

#### Cache Management
```bash
# Clear caches if memory pressure
bitquan-cli clearcaches

# Adjust cache size if needed
# Edit /etc/bitquan/mainnet.toml
[storage]
cache_size = "512MB"  # Adjust based on available RAM
```

## Security Operations

### Access Control

#### RPC Security
```bash
# Generate secure RPC credentials
openssl rand -hex 16  # For username
openssl rand -hex 32  # For password

# Update configuration
[network.rpc]
username = "generated_username"
password = "generated_password"
bind = "127.0.0.1:8332"  # Local only
```

#### Network Security
```bash
# Block malicious IPs
sudo ufw deny from malicious.ip

# Rate limit connections
sudo ufw limit 8333/tcp

# Monitor connection attempts
sudo journalctl -u bitquan | grep "connection from"
```

### Key Management

#### Wallet Security
```bash
# Encrypt wallet
bitquan-wallet encryptwallet "strong_passphrase"

# Lock wallet
bitquan-wallet walletlock

# Backup encrypted wallet
bitquan-wallet backup /secure/backup/wallet.dat
```

#### Node Keys
```bash
# Backup node private key
sudo cp /var/lib/bitquan/node_key /secure/backup/

# Generate new node key (if compromised)
rm /var/lib/bitquan/node_key
sudo systemctl restart bitquan
```

## Performance Optimization

### Hardware Tuning

#### CPU Optimization
```bash
# Set CPU governor to performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Pin processes to CPU cores
taskset -c 0-3 bitquan-node  # Use first 4 cores
```

#### Memory Optimization
```bash
# Configure swap usage
echo vm.swappiness=10 | sudo tee -a /etc/sysctl.conf

# Optimize memory management
echo vm.vfs_cache_pressure=50 | sudo tee -a /etc/sysctl.conf
```

#### Storage Optimization
```bash
# Optimize SSD
echo noop | sudo tee /sys/block/sdX/queue/scheduler

# Mount with noatime for better performance
# In /etc/fstab:
/dev/sdX1 /var/lib/bitquan ext4 noatime 0 2
```

### Network Optimization

#### TCP Tuning
```bash
# Increase connection limits
echo net.core.somaxconn=4096 | sudo tee -a /etc/sysctl.conf
echo net.ipv4.tcp_max_syn_backlog=4096 | sudo tee -a /etc/sysctl.conf

# Optimize for high throughput
echo net.core.rmem_max=16777216 | sudo tee -a /etc/sysctl.conf
echo net.core.wmem_max=16777216 | sudo tee -a /etc/sysctl.conf
```

#### Peer Management
```bash
# Add high-quality peers
bitquan-cli addnode "fast-peer.example.com:8333" "add"

# Disconnect slow peers
bitquan-cli disconnectnode "slow-peer.example.com:8333"

# View peer quality
bitquan-cli getpeerinfo | jq '.[] | select(.pingtime > 1.0)'
```

## Troubleshooting

### Common Issues

#### Sync Problems
```bash
# Check sync status
bitquan-cli getblockchaininfo | jq '.initial_block_download'

# Reset sync (last resort)
sudo systemctl stop bitquan
rm -rf /var/lib/bitquan/peers.dat
sudo systemctl start bitquan
```

#### Memory Issues
```bash
# Check memory usage
free -h
ps aux | grep bitquan-node

# Reduce cache if needed
# Edit config:
[storage]
cache_size = "128MB"
```

#### Network Issues
```bash
# Check connectivity
nc -zv peer.example.com 8333

# Test DNS resolution
nslookup dns-seed.bitquan.org

# Check firewall
sudo ufw status verbose
```

### Log Analysis

#### Important Log Messages
```bash
# View recent errors
sudo journalctl -u bitquan --since "1 hour ago" -p err

# Monitor block production
sudo journalctl -u bitquan -f | grep "New block"

# Track peer connections
sudo journalctl -u bitquan -f | grep "connected"
```

#### Performance Debugging
```bash
# Enable debug logging
# Edit config:
[logging]
level = "debug"

# Profile with perf
sudo perf record -g $(pidof bitquan-node)
sudo perf report
```

## Emergency Procedures

### Fork Handling
```bash
# Check for fork
bitquan-cli getblockchaininfo | jq '.verificationprogress'

# Choose correct chain
bitquan-cli reconsiderblock "correct_block_hash"

# Invalidate wrong chain
bitquan-cli invalidateblock "wrong_block_hash"
```

### Recovery from Corruption
```bash
# Stop node
sudo systemctl stop bitquan

# Check database integrity
bitquan-node --reindex

# Restore from backup if needed
sudo rm -rf /var/lib/bitquan/*
sudo tar -xzf /backup/bitquan-backup.tar.gz -C /var/lib/bitquan/

# Restart
sudo systemctl start bitquan
```

### Security Incident Response
```bash
# Isolate node
sudo ufw deny in
sudo systemctl stop bitquan

# Preserve evidence
sudo cp /var/lib/bitquan /evidence/bitquan-$(date +%s)

# Change credentials
# Generate new RPC username/password
# Update configuration

# Restore from clean backup
# Reinstall from scratch if needed
```

## Automation

### Health Check Script

```bash
#!/bin/bash
# health_check.sh

# Check if node is running
if ! systemctl is-active --quiet bitquan; then
    echo "ERROR: BitQuan node is not running"
    exit 1
fi

# Check sync status
HEIGHT=$(bitquan-cli getblockcount)
if [ $? -ne 0 ]; then
    echo "ERROR: Cannot get block height"
    exit 1
fi

# Check peer connections
PEERS=$(bitquan-cli getpeerinfo | jq '.length')
if [ "$PEERS" -lt 3 ]; then
    echo "WARNING: Low peer count: $PEERS"
fi

echo "OK: Height $HEIGHT, Peers $PEERS"
```

### Backup Automation

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/bitquan"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup
sudo systemctl stop bitquan
sudo tar -czf "$BACKUP_DIR/bitquan_$DATE.tar.gz" /var/lib/bitquan
sudo systemctl start bitquan

# Clean old backups (keep 7 days)
find "$BACKUP_DIR" -name "bitquan_*.tar.gz" -mtime +7 -delete

echo "Backup completed: bitquan_$DATE.tar.gz"
```

## Support and Resources

### Getting Help
- **Documentation**: https://docs.bitquan.org
- **GitHub**: https://github.com/bitquan/bitquan/issues
- **Community**: https://discord.gg/bitquan
- **Security**: security@bitquan.org

### Monitoring Tools
- **BitQuan Explorer**: https://explorer.bitquan.org
- **Network Stats**: https://stats.bitquan.org
- **Node Monitor**: https://monitor.bitquan.org

### Best Practices
1. **Always backup** before upgrades
2. **Monitor** key metrics daily
3. **Test** configuration changes on testnet first
4. **Keep software** updated
5. **Use strong authentication** for RPC access
6. **Monitor disk space** regularly
7. **Document** any custom configurations
