# BitQuan Mining Pool Dashboard

Real-time monitoring and visualization for BitQuan hybrid PoW mining pool operations.

## Overview

The BitQuan mining pool provides:
- **Real-time WebSocket streaming** of pool statistics and miner data
- **Prometheus metrics** for monitoring and alerting
- **Grafana dashboard integration** for visualization
- **Variable difficulty adjustment** with live tracking

## Architecture

```
┌─────────────┐      WebSocket      ┌──────────────┐
│   Miners    │────────────────────▶│   Stratum    │
│  (TCP 3333) │                     │    Server    │
└─────────────┘                     └──────┬───────┘
                                           │
                                           │ Metrics
                                           ▼
┌─────────────┐      HTTP/JSON      ┌──────────────┐
│  Dashboard  │◀────────────────────│  WebSocket   │
│   Clients   │     (Port 8081)     │   Dashboard  │
└─────────────┘                     └──────┬───────┘
                                           │
                                           │ Pull
                                           ▼
┌─────────────┐                     ┌──────────────┐
│  Grafana    │────────────────────▶│ Prometheus   │
│             │     (Port 9090)     │   Metrics    │
└─────────────┘                     └──────────────┘
```

## Stratum & Pool Metrics

### Connection Metrics

**`stratum_connections_total`** (counter)
- Total Stratum connections since server start
- Use to track connection attempts over time

**`stratum_active_miners`** (gauge)
- Current number of active miner connections
- Real-time view of pool participation

### Share Metrics

**`stratum_shares_total{status, algo}`** (counter)
- Total shares submitted by status and algorithm
- Labels:
  - `status`: `"ok"` (accepted) or `"reject"` (rejected)
  - `algo`: `"sha256d"` or `"randomx"`
- Key metric for pool performance

**`stratum_last_valid_share_timestamp`** (gauge)
- Unix timestamp of last accepted share
- Use to detect pool inactivity or stalls

### Difficulty Metrics

**`stratum_vardiff_adjustments_total`** (counter)
- Total number of difficulty adjustments performed
- Indicates vardiff activity level

### Mining Metrics

**`mining_blocks_found_total{algo}`** (counter)
- Total blocks found per algorithm
- Ultimate success metric

**`mining_hash_attempts_total{algo}`** (counter)
- Total hash attempts per algorithm
- Use to calculate effective hashrate

## WebSocket Endpoints

### `/ws/stats` - Pool Statistics Stream

Broadcasts aggregated pool statistics every 5 seconds.

**Message Format:**
```json
{
  "type": "stats",
  "data": {
    "timestamp": 1730500000,
    "active_miners": 14,
    "hashrate_sha256d": 1300000000.0,
    "hashrate_randomx": 81000000.0,
    "shares_ok": 2034,
    "shares_rejected": 57
  }
}
```

**Fields:**
- `timestamp`: Unix timestamp (seconds)
- `active_miners`: Current connected miners
- `hashrate_sha256d`: Estimated SHA-256d hashrate (H/s)
- `hashrate_randomx`: Estimated RandomX hashrate (H/s) [testnet only]
- `shares_ok`: Total accepted shares
- `shares_rejected`: Total rejected shares

### `/ws/miners` - Individual Miner Stream

Broadcasts list of active miners with their stats.

**Message Format:**
```json
{
  "type": "miners",
  "data": [
    {
      "address": "miner1@example.com",
      "algo": "sha256d",
      "difficulty": 1.5,
      "shares_ok": 450,
      "shares_rejected": 12,
      "uptime": 3600
    }
  ]
}
```

**Fields:**
- `address`: Miner username or identifier
- `algo`: Mining algorithm (`"sha256d"` or `"randomx"`)
- `difficulty`: Current difficulty setting
- `shares_ok`: Accepted shares for this session
- `shares_rejected`: Rejected shares for this session
- `uptime`: Connection duration (seconds)

## Grafana Dashboard Configuration

### Panel Definitions

#### 1. Active Miners Gauge

**Panel Type:** Gauge  
**Query:**
```promql
stratum_active_miners
```

**Settings:**
- Min: 0
- Max: 100 (adjust based on expected pool size)
- Thresholds:
  - Green: > 10
  - Yellow: 5-10
  - Red: < 5

#### 2. Hashrate per Algorithm

**Panel Type:** Time Series  
**Queries:**
```promql
# SHA-256d Hashrate (estimated)
rate(mining_hash_attempts_total{algo="sha256d"}[5m]) * 4294967296 / 15

# RandomX Hashrate (estimated)
rate(mining_hash_attempts_total{algo="randomx"}[5m]) * 4294967296 / 15
```

**Settings:**
- Y-axis: Logarithmic scale
- Unit: H/s (hashes per second)
- Legend: `{{algo}}`

#### 3. Share Acceptance Rate

**Panel Type:** Stat  
**Query:**
```promql
sum(rate(stratum_shares_total{status="ok"}[5m])) 
/ 
sum(rate(stratum_shares_total[5m])) * 100
```

**Settings:**
- Unit: Percent (0-100)
- Thresholds:
  - Green: > 95%
  - Yellow: 90-95%
  - Red: < 90%

#### 4. Shares Over Time

**Panel Type:** Time Series  
**Queries:**
```promql
# Accepted shares
rate(stratum_shares_total{status="ok"}[1m]) * 60

# Rejected shares
rate(stratum_shares_total{status="reject"}[1m]) * 60
```

**Settings:**
- Unit: shares/min
- Stack: Normal
- Fill opacity: 20%

#### 5. Difficulty Trend

**Panel Type:** Time Series  
**Query:**
```promql
# Average difficulty per miner (approximation)
rate(stratum_vardiff_adjustments_total[5m])
```

**Settings:**
- Shows vardiff adjustment activity
- Indicates pool adaptation to miner capacity

#### 6. Connection Stats

**Panel Type:** Stat (horizontal)  
**Queries:**
```promql
# Total connections
stratum_connections_total

# Active miners
stratum_active_miners

# Avg uptime (requires custom metric or calculation)
```

#### 7. Blocks Found

**Panel Type:** Stat + Time Series  
**Query:**
```promql
# Total blocks by algorithm
mining_blocks_found_total

# Blocks per hour
rate(mining_blocks_found_total[1h]) * 3600
```

**Settings:**
- Show sparkline for trend
- Color by algorithm

#### 8. Last Share Timestamp

**Panel Type:** Stat  
**Query:**
```promql
# Seconds since last share
time() - stratum_last_valid_share_timestamp
```

**Settings:**
- Unit: seconds
- Alert if > 120s (no shares in 2 minutes)

### Example PromQL Queries

**Share rejection rate per algorithm:**
```promql
sum by (algo) (rate(stratum_shares_total{status="reject"}[5m]))
/
sum by (algo) (rate(stratum_shares_total[5m]))
```

**Average difficulty per miner (requires custom implementation):**
```promql
sum(miner_difficulty) / stratum_active_miners
```

**Pool efficiency (accepted vs total hashes):**
```promql
sum(rate(stratum_shares_total{status="ok"}[5m]))
/
sum(rate(mining_hash_attempts_total[5m]))
```

**Vardiff adjustment frequency:**
```promql
rate(stratum_vardiff_adjustments_total[10m]) * 600
```

## Dashboard Layout

Recommended Grafana dashboard structure:

```
┌─────────────────────────────────────────────────────┐
│  Pool Overview                          [Refresh 5s] │
├─────────────────────────────────────────────────────┤
│  Active Miners │ Hashrate │ Acceptance │ Blocks     │
│     [GAUGE]    │  [STAT]  │   [STAT]   │  [STAT]    │
├─────────────────────────────────────────────────────┤
│  Hashrate per Algorithm                              │
│  [TIME SERIES CHART - Multi-line]                    │
├─────────────────────────────────────────────────────┤
│  Share Submission Rate          │  Difficulty Trend  │
│  [TIME SERIES - Stacked]        │  [TIME SERIES]     │
├─────────────────────────────────────────────────────┤
│  Active Miners Table                                 │
│  [TABLE - Top 20 by hashrate]                        │
└─────────────────────────────────────────────────────┘
```

## Command Line Usage

### Start Pool with Dashboard

```bash
# Full mining pool with dashboard
bitquan-node --enable-stratum \
  --stratum-bind=0.0.0.0:3333 \
  --enable-dashboard \
  --dashboard-port=8081 \
  --enable-vardiff \
  --vardiff-target=15.0 \
  --vardiff-rate=0.05

# Testnet with hybrid PoW
bitquan-node --network=testnet \
  --enable-stratum \
  --pow-mode=hybrid \
  --hybrid-weights="sha256d:1,randomx:2" \
  --enable-dashboard \
  --dashboard-port=8081
```

### CLI Flags

**Dashboard Flags:**
- `--enable-dashboard`: Enable WebSocket dashboard server
- `--dashboard-port=PORT`: Dashboard bind port (default: 8081)

**Vardiff Flags:**
- `--enable-vardiff`: Enable variable difficulty adjustment
- `--vardiff-target=SECONDS`: Target share submission interval (default: 15.0)
- `--vardiff-rate=RATE`: Adjustment rate 0.0-1.0 (default: 0.05)

**Stratum Flags:**
- `--enable-stratum`: Enable Stratum mining server
- `--stratum-bind=ADDR`: Stratum bind address (default: 0.0.0.0:3333)
- `--stratum-difficulty=DIFF`: Default difficulty (default: 1.0)

## Connecting to Dashboard

### WebSocket Client (JavaScript)

```javascript
const ws = new WebSocket('ws://localhost:8081/ws/stats');

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  
  if (msg.type === 'stats') {
    console.log('Active miners:', msg.data.active_miners);
    console.log('SHA-256d hashrate:', msg.data.hashrate_sha256d);
    console.log('Shares OK:', msg.data.shares_ok);
  }
};
```

### Simple HTTP Dashboard

```html
<!DOCTYPE html>
<html>
<head>
  <title>BitQuan Pool Stats</title>
  <style>
    body { font-family: monospace; padding: 20px; }
    .metric { margin: 10px 0; }
    .label { font-weight: bold; }
  </style>
</head>
<body>
  <h1>BitQuan Mining Pool</h1>
  <div id="stats"></div>
  
  <script>
    const ws = new WebSocket('ws://localhost:8081/ws/stats');
    
    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.type === 'stats') {
        const data = msg.data;
        document.getElementById('stats').innerHTML = `
          <div class="metric">
            <span class="label">Active Miners:</span> ${data.active_miners}
          </div>
          <div class="metric">
            <span class="label">Hashrate (SHA-256d):</span> ${(data.hashrate_sha256d / 1e9).toFixed(2)} GH/s
          </div>
          <div class="metric">
            <span class="label">Shares Accepted:</span> ${data.shares_ok}
          </div>
          <div class="metric">
            <span class="label">Shares Rejected:</span> ${data.shares_rejected}
          </div>
          <div class="metric">
            <span class="label">Acceptance Rate:</span> ${(100 * data.shares_ok / (data.shares_ok + data.shares_rejected)).toFixed(2)}%
          </div>
        `;
      }
    };
  </script>
</body>
</html>
```

## Prometheus Scrape Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'bitquan_pool'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 10s
    metrics_path: '/metrics'
```

## Alerting Rules

Example Prometheus alert rules:

```yaml
groups:
  - name: bitquan_pool
    interval: 30s
    rules:
      - alert: PoolInactive
        expr: time() - stratum_last_valid_share_timestamp > 300
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "No shares submitted in 5 minutes"
          
      - alert: HighRejectionRate
        expr: |
          sum(rate(stratum_shares_total{status="reject"}[5m]))
          /
          sum(rate(stratum_shares_total[5m])) > 0.1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Share rejection rate > 10%"
          
      - alert: NoActiveMiners
        expr: stratum_active_miners == 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "No active miners connected"
```

## Troubleshooting

### Dashboard not responding

1. Check if WebSocket server is running:
   ```bash
   curl http://localhost:8081/
   ```

2. Verify dashboard port in logs:
   ```
   Dashboard: WebSocket server listening on 0.0.0.0:8081
   ```

3. Check firewall rules allow port 8081

### Metrics not updating

1. Verify Stratum server is accepting connections:
   ```bash
   telnet localhost 3333
   ```

2. Check for active miners:
   ```bash
   curl http://localhost:9090/metrics | grep stratum_active_miners
   ```

3. Enable debug logging:
   ```bash
   RUST_LOG=bitquan_node=debug bitquan-node --enable-stratum
   ```

### Vardiff not adjusting

1. Check vardiff is enabled:
   ```bash
   # Should see in logs:
   # Stratum: Adjusting difficulty for miner1 from 1.0 to 1.5
   ```

2. Verify sufficient shares submitted (needs 8+ shares)

3. Check target time is reasonable (10-30s recommended)

## Security Considerations

### Production Deployment

1. **Use TLS/WSS for dashboard** in production
2. **Implement authentication** for WebSocket connections
3. **Rate limit** dashboard connections
4. **Restrict Prometheus metrics** to internal network
5. **Monitor for DDoS** on Stratum port

### Recommended Setup

```bash
# Behind reverse proxy with authentication
nginx:
  location /ws/ {
    proxy_pass http://localhost:8081;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    
    # Basic auth
    auth_basic "Pool Dashboard";
    auth_basic_user_file /etc/nginx/.htpasswd;
  }
```

## References

- [Stratum Protocol Documentation](../guides/STRATUM.md)
- [Metrics Reference](./METRICS.md)
- [Testnet Guide](../testnet/README.md)
- [Grafana Documentation](https://grafana.com/docs/)
- [Prometheus Querying](https://prometheus.io/docs/prometheus/latest/querying/basics/)

## Support

For questions or issues:
- GitHub Issues: https://github.com/BitQuan/BitQuan/issues
- Discord: [BitQuan Community]
- Documentation: https://github.com/BitQuan/BitQuan/tree/main/docs
