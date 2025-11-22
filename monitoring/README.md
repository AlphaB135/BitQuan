# BitQuan Monitoring Stack

This directory contains the complete monitoring and observability setup for BitQuan nodes and mining operations.

## Components

### 1. Prometheus
- **Purpose**: Metrics collection and storage
- **Port**: 9090
- **Configuration**: `prometheus.yml`
- **Metrics endpoints**:
  - `localhost:8080/metrics` - Main node metrics
  - `localhost:8081/metrics` - Mining pool metrics
  - `localhost:8082/metrics` - Stratum server metrics

### 2. Grafana
- **Purpose**: Visualization and dashboards
- **Port**: 3000
- **Default credentials**: admin/admin123
- **Dashboard**: Pre-configured BitQuan mining dashboard

### 3. AlertManager
- **Purpose**: Alert routing and notification
- **Port**: 9093
- **Configuration**: `alertmanager.yml`

### 4. Node Exporter
- **Purpose**: System metrics collection
- **Port**: 9100

## Quick Start

### Using Docker Compose (Recommended)

```bash
# Start all monitoring services
cd monitoring
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Access Points

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000
- **AlertManager**: http://localhost:9093
- **Node Exporter**: http://localhost:9100/metrics

## Metrics Available

### Mining Metrics
- `pow_hashrate_gauge` - Hashrate by algorithm (H/s)
- `pow_mined_blocks_total` - Total blocks mined by algorithm
- `pow_block_time_seconds` - Average block time by algorithm
- `pow_hash_attempts_total` - Total hash attempts by algorithm
- `pow_verify_failures_total` - PoW verification failures

### Pool Metrics
- `stratum_pool_balance_gauge` - Current pool balance (satoshis)
- `stratum_blocks_persisted_total` - Total blocks persisted to chain
- `stratum_total_rewards_distributed` - Total rewards distributed
- `stratum_payouts_total` - Total payouts completed

### Network Metrics
- `network_peers_connected` - Number of connected peers
- `network_blocks_broadcast_total` - Total blocks broadcast
- `network_blocks_received_total` - Total blocks received
- `network_sync_active_gauge` - Sync status (0=idle, 1=syncing)

### System Metrics
- `bitquan_mempool_size` - Current mempool size
- `bitquan_blocks_total` - Total blocks processed
- `bitquan_transactions_total` - Total transactions processed
- `http_requests_total` - Total HTTP requests
- `websocket_connections_active` - Active WebSocket connections
- `system_uptime_seconds` - System uptime
- `system_errors_total` - Total system errors

## Alerts

### Critical Alerts
- **Node Down**: Node unreachable for >1 minute
- **Database Connection Failed**: Cannot connect to database
- **Mining Pool Down**: Mining pool service down
- **Stratum Server Down**: Stratum server down

### Warning Alerts
- **High Error Rate**: Error rate >0.1/sec for 2 minutes
- **Low Peer Count**: <3 peers connected for 5 minutes
- **Low Hashrate**: Hashrate <1MH/s for 10 minutes
- **No Blocks Mined**: No blocks in last hour
- **High Block Time**: Block time >10 minutes
- **Pool Balance Low**: Balance <0.01 BTC
- **High Mempool Size**: >10,000 pending transactions
- **High WebSocket Connections**: >1000 active connections

### Info Alerts
- **Node Recently Restarted**: Uptime <5 minutes

## Configuration

### Prometheus Configuration
Edit `prometheus.yml` to:
- Add new scrape targets
- Adjust scrape intervals
- Configure retention periods

### Alert Rules
Edit `bitquan_alerts.yml` to:
- Modify alert thresholds
- Add new alert rules
- Adjust alert durations

### AlertManager Configuration
Edit `alertmanager.yml` to:
- Configure email settings
- Add webhook endpoints
- Set up routing rules

## Grafana Dashboard

The pre-configured dashboard includes:

### Mining Overview
- Hashrate by algorithm
- Blocks mined over time
- Block time trends
- Mining efficiency metrics

### Network Status
- Peer connections
- Network sync status
- Block propagation metrics

### System Health
- Resource usage
- Error rates
- Uptime and availability

### Pool Operations
- Pool balance
- Reward distribution
- Payout statistics

## Integration with BitQuan Node

The monitoring system integrates with the BitQuan node through the `MonitoringSystem` struct:

```rust
use bitquan_node::monitoring::MonitoringSystem;

// Create monitoring system
let monitoring = Arc::new(MonitoringSystem::new(
    mining_metrics,
    rpc_metrics
));

// Start health checks
monitoring.clone().run_health_checks().await;

// Add monitoring routes to HTTP server
let routes = monitoring.routes();
```

## Maintenance

### Data Retention
- Prometheus data retention: 200 hours (default)
- Grafana data retention: Configured per data source

### Backup Configuration
- Backup `prometheus.yml` and `bitquan_alerts.yml`
- Export Grafana dashboard configurations
- Document custom alert rules

### Scaling
- For high-load environments, consider:
  - Remote write to long-term storage
  - Multiple Prometheus instances
  - Load balancing for Grafana

## Troubleshooting

### Common Issues

1. **Prometheus not scraping targets**
   - Check target endpoints are accessible
   - Verify network connectivity
   - Review Prometheus logs

2. **Alerts not firing**
   - Check alert rule syntax
   - Verify AlertManager configuration
   - Review notification channels

3. **Grafana dashboard not showing data**
   - Check Prometheus data source configuration
   - Verify metric names match
   - Check time range settings

### Log Locations
- Prometheus logs: `docker-compose logs prometheus`
- Grafana logs: `docker-compose logs grafana`
- AlertManager logs: `docker-compose logs alertmanager`

## Security Considerations

- Change default Grafana password
- Restrict access to monitoring endpoints
- Use HTTPS in production
- Secure AlertManager webhooks
- Regularly update container images

## Performance Tuning

- Adjust scrape intervals based on requirements
- Optimize Prometheus storage configuration
- Tune Grafana query performance
- Monitor resource usage of monitoring stack
