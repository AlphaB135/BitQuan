# 📊 BitQuan Monitoring Guide

Complete monitoring setup for BitQuan testnet/mainnet nodes.

## 🎯 Overview

BitQuan monitoring stack includes:
- **Prometheus** - Metrics collection
- **Grafana** - Visualization dashboards
- **AlertManager** - Alert notifications
- **Node Exporter** - System metrics

---

## 🚀 Quick Start

### Option 1: Docker Compose (Recommended)

```bash
cd monitoring
docker-compose up -d
```

Access:
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **AlertManager**: http://localhost:9093

### Option 2: Manual Setup

#### Install Prometheus
```bash
# Ubuntu/Debian
sudo apt install -y prometheus

# Configure
sudo cp prometheus.yml /etc/prometheus/prometheus.yml
sudo cp bitquan_alerts.yml /etc/prometheus/bitquan_alerts.yml
sudo systemctl restart prometheus
```

#### Install Grafana
```bash
# Add repository
sudo apt install -y software-properties-common
sudo add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
wget -q -O - https://packages.grafana.com/gpg.key | sudo apt-key add -

# Install
sudo apt update
sudo apt install -y grafana

# Start
sudo systemctl enable grafana-server
sudo systemctl start grafana-server
```

#### Install AlertManager
```bash
# Download
wget https://github.com/prometheus/alertmanager/releases/download/v0.26.0/alertmanager-0.26.0.linux-amd64.tar.gz
tar xvf alertmanager-0.26.0.linux-amd64.tar.gz
cd alertmanager-0.26.0.linux-amd64

# Configure
cp ../alertmanager.yml .

# Run
./alertmanager --config.file=alertmanager.yml &
```

---

## 📊 Dashboards

### Main Dashboard Panels

#### 1. Node Health
- **Uptime** - Node running time
- **Peer Count** - Connected peers
- **Block Height** - Current blockchain height
- **Sync Status** - Synchronization state

#### 2. Mining Performance
- **Hashrate** - Current hashrate (SHA256d, RandomX)
- **Blocks Mined** - Total blocks found
- **Mining Difficulty** - Current difficulty
- **Block Time** - Average time between blocks

#### 3. Pool Statistics
- **Active Miners** - Number of connected miners
- **Pool Hashrate** - Combined pool hashrate
- **Shares Submitted** - Valid/invalid shares
- **Pending Payouts** - Unpaid rewards

#### 4. Network Statistics
- **Transaction Rate** - Tx/second
- **Mempool Size** - Pending transactions
- **Block Propagation** - Time to propagate blocks
- **Bandwidth Usage** - Network I/O

#### 5. System Resources
- **CPU Usage** - Node CPU consumption
- **Memory Usage** - RAM usage
- **Disk Usage** - Storage usage
- **Network Traffic** - Bytes sent/received

---

## 🔔 Alerts

### Critical Alerts
- **Node Down** - Node stopped responding
- **Database Error** - Cannot connect to DB
- **Pool Down** - Mining pool offline
- **High Error Rate** - Errors > 10%

### Warning Alerts
- **Low Peer Count** - < 3 peers connected
- **No Blocks Mined** - No blocks in 2 hours
- **High Block Time** - Block time > 10 minutes
- **Low Pool Balance** - Pool balance < 0.01 BQ
- **High Mempool** - > 10,000 pending tx

### Info Alerts
- **Node Restarted** - Uptime < 5 minutes
- **New Version Available** - Update notification

---

## 📈 Metrics Reference

### Node Metrics
```
# System
system_uptime_seconds          # Node uptime
system_errors_total            # Total errors

# Network
network_peers_connected        # Connected peers
network_bytes_sent             # Bytes sent
network_bytes_received         # Bytes received

# Blockchain
blockchain_height              # Current block height
blockchain_sync_progress       # Sync percentage
blockchain_reorg_count         # Chain reorganizations

# Mempool
mempool_size                   # Pending transactions
mempool_bytes                  # Mempool size in bytes
mempool_fee_histogram          # Fee distribution
```

### Mining Metrics
```
# Proof of Work
pow_hashrate_gauge             # Current hashrate
pow_mined_blocks_total         # Total blocks mined
pow_block_time_seconds         # Time per block
pow_difficulty_gauge           # Current difficulty

# Mining Pool
stratum_connected_miners       # Active miners
stratum_shares_valid_total     # Valid shares
stratum_shares_invalid_total   # Invalid shares
stratum_pool_balance_gauge     # Pool balance
```

### Performance Metrics
```
# RPC
rpc_requests_total             # Total RPC requests
rpc_request_duration_seconds   # Request latency
rpc_errors_total               # RPC errors

# Database
db_queries_total               # Total queries
db_query_duration_seconds      # Query latency
db_connections_active          # Active connections

# WebSocket
websocket_connections_active   # Active WebSocket connections
websocket_messages_sent        # Messages sent
websocket_messages_received    # Messages received
```

---

## 🎨 Grafana Setup

### 1. Access Grafana
```
URL: http://localhost:3000
Default Login: admin/admin
```

### 2. Add Prometheus Data Source
1. Go to **Configuration > Data Sources**
2. Click **Add data source**
3. Select **Prometheus**
4. URL: `http://prometheus:9090` (Docker) or `http://localhost:9090` (Manual)
5. Click **Save & Test**

### 3. Import Dashboard
1. Go to **Dashboards > Import**
2. Upload `grafana-dashboard.json`
3. Select Prometheus data source
4. Click **Import**

### 4. Create Custom Panels
Example PromQL queries:

**Block Height**
```promql
blockchain_height
```

**Hashrate (5m average)**
```promql
rate(pow_hashrate_gauge[5m])
```

**Transaction Rate**
```promql
rate(mempool_tx_added_total[1m])
```

**Peer Count**
```promql
network_peers_connected
```

---

## 🚨 AlertManager Configuration

### Slack Notifications
```yaml
# alertmanager.yml
receivers:
  - name: 'slack'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#bitquan-alerts'
        title: 'BitQuan Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```

### Email Notifications
```yaml
receivers:
  - name: 'email'
    email_configs:
      - to: 'admin@bitquan.io'
        from: 'alerts@bitquan.io'
        smarthost: 'smtp.gmail.com:587'
        auth_username: 'alerts@bitquan.io'
        auth_password: 'YOUR_PASSWORD'
```

### Discord Webhook
```yaml
receivers:
  - name: 'discord'
    webhook_configs:
      - url: 'YOUR_DISCORD_WEBHOOK_URL'
        send_resolved: true
```

---

## 📱 Mobile Monitoring

### Grafana Mobile App
1. Download Grafana app (iOS/Android)
2. Add server: `http://your-server:3000`
3. Login with credentials
4. View dashboards on mobile

### Push Notifications
Configure in AlertManager to send push notifications via:
- **Pushover**
- **Telegram Bot**
- **PagerDuty**

---

## 🔍 Troubleshooting

### Metrics Not Showing
```bash
# Check if node is exposing metrics
curl http://localhost:9090/metrics

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Restart Prometheus
sudo systemctl restart prometheus
```

### Grafana Can't Connect
```bash
# Check Grafana logs
sudo journalctl -u grafana-server -f

# Test Prometheus connectivity
curl http://localhost:9090/-/healthy

# Restart Grafana
sudo systemctl restart grafana-server
```

### Alerts Not Firing
```bash
# Check AlertManager status
curl http://localhost:9093/-/healthy

# View active alerts
curl http://localhost:9093/api/v1/alerts

# Check alert rules
promtool check rules bitquan_alerts.yml
```

---

## 📊 Example Queries

### Top 10 Miners by Hashrate
```promql
topk(10, stratum_miner_hashrate_gauge)
```

### Block Time Trend (24h)
```promql
avg_over_time(pow_block_time_seconds[24h])
```

### Mempool Growth Rate
```promql
deriv(mempool_size[5m])
```

### Network Bandwidth
```promql
rate(network_bytes_sent[1m]) + rate(network_bytes_received[1m])
```

### Error Rate Percentage
```promql
100 * rate(system_errors_total[5m]) / rate(rpc_requests_total[5m])
```

---

## 🎯 Best Practices

### 1. Retention Policy
```yaml
# prometheus.yml
storage:
  tsdb:
    retention.time: 30d    # Keep 30 days
    retention.size: 50GB   # Max 50GB
```

### 2. Scrape Intervals
- **Node metrics**: 15s
- **System metrics**: 30s
- **Pool metrics**: 15s
- **Custom metrics**: 60s

### 3. Alert Thresholds
- Adjust based on your network size
- Start conservative, tune over time
- Group similar alerts
- Use silence for maintenance

### 4. Dashboard Organization
- Separate testnet/mainnet dashboards
- Create role-specific views (admin, miner, user)
- Use variables for dynamic filtering
- Add annotations for events

---

## 📚 Resources

### Prometheus
- [Prometheus Docs](https://prometheus.io/docs)
- [PromQL Guide](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Best Practices](https://prometheus.io/docs/practices/)

### Grafana
- [Grafana Docs](https://grafana.com/docs)
- [Dashboard Examples](https://grafana.com/grafana/dashboards)
- [Panel Plugins](https://grafana.com/plugins)

### AlertManager
- [AlertManager Docs](https://prometheus.io/docs/alerting/latest/alertmanager/)
- [Routing Tree](https://prometheus.io/docs/alerting/latest/configuration/)
- [Notification Templates](https://prometheus.io/docs/alerting/latest/notifications/)

---

## 🆘 Support

Issues with monitoring?
- **GitHub**: https://github.com/AlphaB135/BitQuan/issues
- **Discord**: https://discord.gg/bitquan (in `#monitoring` channel)
- **Email**: monitoring@bitquan.io

---

**Happy Monitoring! 📊🚀**
