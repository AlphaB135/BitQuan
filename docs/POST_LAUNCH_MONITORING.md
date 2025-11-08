# BitQuan Post-Launch Monitoring Guide

**Version:** v1.0.0  
**Purpose:** Production monitoring and operational procedures for BitQuan mainnet  
**Target Audience:** Node Operators, Infrastructure Teams, DevOps Engineers

---

## 🎯 Overview

This guide provides comprehensive monitoring procedures for BitQuan mainnet operations, including real-time health monitoring, performance metrics, and troubleshooting scenarios.

---

## 📊 Monitoring Infrastructure

### Grafana Dashboards

#### Main Dashboard: `bitquan-mainnet-overview`
- **URL:** https://grafana.bitquan.org/d/bitquan-mainnet-overview
- **Refresh Rate:** 30 seconds
- **Time Range:** Last 24 hours (default)

**Key Panels:**
1. **Network Health**
   - Active peers count
   - Block propagation time
   - Mempool size
   - Network difficulty

2. **Mining Operations**
   - Hashrate distribution
   - Block production rate
   - Mining pool statistics
   - Stratum connections

3. **Node Performance**
   - CPU usage
   - Memory consumption
   - Disk I/O
   - Network bandwidth

#### Security Dashboard: `bitquan-security`
- **URL:** https://grafana.bitquan.org/d/bitquan-security
- **Focus:** Security events and anomaly detection

**Key Panels:**
1. **Authentication Events**
   - Failed login attempts
   - JWT token usage
   - API access patterns

2. **Network Security**
   - DDoS attack indicators
   - Suspicious peer behavior
   - Rate limiting events

3. **System Integrity**
   - File system changes
   - Process monitoring
   - Memory access violations

---

## 🔍 Metrics Collection

### Prometheus Endpoints

#### Primary Metrics: `http://localhost:9090/metrics`
```bash
# Network health metrics
curl http://localhost:9090/metrics | grep network_
curl http://localhost:9090/metrics | grep peer_
curl http://localhost:9090/metrics | grep block_

# Mining metrics
curl http://localhost:9090/metrics | grep mining_
curl http://localhost:9090/metrics | grep stratum_
curl http://localhost:9090/metrics | grep hashrate_

# Performance metrics
curl http://localhost:9090/metrics | grep cpu_
curl http://localhost:9090/metrics | grep memory_
curl http://localhost:9090/metrics | grep disk_
```

#### Health Check: `http://localhost:8080/health`
```bash
curl http://localhost:8080/health
# Expected response:
# {"status":"healthy","timestamp":"2025-11-09T12:00:00Z","version":"v1.0.0"}
```

#### Node Status: `http://localhost:8332/getinfo`
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getinfo","params":[],"id":1}' \
  http://localhost:8332/
```

---

## 🚨 Alerting Rules

### Critical Alerts (PagerDuty/SMS)

#### 1. Node Down
```yaml
- alert: NodeDown
  expr: up{job="bitquan-node"} == 0
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "BitQuan node is down"
    description: "Node {{ $labels.instance }} has been down for more than 1 minute"
```

#### 2. Block Production Stalled
```yaml
- alert: BlockProductionStalled
  expr: time() - bitquan_block_last_timestamp > 1200  # 20 minutes
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Block production has stalled"
    description: "No new blocks for {{ $value }} seconds"
```

#### 3. Memory Exhaustion
```yaml
- alert: MemoryExhaustion
  expr: (node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes) / node_memory_MemTotal_bytes > 0.95
  for: 2m
  labels:
    severity: critical
  annotations:
    summary: "Memory usage critical"
    description: "Memory usage is {{ $value | humanizePercentage }}"
```

### Warning Alerts (Email/Slack)

#### 1. High Peer Latency
```yaml
- alert: HighPeerLatency
  expr: bitquan_peer_latency_seconds > 5.0
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High peer latency detected"
    description: "Average peer latency is {{ $value }} seconds"
```

#### 2. Mempool Congestion
```yaml
- alert: MempoolCongestion
  expr: bitquan_mempool_size_bytes > 100000000  # 100MB
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Mempool is congested"
    description: "Mempool size is {{ $value | humanizeBytes }}"
```

---

## 🔧 Common Troubleshooting Scenarios

### Scenario 1: Peer Sync Lag

#### Symptoms
- Block height lagging behind network
- Low peer count
- High sync latency

#### Diagnosis
```bash
# Check current height
curl -s http://localhost:8332/getblockcount | jq '.result'

# Check peer connections
curl -s http://localhost:8332/getpeerinfo | jq '.result | length'

# Check sync status
curl -s http://localhost:8332/getblockchaininfo | jq '.result'
```

#### Solutions
1. **Add Bootstrap Peers**
```bash
./bitquan-node --addnode bootstrap1.bitquan.org --addnode bootstrap2.bitquan.org
```

2. **Check Network Connectivity**
```bash
# Test DNS resolution
nslookup dns-seed.bitquan.org

# Test port connectivity
nc -zv peer.bitquan.org 8333
```

3. **Reset Sync State**
```bash
# Only if necessary - will re-download entire blockchain
./bitquan-node --reindex
```

### Scenario 2: RPC Saturation

#### Symptoms
- Slow RPC responses
- HTTP 503 errors
- High CPU usage

#### Diagnosis
```bash
# Check RPC connections
curl -s http://localhost:9090/metrics | grep rpc_connections

# Check response times
curl -w "@curl-format.txt" -s -o /dev/null http://localhost:8332/getblockcount

# Check CPU usage
top -p $(pgrep bitquan-node)
```

#### Solutions
1. **Increase Connection Limits**
```toml
[rpc]
max_connections = 200
request_timeout = 30
rate_limit = 20  # requests per second
```

2. **Enable Caching**
```toml
[rpc]
enable_cache = true
cache_size = "100MB"
cache_ttl = 300  # 5 minutes
```

3. **Load Balance RPC**
```bash
# Deploy multiple RPC nodes behind load balancer
nginx -c /etc/nginx/bitquan-rpc.conf
```

### Scenario 3: Mining Pool Issues

#### Symptoms
- Decreased hashrate
- Miner disconnections
- Invalid share submissions

#### Diagnosis
```bash
# Check stratum connections
curl -s http://localhost:9090/metrics | grep stratum_connections

# Check hashrate
curl -s http://localhost:9090/metrics | grep mining_hashrate

# Check share validation
curl -s http://localhost:9090/metrics | grep stratum_shares
```

#### Solutions
1. **Adjust Difficulty**
```bash
# Check current difficulty
curl -s http://localhost:8332/getmininginfo | jq '.result.difficulty'

# Adjust vardiff settings
./bitquan-node --vardiff-min 1 --vardiff-max 1000
```

2. **Monitor Pool Health**
```bash
# Check pool statistics
curl -s http://localhost:8080/api/pool/stats

# Check miner sessions
curl -s http://localhost:8080/api/pool/miners
```

---

## 📈 Performance Monitoring

### Key Performance Indicators (KPIs)

#### Network Performance
- **Block Propagation Time:** < 10 seconds (95th percentile)
- **Peer Connection Count:** 50-200 active peers
- **Mempool Processing:** < 1000 transactions/second
- **Sync Time:** < 30 minutes for initial sync

#### Mining Performance
- **Hashrate Utilization:** > 90% of pool capacity
- **Share Acceptance Rate:** > 95%
- **Block Finding Rate:** Consistent with difficulty
- **Miner Retention:** > 80% after 24 hours

#### System Performance
- **CPU Usage:** < 80% average
- **Memory Usage:** < 4GB steady state
- **Disk I/O:** < 100MB/s average
- **Network Bandwidth:** < 10Mbps sustained

### Performance Tuning

#### Database Optimization
```toml
[storage]
cache_size = "1GB"
max_open_files = 10000
compression = "lz4"
write_buffer_size = "128MB"
```

#### Network Optimization
```toml
[network]
max_peers = 200
outbound_connections = 8
download_window = 1024
upload_window = 1024
```

---

## 🔒 Security Monitoring

### Security Event Detection

#### Authentication Monitoring
```bash
# Monitor failed logins
curl -s http://localhost:9090/metrics | grep auth_failures_total

# Monitor JWT usage
curl -s http://localhost:9090/metrics | grep jwt_tokens_issued
```

#### Network Security
```bash
# Monitor DDoS indicators
curl -s http://localhost:9090/metrics | grep ddos_

# Monitor rate limiting
curl -s http://localhost:9090/metrics | grep rate_limit_
```

#### System Integrity
```bash
# Monitor file changes
inotifywait -m -r /var/lib/bitquan/

# Monitor process execution
auditctl -w /usr/local/bin/bitquan-node -p x
```

### Incident Response

#### Security Incident Checklist
1. **Isolation**
   - Disconnect affected node from network
   - Preserve forensic evidence
   - Document timeline

2. **Investigation**
   - Review logs for suspicious activity
   - Analyze network traffic
   - Check system integrity

3. **Recovery**
   - Patch vulnerabilities
   - Restore from clean backup
   - Monitor for recurrence

4. **Post-Mortem**
   - Document root cause
   - Update security procedures
   - Implement preventive measures

---

## 📋 Daily Operations Checklist

### Morning Checks (08:00 UTC)
- [ ] Verify all nodes are online and healthy
- [ ] Check block production is normal
- [ ] Review overnight alerts
- [ ] Monitor pool hashrate
- [ ] Check backup completion

### Midday Checks (12:00 UTC)
- [ ] Review performance metrics
- [ ] Check peer connectivity
- [ ] Monitor mempool size
- [ ] Verify RPC response times
- [ ] Check disk space usage

### Evening Checks (18:00 UTC)
- [ ] Review daily performance summary
- [ ] Check for security events
- [ ] Monitor network synchronization
- [ ] Verify backup integrity
- [ ] Document any issues

### Weekly Reviews
- [ ] Analyze performance trends
- [ ] Review alert effectiveness
- [ ] Update monitoring thresholds
- [ ] Check software updates
- [ ] Conduct security audit

---

## 🛠️ Maintenance Procedures

### Scheduled Maintenance

#### Weekly Maintenance
```bash
# Rotate logs
logrotate -f /etc/logrotate.d/bitquan

# Clean old metrics
find /var/lib/prometheus/ -name "*.wal" -mtime +7 -delete

# Update peer lists
./bitquan-node --update-seeds
```

#### Monthly Maintenance
```bash
# Database optimization
./bitquan-node --compact-database

# Security updates
apt update && apt upgrade -y

# Performance tuning
./bitquan-node --optimize-config
```

### Emergency Procedures

#### Node Recovery
```bash
# Stop node gracefully
systemctl stop bitquan

# Check filesystem integrity
fsck -f /var/lib/bitquan

# Restore from backup if needed
cp -r /backup/bitquan/latest/* /var/lib/bitquan/

# Start node
systemctl start bitquan
```

#### Network Partition Recovery
```bash
# Identify partitioned nodes
curl -s http://localhost:8332/getpeerinfo | jq '.result[] | select(.synced_headers < .blocks)'

# Force reconnection
./bitquan-node --reconnect-peers

# Verify network health
curl -s http://localhost:8332/getblockchaininfo
```

---

## 📞 Support and Escalation

### Contact Information

#### Technical Support
- **Email:** support@bitquan.org
- **Slack:** #bitquan-ops
- **PagerDuty:** +1-555-BITQUAN

#### Security Incidents
- **Email:** security@bitquan.org
- **PGP Key:** Available on website
- **Hotline:** +1-555-SECURE

#### Escalation Levels
1. **Level 1:** Basic monitoring and alerts
2. **Level 2:** Technical troubleshooting
3. **Level 3:** Security incidents
4. **Level 4:** Critical infrastructure

### Documentation Resources
- **Operations Manual:** [OPS_GUIDE.md](OPS_GUIDE.md)
- **API Reference:** [API_DOCS.md](API_DOCS.md)
- **Security Guide:** [SECURITY.md](SECURITY.md)
- **Troubleshooting:** [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

## 📊 Monitoring Tools Summary

| Tool | Purpose | Access |
|------|---------|--------|
| Grafana | Visual dashboards | https://grafana.bitquan.org |
| Prometheus | Metrics collection | http://localhost:9090 |
| AlertManager | Alert routing | https://alerts.bitquan.org |
| Kibana | Log analysis | https://logs.bitquan.org |
| Jaeger | Distributed tracing | https://trace.bitquan.org |

---

**Last Updated:** November 9, 2025  
**Version:** v1.0.0  
**Next Review:** December 9, 2025

---

*This monitoring guide is essential for maintaining BitQuan mainnet operations. Regular review and updates ensure optimal performance and security.*