# BitQuan Mining Metrics Reference

Complete documentation for Prometheus metrics exposed by BitQuan hybrid miner.

## Overview

BitQuan's hybrid miner tracks detailed per-algorithm metrics for monitoring, debugging, and performance analysis. All metrics follow Prometheus naming conventions and are exposed in text format.

## Metrics Endpoint

```bash
# Default endpoint (when metrics server enabled)
http://localhost:9090/metrics

# Query specific metric
curl -s http://localhost:9090/metrics | grep pow_mined_blocks_total
```

## Core Metrics

### pow_mined_blocks_total

**Type:** Counter  
**Description:** Total number of blocks successfully mined per algorithm  
**Labels:**
- `algo`: Algorithm name (`sha256d`, `randomx`)

**Example:**
```prometheus
pow_mined_blocks_total{algo="sha256d"} 42
pow_mined_blocks_total{algo="randomx"} 73
```

**Use Cases:**
- Track mining success rate per algorithm
- Compare algorithm effectiveness
- Validate weighted distribution

---

### pow_hash_attempts_total

**Type:** Counter  
**Description:** Total hash computations attempted per algorithm  
**Labels:**
- `algo`: Algorithm name

**Example:**
```prometheus
pow_hash_attempts_total{algo="sha256d"} 15000000
pow_hash_attempts_total{algo="randomx"} 8500000
```

**Use Cases:**
- Calculate actual hashrate
- Monitor mining efficiency
- Detect performance degradation

---

### pow_verify_failures_total

**Type:** Counter  
**Description:** PoW verification failures (should be rare in production)  
**Labels:**
- `algo`: Algorithm name

**Example:**
```prometheus
pow_verify_failures_total{algo="sha256d"} 0
pow_verify_failures_total{algo="randomx"} 2
```

**Troubleshooting:**
- High values indicate configuration issues
- Non-zero RandomX failures may signal cache corruption
- Check logs for specific error messages

---

### pow_hashrate_gauge

**Type:** Gauge  
**Description:** Estimated hashrate in hashes per second  
**Labels:**
- `algo`: Algorithm name

**Calculation:**
```
hashrate = hash_attempts / avg_block_time
```

**Example:**
```prometheus
pow_hashrate_gauge{algo="sha256d"} 12500000.50
pow_hashrate_gauge{algo="randomx"} 450000.25
```

**Notes:**
- Updates dynamically as blocks are mined
- Based on moving average (last 100 blocks)
- May be inaccurate during initial warm-up

---

### pow_block_time_seconds

**Type:** Gauge  
**Description:** Average time to mine a block (seconds)  
**Labels:**
- `algo`: Algorithm name

**Example:**
```prometheus
pow_block_time_seconds{algo="sha256d"} 8.35
pow_block_time_seconds{algo="randomx"} 12.47
```

**Use Cases:**
- Monitor difficulty adjustment effectiveness
- Detect algorithm performance changes
- Validate target block time (600s for mainnet)

---

## Derived Metrics

### Mining Efficiency

```prometheus
efficiency = pow_mined_blocks_total / (pow_hash_attempts_total / 1000000)
```

Higher efficiency = fewer attempts per block = better target matching.

### Algorithm Distribution

```prometheus
sha256d_ratio = pow_mined_blocks_total{algo="sha256d"} / total_blocks
randomx_ratio = pow_mined_blocks_total{algo="randomx"} / total_blocks
```

Should match configured weights (e.g., `sha256d:1,randomx:2` → 33%/67%).

### Success Rate

```prometheus
success_rate = pow_mined_blocks_total / (pow_hash_attempts_total / avg_attempts_per_block)
```

Indicates how often mining attempts succeed relative to difficulty.

## Example Prometheus Queries

### Total Blocks Mined

```promql
sum(pow_mined_blocks_total)
```

### Hashrate by Algorithm

```promql
rate(pow_hash_attempts_total[5m])
```

### Average Block Time (Last Hour)

```promql
avg_over_time(pow_block_time_seconds[1h])
```

### Algorithm Success Ratio

```promql
pow_mined_blocks_total{algo="randomx"} / 
(pow_mined_blocks_total{algo="sha256d"} + pow_mined_blocks_total{algo="randomx"})
```

## Grafana Dashboard Example

```json
{
  "panels": [
    {
      "title": "Blocks Mined by Algorithm",
      "targets": [
        {
          "expr": "pow_mined_blocks_total"
        }
      ],
      "type": "graph"
    },
    {
      "title": "Hashrate Comparison",
      "targets": [
        {
          "expr": "pow_hashrate_gauge"
        }
      ],
      "type": "graph"
    }
  ]
}
```

## Monitoring Best Practices

### Alert Rules

```yaml
groups:
  - name: bitquan_mining
    rules:
      - alert: HighVerifyFailures
        expr: rate(pow_verify_failures_total[5m]) > 0.1
        labels:
          severity: warning
        annotations:
          summary: "High PoW verification failure rate"
      
      - alert: LowHashrate
        expr: pow_hashrate_gauge < 1000000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Hashrate dropped below threshold"
      
      - alert: SlowBlockTime
        expr: pow_block_time_seconds > 1800
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Block time exceeds 30 minutes"
```

### Log Correlation

Metrics are complemented by structured logs:

```
INFO [HybridMiner] Block mined algo=randomx height=1234 diff=1.02x avg=8.3s
WARN PoW verification failed algo=sha256d hash=... reason=InvalidTarget
```

Match log timestamps with metric spikes for root cause analysis.

## Integration Examples

### cURL

```bash
curl -s http://localhost:9090/metrics | \
  grep pow_mined_blocks_total | \
  awk '{print $2}' | \
  paste -sd+ | bc
```

### Python

```python
import requests

response = requests.get('http://localhost:9090/metrics')
for line in response.text.split('\n'):
    if line.startswith('pow_mined_blocks_total'):
        print(line)
```

### Go

```go
package main

import (
    "fmt"
    "io"
    "net/http"
    "strings"
)

func main() {
    resp, _ := http.Get("http://localhost:9090/metrics")
    body, _ := io.ReadAll(resp.Body)
    
    for _, line := range strings.Split(string(body), "\n") {
        if strings.HasPrefix(line, "pow_hashrate_gauge") {
            fmt.Println(line)
        }
    }
}
```

## Performance Impact

Metrics collection has minimal overhead:
- Counter increments: ~5-10ns per operation
- Gauge updates: ~20ns per update
- Memory: ~1KB per algorithm

**Recommendation:** Enable metrics in all production environments.

## Future Metrics

Planned additions (Phase 3+):
- `pow_block_propagation_time` - Network propagation latency
- `pow_orphan_rate` - Orphaned blocks per algorithm
- `pow_chain_work_total` - Cumulative chain work
- `pow_difficulty_ratio` - Relative difficulty by algorithm

## Troubleshooting

### Metrics Not Updating

**Causes:**
1. Miner not running in hybrid mode
2. Metrics server not started
3. Firewall blocking port 9090

**Solutions:**
```bash
# Check miner mode
ps aux | grep bitquan-node

# Test metrics endpoint
curl -v http://localhost:9090/metrics

# Check port binding
lsof -i :9090
```

### Inconsistent Hashrate

**Causes:**
- Small sample size (few blocks)
- Difficulty changes mid-session
- System resource contention

**Solutions:**
- Wait for 20+ blocks for stable estimate
- Monitor system CPU/memory usage
- Use moving averages in dashboards

## References

- [Prometheus Metric Types](https://prometheus.io/docs/concepts/metric_types/)
- [Naming Best Practices](https://prometheus.io/docs/practices/naming/)
- [BitQuan Testnet Guide](./TESTNET_README.md)
- [Mining Architecture](./mining.md)

## Support

For metrics-related questions:
- **GitHub Issues**: Tag with `metrics` label
- **Discussions**: [Monitoring & Observability](https://github.com/AlphaB135/BitQuan/discussions)
