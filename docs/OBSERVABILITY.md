# BitQuan Observability & Monitoring

**Version**: 1.0  
**Last Updated**: 2024-11-04  
**Audience**: DevOps, Node Operators, SRE

---

## Overview

This document defines the observability strategy for BitQuan nodes, including metrics, alerts, logging, and monitoring best practices. Proper observability is critical for maintaining network health, detecting attacks, and ensuring consensus stability.

---

## Metrics

### Prometheus Metrics Endpoint

**Default**: `http://localhost:9090/metrics`

Configure in `config/testnet.toml`:

```toml
[metrics]
enabled = true
listen_addr = "127.0.0.1:9090"
```

### Core Metrics

#### 1. Consensus Metrics

```prometheus
# Block interval (seconds between blocks)
block_interval_seconds{network="testnet"} histogram
# Labels: network
# Target: p50=600s, p99=1200s

# Chain reorganizations (total count)
reorg_count_total{network="testnet"} counter
# Labels: network, depth
# Target: <0.5% of blocks (7-day window)

# BurstGuard activations (difficulty spike protection)
guard_activation_total{network="testnet",reason="spike"} counter
# Labels: network, reason (spike, timestamp_manipulation)
# Target: <2 per 200 blocks

# Current difficulty
difficulty_current{network="testnet"} gauge
# Labels: network

# Block height
block_height{network="testnet"} gauge
# Labels: network

# Blocks validated
blocks_validated_total{network="testnet",status="valid"} counter
# Labels: network, status (valid, invalid)

# Validation duration
block_validation_duration_seconds{network="testnet"} histogram
# Labels: network
```

#### 2. Mempool Metrics

```prometheus
# Mempool size (transaction count)
mempool_size{network="testnet"} gauge
# Labels: network
# Target: <50,000 transactions

# Mempool memory usage (bytes)
mempool_memory_bytes{network="testnet"} gauge
# Labels: network
# Target: <500 MB

# Transactions accepted
mempool_tx_accepted_total{network="testnet",reason="new"} counter
# Labels: network, reason (new, replacement)

# Transactions rejected
mempool_tx_rejected_total{network="testnet",reason="fee_too_low"} counter
# Labels: network, reason (fee_too_low, double_spend, invalid_signature, etc.)

# Transaction evictions
mempool_tx_evicted_total{network="testnet",reason="full"} counter
# Labels: network, reason (full, replaced, expired)
```

#### 3. Network (P2P) Metrics

```prometheus
# Active peer connections
p2p_peer_count{network="testnet",direction="inbound"} gauge
# Labels: network, direction (inbound, outbound)
# Target: >5 total peers

# Banned peers
p2p_banned_peers_total{network="testnet"} counter
# Labels: network

# Messages received
p2p_messages_received_total{network="testnet",type="block"} counter
# Labels: network, type (block, transaction, ping, etc.)

# Messages sent
p2p_messages_sent_total{network="testnet",type="block"} counter
# Labels: network, type

# Message validation failures
p2p_validation_failures_total{network="testnet",reason="invalid_signature"} counter
# Labels: network, reason

# Bytes transferred
p2p_bytes_transferred_total{network="testnet",direction="inbound"} counter
# Labels: network, direction (inbound, outbound)
```

#### 4. RPC Metrics

```prometheus
# RPC requests
rpc_requests_total{method="getblockchaininfo",status="200"} counter
# Labels: method, status (200, 400, 401, 403, 429, 500)

# RPC request duration
rpc_request_duration_seconds{method="getblockchaininfo"} histogram
# Labels: method
# Target: p99 <5s

# Active RPC connections
rpc_connections_active{network="testnet"} gauge
# Labels: network

# Rate limit hits
rpc_rate_limit_exceeded_total{endpoint="/wallet/send"} counter
# Labels: endpoint
```

#### 5. Wallet Metrics

```prometheus
# Wallet balance
wallet_balance_satoshis{address_type="p2pkh"} gauge
# Labels: address_type

# Transactions signed
wallet_tx_signed_total{network="testnet",result="success"} counter
# Labels: network, result (success, failure)

# Key generation
wallet_keys_generated_total{algorithm="sphincs"} counter
# Labels: algorithm (sphincs, dilithium)
```

#### 6. Storage Metrics

```prometheus
# Database size
storage_db_size_bytes{database="blocks"} gauge
# Labels: database (blocks, chainstate, wallet)

# Database operations
storage_operations_total{operation="get",status="success"} counter
# Labels: operation (get, put, delete), status

# Database operation duration
storage_operation_duration_seconds{operation="get"} histogram
# Labels: operation
```

---

## Service Level Objectives (SLOs)

### Testnet SLOs

| Metric                 | Target            | Measurement Window |
| ---------------------- | ----------------- | ------------------ |
| Block interval p50     | 10 min ± 20%      | 7 days             |
| Block interval p99     | <20 min           | 7 days             |
| Reorg rate             | <0.5%             | 7 days             |
| BurstGuard activations | <2 per 200 blocks | 24 hours           |
| P2P peer count         | >5                | 1 hour             |
| RPC availability       | >99%              | 30 days            |
| RPC p99 latency        | <5 seconds        | 24 hours           |
| Mempool capacity       | <80% full         | 1 hour             |

### Mainnet SLOs (Future)

| Metric                 | Target             | Measurement Window |
| ---------------------- | ------------------ | ------------------ |
| Block interval p50     | 10 min ± 10%       | 30 days            |
| Block interval p99     | <15 min            | 30 days            |
| Reorg rate             | <0.1%              | 30 days            |
| BurstGuard activations | <1 per 1000 blocks | 7 days             |
| P2P peer count         | >20                | 1 hour             |
| RPC availability       | >99.9%             | 30 days            |
| RPC p99 latency        | <2 seconds         | 24 hours           |

---

## Alert Rules

### Prometheus Alertmanager Configuration

```yaml
# /etc/prometheus/alertmanager.yml

global:
  resolve_timeout: 5m

route:
  group_by: ["alertname", "severity"]
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: "bitquan-alerts"
  routes:
    - match:
        severity: critical
      receiver: "bitquan-critical"
      continue: true
    - match:
        severity: warning
      receiver: "bitquan-warnings"

receivers:
  - name: "bitquan-critical"
    email_configs:
      - to: "alerts@bitquan.dev"
        from: "prometheus@bitquan.dev"
        smarthost: "smtp.example.com:587"
        headers:
          Subject: "🚨 [CRITICAL] BitQuan Alert: {{ .GroupLabels.alertname }}"

    slack_configs:
      - api_url: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
        channel: "#bitquan-critical"
        title: "🚨 Critical Alert"
        text: "{{ range .Alerts }}{{ .Annotations.description }}{{ end }}"

  - name: "bitquan-warnings"
    email_configs:
      - to: "warnings@bitquan.dev"
        from: "prometheus@bitquan.dev"
        headers:
          Subject: "⚠️  [WARNING] BitQuan Alert: {{ .GroupLabels.alertname }}"

  - name: "bitquan-alerts"
    email_configs:
      - to: "alerts@bitquan.dev"
        from: "prometheus@bitquan.dev"
```

### Alert Rules

```yaml
# /etc/prometheus/rules/bitquan_alerts.yml

groups:
  - name: bitquan_consensus_critical
    interval: 30s
    rules:
      - alert: ChainReorgSpike
        expr: increase(reorg_count_total{network="testnet"}[1h]) > 3
        for: 5m
        labels:
          severity: critical
          component: consensus
        annotations:
          summary: "Chain reorganization rate exceeds threshold"
          description: "{{ $value }} reorgs detected in last hour on {{ $labels.network }}"
          runbook: "https://github.com/AlphaB135/BitQuan/blob/main/docs/RUNBOOK.md#network-split-detection"

      - alert: ConsensusFailure
        expr: increase(blocks_validated_total{status="invalid"}[10m]) > 5
        for: 2m
        labels:
          severity: critical
          component: consensus
        annotations:
          summary: "Multiple invalid blocks detected"
          description: "{{ $value }} invalid blocks in 10 minutes - possible consensus break"

      - alert: BlockProductionStalled
        expr: time() - block_height{network="testnet"} > 1800
        for: 5m
        labels:
          severity: critical
          component: consensus
        annotations:
          summary: "No new blocks for 30 minutes"
          description: "Block production may be stalled on {{ $labels.network }}"

  - name: bitquan_consensus_warnings
    interval: 60s
    rules:
      - alert: BurstGuardFlapping
        expr: increase(guard_activation_total{network="testnet"}[10m]) > 5
        for: 5m
        labels:
          severity: warning
          component: consensus
        annotations:
          summary: "BurstGuard activating frequently"
          description: "{{ $value }} BurstGuard activations in 10 minutes - may need threshold adjustment"
          runbook: "https://github.com/AlphaB135/BitQuan/blob/main/docs/RUNBOOK.md#bump-guard"

      - alert: BlockIntervalHigh
        expr: histogram_quantile(0.99, rate(block_interval_seconds_bucket[1h])) > 1200
        for: 15m
        labels:
          severity: warning
          component: consensus
        annotations:
          summary: "Block interval p99 exceeds 20 minutes"
          description: "Network may be experiencing low hash rate"

  - name: bitquan_network_critical
    interval: 30s
    rules:
      - alert: PeerCountCritical
        expr: p2p_peer_count{network="testnet"} < 3
        for: 10m
        labels:
          severity: critical
          component: network
        annotations:
          summary: "Peer count critically low"
          description: "Only {{ $value }} peers connected - risk of isolation"

      - alert: HighBanRate
        expr: rate(p2p_banned_peers_total[5m]) > 1
        for: 5m
        labels:
          severity: warning
          component: network
        annotations:
          summary: "High peer ban rate detected"
          description: "{{ $value }} peers/sec being banned - possible attack"

  - name: bitquan_rpc_critical
    interval: 30s
    rules:
      - alert: RPCErrorRateHigh
        expr: rate(rpc_requests_total{status=~"5.."}[5m]) / rate(rpc_requests_total[5m]) > 0.10
        for: 2m
        labels:
          severity: high
          component: rpc
        annotations:
          summary: "RPC error rate exceeds 10%"
          description: "{{ $value | humanizePercentage }} of RPC requests failing"

      - alert: RPCLatencyHigh
        expr: histogram_quantile(0.99, rate(rpc_request_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
          component: rpc
        annotations:
          summary: "RPC p99 latency exceeds 5 seconds"
          description: "{{ $value }}s latency - possible overload"

  - name: bitquan_mempool_warnings
    interval: 60s
    rules:
      - alert: MempoolFull
        expr: mempool_size{network="testnet"} / 50000 > 0.8
        for: 10m
        labels:
          severity: warning
          component: mempool
        annotations:
          summary: "Mempool >80% full"
          description: "{{ $value }} transactions in mempool - may need to increase limits"

      - alert: HighRejectionRate
        expr: rate(mempool_tx_rejected_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
          component: mempool
        annotations:
          summary: "High transaction rejection rate"
          description: "{{ $value }} tx/sec being rejected"

  - name: bitquan_storage_warnings
    interval: 300s
    rules:
      - alert: DatabaseSizeGrowing
        expr: rate(storage_db_size_bytes[1h]) > 1e9 # 1 GB/hour
        for: 1h
        labels:
          severity: warning
          component: storage
        annotations:
          summary: "Database growing rapidly"
          description: "{{ $labels.database }} growing at {{ $value | humanize1024 }}B/sec"

      - alert: SlowDatabaseOperations
        expr: histogram_quantile(0.99, rate(storage_operation_duration_seconds_bucket[5m])) > 1
        for: 10m
        labels:
          severity: warning
          component: storage
        annotations:
          summary: "Slow database operations detected"
          description: "p99 latency: {{ $value }}s"
```

---

## Structured Logging

### Log Format

**JSON structured logs** for production:

```json
{
  "timestamp": "2024-11-04T09:53:20.123Z",
  "level": "WARN",
  "component": "rpc",
  "event": "rate_limit_exceeded",
  "client_ip": "203.0.113.42",
  "endpoint": "/wallet/send",
  "retry_after": 60,
  "status_code": 429,
  "request_id": "req-abc123"
}
```

### Log Levels

| Level     | Use Case                    | Examples                       |
| --------- | --------------------------- | ------------------------------ |
| **ERROR** | System failures             | Consensus break, DB corruption |
| **WARN**  | Recoverable issues          | Rate limits, banned peers      |
| **INFO**  | Normal operations           | Block mined, tx validated      |
| **DEBUG** | Development/troubleshooting | Function entry/exit            |
| **TRACE** | Verbose debugging           | Variable values, loops         |

### Critical Events to Log

#### Authentication & Authorization (401, 403)

```json
{
  "timestamp": "2024-11-04T09:53:20.123Z",
  "level": "WARN",
  "component": "rpc",
  "event": "unauthorized_access",
  "client_ip": "203.0.113.42",
  "endpoint": "/admin/shutdown",
  "auth_method": "bearer_token",
  "status_code": 401,
  "reason": "invalid_token"
}
```

#### Rate Limiting (429)

```json
{
  "timestamp": "2024-11-04T09:53:20.123Z",
  "level": "WARN",
  "component": "rpc",
  "event": "rate_limit_exceeded",
  "client_ip": "203.0.113.42",
  "endpoint": "/wallet/send",
  "requests_per_minute": 120,
  "limit": 100,
  "retry_after": 60,
  "status_code": 429
}
```

#### Payload Too Large (413)

```json
{
  "timestamp": "2024-11-04T09:53:20.123Z",
  "level": "WARN",
  "component": "rpc",
  "event": "payload_too_large",
  "client_ip": "203.0.113.42",
  "endpoint": "/transaction/broadcast",
  "payload_size_bytes": 2097152,
  "limit_bytes": 1048576,
  "status_code": 413
}
```

#### Request Timeout (408)

```json
{
  "timestamp": "2024-11-04T09:53:20.123Z",
  "level": "WARN",
  "component": "rpc",
  "event": "request_timeout",
  "client_ip": "203.0.113.42",
  "endpoint": "/blockchain/sync",
  "duration_ms": 30000,
  "timeout_ms": 30000,
  "status_code": 408
}
```

### Log Aggregation

**Recommended Stack**: ELK (Elasticsearch, Logstash, Kibana) or Grafana Loki

**Example Logstash Config**:

```ruby
# /etc/logstash/conf.d/bitquan.conf

input {
  file {
    path => "/var/log/bitquan/*.log"
    codec => json
    type => "bitquan"
  }
}

filter {
  if [type] == "bitquan" {
    # Parse timestamp
    date {
      match => ["timestamp", "ISO8601"]
      target => "@timestamp"
    }

    # Add geo IP data
    if [client_ip] {
      geoip {
        source => "client_ip"
        target => "geoip"
      }
    }

    # Tag critical events
    if [level] == "ERROR" or [level] == "CRITICAL" {
      mutate {
        add_tag => ["critical"]
      }
    }
  }
}

output {
  elasticsearch {
    hosts => ["localhost:9200"]
    index => "bitquan-%{+YYYY.MM.dd}"
  }
}
```

---

## Dashboards

### Grafana Dashboard Templates

#### 1. Consensus Dashboard

**Panels**:

- Block height (time series)
- Block interval distribution (histogram)
- Reorg count (counter)
- BurstGuard activations (counter)
- Difficulty over time (time series)

**Example PromQL**:

```promql
# Block interval p50
histogram_quantile(0.50, rate(block_interval_seconds_bucket[5m]))

# Reorg rate (per 1000 blocks)
rate(reorg_count_total[1h]) * 1000 / rate(block_height[1h])

# BurstGuard activations per hour
increase(guard_activation_total[1h])
```

#### 2. Network Health Dashboard

**Panels**:

- Peer count (gauge)
- Banned peers (counter)
- Message throughput (time series)
- Bytes transferred (time series)

#### 3. RPC Performance Dashboard

**Panels**:

- Request rate (time series)
- Error rate by status code (stacked area)
- Latency percentiles (heatmap)
- Active connections (gauge)

#### 4. Mempool Dashboard

**Panels**:

- Mempool size (time series)
- Memory usage (time series)
- Transaction acceptance vs rejection (stacked bar)
- Fee distribution (histogram)

---

## Tracing

### Distributed Tracing (Optional)

For complex deployments, consider **OpenTelemetry** or **Jaeger**:

```rust
// Example: Instrument critical paths
use opentelemetry::trace::{Tracer, Span};

#[instrument]
pub async fn validate_block(block: &Block) -> Result<(), Error> {
    let span = tracer.start("validate_block");

    // Validate block header
    let _header_span = tracer.start("validate_header");
    validate_header(&block.header)?;

    // Validate transactions
    let _tx_span = tracer.start("validate_transactions");
    for tx in &block.transactions {
        validate_transaction(tx)?;
    }

    Ok(())
}
```

---

## Monitoring Best Practices

### 1. **Monitor Both Symptoms and Causes**

- Symptom: "Block interval is high"
- Cause: "Peer count dropped" or "Difficulty spiked"

### 2. **Use SLOs, Not Just Alerts**

- Define acceptable thresholds
- Measure over time windows
- Burn down error budgets

### 3. **Alert on User Impact**

- ❌ Bad: "CPU at 80%"
- ✅ Good: "RPC latency p99 >5s affecting users"

### 4. **Runbooks for Every Alert**

- Link to documentation
- Clear action steps
- Escalation path

### 5. **Regular Review**

- Weekly: Review alert fatigue
- Monthly: Update SLOs based on data
- Quarterly: Audit unused metrics

---

## Production Checklist

- [ ] Prometheus scraping metrics endpoint
- [ ] Alertmanager configured with notifications
- [ ] Grafana dashboards created
- [ ] Log aggregation (ELK/Loki) configured
- [ ] Structured JSON logging enabled
- [ ] Alert runbooks documented
- [ ] SLOs defined and tracked
- [ ] On-call rotation established
- [ ] Backup monitoring (secondary Prometheus)
- [ ] Status page for public visibility

---

## Resources

- **Prometheus**: https://prometheus.io/docs/
- **Grafana**: https://grafana.com/docs/
- **Alertmanager**: https://prometheus.io/docs/alerting/latest/alertmanager/
- **OpenTelemetry**: https://opentelemetry.io/
- **ELK Stack**: https://www.elastic.co/what-is/elk-stack

---

**Document Version**: 1.0  
**Maintainer**: BitQuan DevOps Team  
**Last Updated**: 2024-11-04
