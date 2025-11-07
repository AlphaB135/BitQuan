# Operations Documentation

**Last Updated: 2025-01-07**

This section contains operational guides, runbooks, monitoring setup, and production deployment documentation for BitQuan.

## 📋 Production Readiness

### Pre-Launch
- **[Pre-Launch Checklist](./PRELAUNCH_CHECKLIST.md)** - Complete validation before mainnet
- **[PHASE7 Launch Ready](./PHASE7_LAUNCH_READY.md)** - Phase 7 completion status
- **[PHASE7 Quick Reference](./PHASE7_QUICKREF.md)** - Quick reference guide
- **[PHASE7 Complete](./PHASE7_COMPLETE.md)** - Detailed completion report

### Deployment & Operations
- **[Runbook](./RUNBOOK.md)** - Production operations runbook
- **[Observability](./OBSERVABILITY.md)** - Monitoring, metrics, and alerts
- **[DNS Seeds](../testnet/)** - Seed node configuration

## 🔍 Monitoring

### Metrics & Dashboards
- Prometheus metrics endpoint: `http://localhost:9090/metrics`
- Grafana dashboards available in `docs/assets/dashboards/`
- Key metrics tracked:
  - Block height and sync status
  - Peer connections
  - Mempool size and transaction rate
  - Mining hashrate (if enabled)
  - Resource utilization (CPU, RAM, disk)

### Alerting
- Alert rules for critical conditions
- PagerDuty/Slack integration
- Escalation policies

See [OBSERVABILITY.md](./OBSERVABILITY.md) for complete monitoring setup.

## 🚀 Deployment

### System Requirements
- **CPU**: 4+ cores (8+ recommended)
- **RAM**: 8GB minimum (16GB+ recommended)
- **Disk**: 100GB+ SSD
- **Network**: 100Mbps+ with low latency
- **OS**: Ubuntu 22.04 LTS, Debian 12, or Rocky Linux 9

### Installation

```bash
# Download release binary
wget https://github.com/your-org/BitQuan/releases/download/v1.0.0/bitquan-node-linux-amd64

# Verify checksum
sha256sum -c bitquan-node-linux-amd64.sha256

# Install
sudo install -m 0755 bitquan-node-linux-amd64 /usr/local/bin/bitquan-node

# Run preflight checks
bq-preflight --config /etc/bitquan/mainnet.toml

# Start node
systemctl start bitquan-node
```

See [RUNBOOK.md](./RUNBOOK.md) for detailed deployment procedures.

## 🛠️ Operations Tasks

### Common Tasks
- Start/stop node: `systemctl start|stop bitquan-node`
- Check status: `systemctl status bitquan-node`
- View logs: `journalctl -u bitquan-node -f`
- Monitor peers: `bitquan-node p2p-status`
- Check sync: `curl http://localhost:9090/metrics | grep block_height`

### Emergency Procedures
- **Network partition**: See RUNBOOK.md § Network Issues
- **Database corruption**: Run `bitquan-node verify-db --rebuild`
- **High mempool**: Adjust `--mempool-max-size` and restart
- **Out of disk space**: Prune old data, expand volume

## 📊 Performance Tuning

### RocksDB Optimization
```toml
[database]
max_open_files = 1000
write_buffer_size = 67108864  # 64MB
max_write_buffer_number = 3
target_file_size_base = 67108864
```

### Network Tuning
```bash
# Increase connection limits
ulimit -n 65535

# TCP keepalive
net.ipv4.tcp_keepalive_time = 120
net.ipv4.tcp_keepalive_intvl = 30
net.ipv4.tcp_keepalive_probes = 3
```

### Resource Limits
```ini
[Service]
LimitNOFILE=65535
LimitNPROC=4096
MemoryMax=16G
CPUQuota=400%
```

## 🔐 Security Hardening

- Run as non-root user
- Firewall rules (only P2P and RPC ports)
- TLS for RPC endpoints
- JWT authentication required
- Regular security updates
- Encrypted backups

See [../security/](../security/) for security policies and best practices.

## 📚 Related Documentation

- [CLI Tools](../cli/) - Command reference
- [Testnet Guide](../testnet/) - Testnet setup
- [Development](../dev/) - Build from source

---

*Updated on: 2025-01-07*
