# Public Metrics Dashboard

**Last Updated**: 2026-03-27

## Overview

BitQuan exposes Prometheus-format metrics for public monitoring. This enables anyone to verify network health, mining activity, and transaction volume in real time.

## Available Metrics

### Core Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `block_height` | gauge | Current blockchain height |
| `connected_peers` | gauge | Number of connected P2P peers |
| `mempool_size` | gauge | Transactions waiting to be mined |
| `total_reorgs` | counter | Chain reorganizations since start |
| `ban_score_events` | counter_vec | Peer bans by reason |

### Traction Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `bitquan_total_transactions` | counter | Total transactions processed |
| `bitquan_blocks_per_hour` | gauge | Blocks mined in the last hour |
| `bitquan_avg_block_time_seconds` | gauge | Average block time (last 100 blocks) |
| `bitquan_active_miners` | gauge | Unique miners in the last 24 hours |
| `bitquan_network_hashrate_hps` | gauge | Estimated network hashrate (H/s) |
| `bitquan_total_blocks_mined` | counter | Total blocks since genesis |
| `bitquan_uptime_seconds` | gauge | Node uptime in seconds |

## Setup

### 1. Start Node with Metrics

The node exposes metrics automatically. No additional configuration needed.

```bash
./target/release/bitquan-node run --config config/testnet.toml
```

### 2. Start Monitoring Stack

```bash
cd monitoring
docker-compose up -d
```

This starts:
- **Prometheus** on `http://localhost:9090` — scrapes node metrics
- **Grafana** on `http://localhost:3000` — visualizes metrics (admin / admin123)

### 3. Configure Prometheus Scrape

Prometheus config (`monitoring/prometheus.yml`) should include:

```yaml
scrape_configs:
  - job_name: 'bitquan-node'
    static_configs:
      - targets: ['host.docker.internal:8080']
    scrape_interval: 15s
```

### 4. View Dashboard

Open `http://localhost:3000` and import the BitQuan dashboard from `monitoring/grafana-dashboard.json`.

## Querying Metrics (PromQL)

```promql
# Current block height
block_height

# Transactions per hour
rate(bitquan_total_transactions[1h]) * 3600

# Average block time
bitquan_avg_block_time_seconds

# Network hashrate
bitquan_network_hashrate_hps

# Active miners
bitquan_active_miners

# Peer count over time
connected_peers

# Reorg rate
rate(total_reorgs[24h])
```

## Public Embedding

To embed metrics on a website:

```html
<iframe src="http://grafana.example.com/d-solo/<dashboard-id>?orgId=1&theme=light"
        width="800" height="400" frameborder="0"></iframe>
```

## API Access

Metrics are available in Prometheus text format at the node's metrics endpoint:

```bash
curl http://localhost:8080/metrics
```

Example output:
```
# HELP bitquan_total_transactions Total number of transactions processed
# TYPE bitquan_total_transactions counter
bitquan_total_transactions 42

# HELP bitquan_blocks_per_hour Blocks mined in the last hour
# TYPE bitquan_blocks_per_hour gauge
bitquan_blocks_per_hour 6

# HELP bitquan_network_hashrate_hps Estimated network hashrate in hashes per second
# TYPE bitquan_network_hashrate_hps gauge
bitquan_network_hashrate_hps 14000000
```
