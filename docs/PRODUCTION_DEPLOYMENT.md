# BitQuan Production Deployment Guide

**Version:** 1.0
**Last Updated:** 2026-03-17
**Status:** Production Ready

---

## Table of Contents

1. [Hardware Requirements](#1-hardware-requirements)
2. [Network Requirements](#2-network-requirements)
3. [Security Checklist](#3-security-checklist)
4. [Monitoring Setup](#4-monitoring-setup)
5. [Backup Strategy](#5-backup-strategy)
6. [Upgrade Path](#6-upgrade-path)

---

## 1. Hardware Requirements

### Minimum Requirements (Full Node)

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 4 cores, 2.4GHz | 8 cores, 3.0GHz+ |
| **RAM** | 8 GB | 16 GB |
| **Storage** | 100 GB SSD | 500 GB NVMe SSD |
| **Network** | 10 Mbps | 100 Mbps+ fiber |
| **OS** | Ubuntu 22.04 LTS | Ubuntu 24.04 LTS |

### Mining Node Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 8 cores, 3.0GHz | 16+ cores |
| **RAM** | 16 GB | 32 GB |
| **Storage** | 200 GB NVMe | 1 TB NVMe |
| **GPU** | Optional | NVIDIA RTX 3080+ |

### Pool Operator Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 8 cores | 16+ cores |
| **RAM** | 32 GB | 64 GB |
| **Storage** | 500 GB NVMe | 2 TB NVMe RAID |
| **Bandwidth** | 1 Gbps | 10 Gbps |
| **Redundancy** | Single server | Load-balanced cluster |

### Storage Growth Estimates

Based on Dilithium5 signature sizes (~4.6 KB per signature):

| Timeframe | Estimated Chain Size | With Pruning |
|-----------|---------------------|--------------|
| 1 year | ~15-20 GB | ~5-10 GB |
| 5 years | ~75-100 GB | ~5-15 GB |
| 10 years | ~150-200 GB | ~10-20 GB |

**Note:** Pruning (BQIP-0003) significantly reduces storage by keeping only UTXO set and block headers.

### Comparison with Other Blockchains

| Blockchain | Annual Storage Growth | 10-Year Projection |
|------------|----------------------|-------------------|
| Bitcoin | ~60 GB/year | ~600 GB |
| Ethereum | ~200 GB/year | ~2 TB |
| QRL | ~25 GB/year | ~250 GB |
| **BitQuan** | ~15-20 GB/year | ~150-200 GB |

---

## 2. Network Requirements

### Port Configuration

| Port | Protocol | Purpose | External Access |
|------|----------|---------|-----------------|
| **8333** | TCP | P2P Network | **Required** |
| **8332** | TCP | RPC API | Optional (internal) |
| **3333** | TCP | Stratum Mining | Optional (miners) |
| **9090** | TCP | Prometheus Metrics | Internal only |
| **8080** | TCP | Dashboard/Web UI | Optional |

### Firewall Configuration (UFW)

```bash
# Essential P2P port
sudo ufw allow 8333/tcp comment 'BitQuan P2P'

# RPC (restrict to localhost or trusted IPs)
sudo ufw allow from 10.0.0.0/8 to any port 8332 comment 'BitQuan RPC'

# Stratum (if mining pool)
sudo ufw allow 3333/tcp comment 'BitQuan Stratum'

# Dashboard (if public)
sudo ufw allow 8080/tcp comment 'BitQuan Dashboard'

# Enable firewall
sudo ufw enable
```

### iptables Alternative

```bash
# P2P
iptables -A INPUT -p tcp --dport 8333 -m state --state NEW -j ACCEPT

# Rate limit RPC
iptables -A INPUT -p tcp --dport 8332 -m state --state NEW -m recent --set
iptables -A INPUT -p tcp --dport 8332 -m state --state NEW -m recent --update --seconds 60 --hitcount 5 -j DROP

# Save rules
iptables-save > /etc/iptables/rules.v4
```

### Bandwidth Requirements

| Node Type | Daily Upload | Daily Download | Monthly |
|-----------|-------------|----------------|---------|
| Full Node | ~500 MB | ~1 GB | ~50 GB |
| Mining Node | ~1 GB | ~2 GB | ~100 GB |
| Pool Operator | ~10 GB | ~5 GB | ~500 GB |

### DNS Configuration

```bash
# Example DNS records
mainnet.bitquan.org      A     1.2.3.4
seed1.bitquan.org        A     1.2.3.4
seed2.bitquan.org        A     5.6.7.8
rpc.bitquan.org          A     1.2.3.4
pool.bitquan.org         A     1.2.3.4
```

### Connection Limits

```toml
# /etc/bitquan/mainnet.toml
[network.p2p]
max_peers = 125          # Maximum peer connections
max_inbound = 80         # Maximum inbound connections
max_outbound = 10        # Maximum outbound connections
connection_timeout = 30  # Seconds
```

---

## 3. Security Checklist

### Pre-Deployment Security Checklist

#### OS Hardening

- [ ] **Disable root SSH login**
  ```bash
  sed -i 's/PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config
  ```

- [ ] **SSH key-only authentication**
  ```bash
  sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
  systemctl restart sshd
  ```

- [ ] **Install fail2ban**
  ```bash
  apt install -y fail2ban
  systemctl enable fail2ban
  systemctl start fail2ban
  ```

- [ ] **Enable automatic security updates**
  ```bash
  apt install -y unattended-upgrades
  dpkg-reconfigure -plow unattended-upgrades
  ```

#### File Permissions

- [ ] **Secure configuration files**
  ```bash
  chmod 600 /etc/bitquan/mainnet.toml
  chown bitquan:bitquan /etc/bitquan/mainnet.toml
  ```

- [ ] **Secure data directory**
  ```bash
  chmod 700 /var/lib/bitquan
  chown -R bitquan:bitquan /var/lib/bitquan
  ```

- [ ] **Secure wallet files**
  ```bash
  chmod 600 /var/lib/bitquan/wallets/*.keystore
  ```

### TLS/SSL Setup

#### Using Let's Encrypt

```bash
# Install certbot
apt install -y certbot

# Obtain certificate
certbot certonly --standalone -d rpc.bitquan.org

# Auto-renewal
systemctl enable certbot.timer
systemctl start certbot.timer
```

#### Nginx Reverse Proxy with TLS

```nginx
# /etc/nginx/sites-available/bitquan
server {
    listen 443 ssl http2;
    server_name rpc.bitquan.org;

    ssl_certificate /etc/letsencrypt/live/rpc.bitquan.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/rpc.bitquan.org/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers off;

    location / {
        proxy_pass http://127.0.0.1:8332;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name rpc.bitquan.org;
    return 301 https://$server_name$request_uri;
}
```

### JWT Authentication Setup

```bash
# Generate JWT secret
openssl rand -hex 32 > /etc/bitquan/jwt_secret
chmod 600 /etc/bitquan/jwt_secret

# Add to configuration
cat >> /etc/bitquan/mainnet.toml << EOF
[rpc]
jwt_enabled = true
jwt_secret_file = "/etc/bitquan/jwt_secret"
jwt_expiry = 3600  # 1 hour
EOF
```

#### Creating JWT Users

```bash
# Add user
bitquan-cli jwt-user-add --username admin --role admin

# List users
bitquan-cli jwt-user-list

# Remove user
bitquan-cli jwt-user-remove --username admin
```

### RPC Security Configuration

```toml
# /etc/bitquan/mainnet.toml
[rpc]
enabled = true
bind = "127.0.0.1:8332"  # localhost only
username = "admin"
password = "use-strong-password-here"
require_auth = true
rate_limit = 100         # requests per minute
cors_origins = []        # or ["https://yourdomain.com"]
```

### Security Audit Commands

```bash
# Check open ports
ss -tulpn

# Check running services
systemctl list-units --type=service --state=running

# Check failed login attempts
lastb | head -20

# Check firewall status
ufw status verbose

# Check for security updates
apt list --upgradable 2>/dev/null | grep -i security
```

---

## 4. Monitoring Setup

### Prometheus Configuration

```yaml
# /etc/prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - localhost:9093

rule_files:
  - /etc/prometheus/bitquan_alerts.yml

scrape_configs:
  - job_name: 'bitquan'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: /metrics

  - job_name: 'node_exporter'
    static_configs:
      - targets: ['localhost:9100']
```

### BitQuan Alert Rules

```yaml
# /etc/prometheus/bitquan_alerts.yml
groups:
  - name: bitquan_node
    rules:
      - alert: NodeDown
        expr: up{job="bitquan"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "BitQuan node is down"
          description: "BitQuan node has been down for more than 1 minute."

      - alert: BlockSyncStalled
        expr: rate(bitquan_block_height[5m]) == 0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Block sync stalled"
          description: "No new blocks synced in 10 minutes."

      - alert: LowPeerCount
        expr: bitquan_peer_count < 3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count"
          description: "Less than 3 peers connected."

      - alert: HighMemoryUsage
        expr: (node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes) / node_memory_MemTotal_bytes > 0.9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage"
          description: "Memory usage above 90%."

      - alert: DiskSpaceLow
        expr: (node_filesystem_avail_bytes{mountpoint="/var/lib/bitquan"} / node_filesystem_size_bytes{mountpoint="/var/lib/bitquan"}) < 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Disk space critical"
          description: "Less than 10% disk space remaining."

      - alert: MiningHashrateDrop
        expr: rate(bitquan_mining_hashes_total[5m]) < 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Mining hashrate dropped"
          description: "Mining hashrate below expected threshold."
```

### Grafana Dashboard Setup

```bash
# Import BitQuan dashboard
# Dashboard ID: (TBD - will be published to Grafana.com)

# Manual import
curl -o bitquan-dashboard.json https://docs.bitquan.org/dashboards/mainnet.json
# In Grafana UI: Dashboards > Import > Upload JSON file
```

### Key Metrics to Monitor

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `bitquan_block_height` | Current block height | No change for 10m |
| `bitquan_peer_count` | Connected peers | < 3 peers |
| `bitquan_mempool_size` | Pending transactions | > 10,000 |
| `bitquan_mining_hashrate` | Current hashrate | Drop > 50% |
| `bitquan_rpc_requests` | RPC request rate | > 1000/min |
| `node_cpu_seconds_total` | CPU usage | > 80% |
| `node_memory_MemAvailable_bytes` | Available memory | < 10% |

### AlertManager Configuration

```yaml
# /etc/alertmanager/alertmanager.yml
global:
  resolve_timeout: 5m
  smtp_smarthost: 'smtp.example.com:587'
  smtp_from: 'alerts@bitquan.org'
  smtp_auth_username: 'alerts@bitquan.org'
  smtp_auth_password: 'your-password'

route:
  group_by: ['alertname']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'team-email'
  routes:
    - match:
        severity: critical
      receiver: 'team-pagerduty'

receivers:
  - name: 'team-email'
    email_configs:
      - to: 'ops@bitquan.org'

  - name: 'team-pagerduty'
    pagerduty_configs:
      - service_key: 'your-pagerduty-key'

  - name: 'team-slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
        channel: '#bitquan-alerts'
```

### Health Check Script

```bash
#!/bin/bash
# /opt/bitquan/scripts/health-check.sh

ENDPOINT="http://localhost:8332/health"
ALERT_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK"

response=$(curl -s -o /dev/null -w "%{http_code}" $ENDPOINT)

if [ "$response" != "200" ]; then
    curl -X POST "$ALERT_WEBHOOK" \
        -H "Content-Type: application/json" \
        -d "{\"text\": \"🚨 BitQuan health check failed! HTTP $response\"}"
    exit 1
fi

echo "Health check passed"
exit 0
```

---

## 5. Backup Strategy

### Backup Types

| Type | Frequency | Retention | Size Estimate |
|------|-----------|-----------|---------------|
| **Full** | Daily (2 AM) | 30 days | ~5-10 GB |
| **Incremental** | Every 6 hours | 7 days | ~100-500 MB |
| **Config-only** | On change | 90 days | ~1 MB |
| **Wallet** | On change | Forever | ~10 KB |

### Recovery Objectives

| Metric | Target | Description |
|--------|--------|-------------|
| **RPO** | 15 minutes | Maximum data loss |
| **RTO** | 1 hour | Time to restore service |
| **Availability** | 99.9% | Post-restore uptime |

### Automated Backup Setup

```bash
# Create backup directories
mkdir -p /opt/backups/bitquan/{daily,incremental,config,wallet}

# Create backup user
useradd -r -s /bin/false backup
chown -R backup:backup /opt/backups
```

### Backup Script

```bash
#!/bin/bash
# /opt/bitquan/scripts/backup.sh

set -e

BACKUP_DIR="/opt/backups/bitquan"
DATA_DIR="/var/lib/bitquan"
CONFIG_DIR="/etc/bitquan"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30

# Full backup
full_backup() {
    echo "Starting full backup..."
    tar -czf "$BACKUP_DIR/daily/bitquan_full_$TIMESTAMP.tar.gz" \
        -C "$(dirname $DATA_DIR)" "$(basename $DATA_DIR)" \
        -C "$(dirname $CONFIG_DIR)" "$(basename $CONFIG_DIR)"

    # Encrypt with GPG
    gpg --encrypt --recipient backup@bitquan.org \
        "$BACKUP_DIR/daily/bitquan_full_$TIMESTAMP.tar.gz"

    # Remove unencrypted backup
    rm "$BACKUP_DIR/daily/bitquan_full_$TIMESTAMP.tar.gz"

    echo "Full backup completed: bitquan_full_$TIMESTAMP.tar.gz.gpg"
}

# Incremental backup
incremental_backup() {
    echo "Starting incremental backup..."
    rsync -av --delete "$DATA_DIR/" "$BACKUP_DIR/incremental/current/"
    tar -czf "$BACKUP_DIR/incremental/bitquan_inc_$TIMESTAMP.tar.gz" \
        -C "$BACKUP_DIR/incremental" "current"
    echo "Incremental backup completed"
}

# Wallet backup
wallet_backup() {
    echo "Backing up wallets..."
    cp -r "$DATA_DIR/wallets/"* "$BACKUP_DIR/wallet/"
    echo "Wallet backup completed"
}

# Cleanup old backups
cleanup() {
    find "$BACKUP_DIR/daily" -name "*.gpg" -mtime +$RETENTION_DAYS -delete
    find "$BACKUP_DIR/incremental" -name "*.tar.gz" -mtime +7 -delete
}

# Main
case "${1:-full}" in
    full) full_backup ;;
    incremental) incremental_backup ;;
    wallet) wallet_backup ;;
    cleanup) cleanup ;;
    *) echo "Usage: $0 {full|incremental|wallet|cleanup}" ;;
esac
```

### Systemd Timer for Automated Backups

```ini
# /etc/systemd/system/bitquan-backup.service
[Unit]
Description=BitQuan Backup Service
After=bitquan.service

[Service]
Type=oneshot
User=backup
ExecStart=/opt/bitquan/scripts/backup.sh full
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/bitquan-backup.timer
[Unit]
Description=Run BitQuan Backup Daily

[Timer]
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
# Enable backup timer
systemctl daemon-reload
systemctl enable bitquan-backup.timer
systemctl start bitquan-backup.timer
```

### Recovery Procedure

```bash
#!/bin/bash
# /opt/bitquan/scripts/recover.sh

BACKUP_FILE=$1
DATA_DIR="/var/lib/bitquan"

if [ -z "$BACKUP_FILE" ]; then
    echo "Available backups:"
    ls -la /opt/backups/bitquan/daily/*.gpg | tail -10
    exit 1
fi

# Stop service
systemctl stop bitquan

# Decrypt backup
gpg --decrypt "$BACKUP_FILE" > /tmp/restore.tar.gz

# Extract
tar -xzf /tmp/restore.tar.gz -C /tmp/

# Restore data
rm -rf "$DATA_DIR/*"
cp -r /tmp/var/lib/bitquan/* "$DATA_DIR/"

# Fix permissions
chown -R bitquan:bitquan "$DATA_DIR"

# Start service
systemctl start bitquan

echo "Recovery completed"
```

---

## 6. Upgrade Path

### Version Compatibility

| From Version | To Version | Migration Required | Downtime |
|--------------|------------|-------------------|----------|
| v1.0.x | v1.1.x | No | ~5 min |
| v1.x | v2.0.0 | Yes (schema) | ~30 min |
| v0.x | v1.0.0 | Yes (full resync) | 6-12 hrs |

### Standard Upgrade Procedure

```bash
# 1. Backup current state
/opt/bitquan/scripts/backup.sh full

# 2. Stop service
sudo systemctl stop bitquan

# 3. Download new version
wget https://github.com/bitquan/bitquan/releases/download/v1.1.0/bitquan-v1.1.0-linux-x86_64.tar.gz

# 4. Verify signature
gpg --verify bitquan-v1.1.0-linux-x86_64.tar.gz.asc

# 5. Extract and install
tar -xzf bitquan-v1.1.0-linux-x86_64.tar.gz
sudo cp bitquan-node /usr/local/bin/

# 6. Check configuration compatibility
diff /etc/bitquan/mainnet.toml /etc/bitquan/mainnet.toml.new

# 7. Start service
sudo systemctl start bitquan

# 8. Verify
bitquan-cli getblockchaininfo
sudo journalctl -u bitquan -f
```

### Rolling Upgrade (Cluster)

For pool operators with multiple nodes:

```bash
# Node 1 (Primary)
sudo systemctl stop bitquan
# ... upgrade ...
sudo systemctl start bitquan
# Wait for sync

# Node 2 (Secondary)
sudo systemctl stop bitquan
# ... upgrade ...
sudo systemctl start bitquan

# Update load balancer to point to new primary
```

### Database Migration

If schema changes are required:

```bash
# Check migration status
bitquan-cli migrationstatus

# Run migration (if required)
bitquan-cli migrate --version v2.0.0

# Verify migration
bitquan-cli verifychain
```

### Rollback Procedure

```bash
# If upgrade fails, rollback to previous version

# 1. Stop service
sudo systemctl stop bitquan

# 2. Restore from backup
/opt/bitquan/scripts/recover.sh /opt/backups/bitquan/daily/bitquan_full_YYYYMMDD.tar.gz.gpg

# 3. Install previous version
sudo cp /opt/bitquan/versions/v1.0.0/bitquan-node /usr/local/bin/

# 4. Start service
sudo systemctl start bitquan

# 5. Verify
bitquan-cli getblockchaininfo
```

### Version-Specific Notes

#### v1.0.0 → v1.1.0

- New RPC endpoints: `getmempoolancestors`, `getmempooldescendants`
- Configuration change: `rpc.rate_limit` default changed from 100 to 200
- No database migration required

#### v1.x → v2.0.0 (Planned)

- **Breaking changes:**
  - New UTXO database schema
  - Signature format change (Dilithium5 → Dilithium5-AES)
- **Migration required:** Yes
- **Estimated downtime:** 30 minutes
- **Pre-migration checklist:**
  - [ ] Full backup
  - [ ] Test on staging environment
  - [ ] Notify users of downtime
  - [ ] Prepare rollback plan

---

## Post-Deployment Checklist

### Verification Steps

- [ ] Node is syncing blocks
  ```bash
  bitquan-cli getblockcount
  ```

- [ ] Peer connections established
  ```bash
  bitquan-cli getpeerinfo | jq 'length'
  ```

- [ ] RPC is responding
  ```bash
  curl -X POST http://localhost:8332 -d '{"method":"getblockchaininfo"}'
  ```

- [ ] Monitoring is collecting metrics
  ```bash
  curl http://localhost:9090/metrics | grep bitquan
  ```

- [ ] Alerts are configured
  ```bash
  amtool alert --alertmanager.url=http://localhost:9093
  ```

- [ ] Backup timer is running
  ```bash
  systemctl list-timers | grep bitquan-backup
  ```

- [ ] Firewall is active
  ```bash
  ufw status
  ```

- [ ] TLS certificates are valid
  ```bash
  certbot certificates
  ```

---

## Support Resources

| Resource | URL |
|----------|-----|
| Documentation | https://docs.bitquan.org |
| GitHub Issues | https://github.com/bitquan/bitquan/issues |
| Discord | https://discord.gg/bitquan |
| Security Email | security@bitquan.org |
| Emergency Hotline | (see internal docs) |

---

**Related Documents:**
- [Mainnet Deployment](./operations/mainnet-deployment.md)
- [VPS Deployment Guide](./operations/vps-deployment.md)
- [Disaster Recovery](./operations/DISASTER-RECOVERY.md)
- [Monitoring Guide](./MONITORING.md)
- [Security Standards](./SECURITY_STANDARDS.md)

---

*Last Updated: 2026-03-17*
*Author: BitQuan Core Team*
