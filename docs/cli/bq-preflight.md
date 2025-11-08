# bq-preflight - Pre-Deployment Validation Tool

**Last Updated: 2025-01-07**

`bq-preflight` runs comprehensive pre-deployment checks to validate system configuration, dependencies, network connectivity, and readiness before launching BitQuan nodes in production.

## Usage

```bash
bq-preflight [OPTIONS]
```

## Quick Start

```bash
# Run all checks with default config
bq-preflight

# Specify custom config
bq-preflight --config config/mainnet.toml

# Run specific check categories
bq-preflight --checks system,network,security

# Generate detailed report
bq-preflight --report preflight-report.json

# CI/CD mode (exit 1 on any failure)
bq-preflight --strict
```

## Check Categories

### System Checks

Validates OS, kernel, dependencies, and resources:

```bash
bq-preflight --checks system --verbose
```

- ✅ OS version and kernel compatibility
- ✅ Required system libraries (OpenSSL, etc.)
- ✅ CPU features (AES-NI, AVX2)
- ✅ Available RAM (min 8GB recommended)
- ✅ Disk space (min 100GB for mainnet)
- ✅ File descriptor limits (ulimit)
- ✅ Timezone and NTP sync
- ✅ Rust toolchain version

### Network Checks

Tests connectivity and firewall rules:

```bash
bq-preflight --checks network --verbose
```

- ✅ Internet connectivity
- ✅ DNS resolution (seed nodes)
- ✅ P2P port accessibility (28333)
- ✅ RPC port firewall rules (28332)
- ✅ Outbound connection limits
- ✅ Bandwidth requirements
- ✅ IPv4/IPv6 support
- ✅ Peer discovery (connect to 3+ seeds)

### Security Checks

Validates security configuration:

```bash
bq-preflight --checks security --verbose
```

- ✅ JWT secret file permissions (0600)
- ✅ TLS certificate validity
- ✅ Key file permissions
- ✅ SELinux/AppArmor policies
- ✅ User privileges (non-root)
- ✅ Firewall rules (UFW/iptables)
- ✅ Entropy availability (/dev/urandom)
- ✅ Secure boot status

### Database Checks

Validates blockchain data and database:

```bash
bq-preflight --checks database --verbose
```

- ✅ RocksDB integrity
- ✅ Chainstate consistency
- ✅ Genesis block verification
- ✅ Index completeness
- ✅ UTXO set validation
- ✅ Backup availability
- ✅ Disk I/O performance

### Configuration Checks

Validates config files:

```bash
bq-preflight --checks config --verbose
```

- ✅ Config file syntax (TOML)
- ✅ Required fields present
- ✅ Network selection (mainnet/testnet)
- ✅ Peer list validity
- ✅ RPC binding addresses
- ✅ Log levels and paths
- ✅ Resource limits
- ✅ Mining configuration (if enabled)

## Configuration File

Example `preflight.toml`:

```toml
[system]
min_ram_gb = 8
min_disk_gb = 100
required_cpu_features = ["aes", "avx2"]

[network]
p2p_port = 28333
rpc_port = 28332
seed_nodes = [
    "seed1.bitquan.network:28333",
    "seed2.bitquan.network:28333",
]
min_peer_connections = 3
connectivity_timeout = "10s"

[security]
require_tls = true
jwt_secret_path = "data/jwt.secret"
tls_cert_path = "certs/server.crt"
tls_key_path = "certs/server.key"
allow_root = false

[database]
chaindata_path = "data/chaindata"
verify_genesis = true
check_indices = true

[thresholds]
disk_usage_warn = 80  # percent
memory_usage_warn = 90
open_files_warn = 10000
```

## Output Formats

### Console (default)

```
🔍 BitQuan Preflight Checks v1.0.0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ System Checks (5/5)
  ✅ OS Version: Ubuntu 22.04 LTS
  ✅ RAM: 16 GB (8 GB required)
  ✅ Disk Space: 250 GB (100 GB required)
  ✅ CPU Features: AES-NI, AVX2 ✓
  ✅ File Descriptors: 65535 (1024 required)

✅ Network Checks (4/4)
  ✅ Internet: Connected
  ✅ DNS: Resolved 3 seed nodes
  ✅ P2P Port 28333: Accessible
  ✅ RPC Port 28332: Firewalled (correct)

⚠️  Security Checks (3/4)
  ✅ JWT Secret: 0600 permissions ✓
  ✅ TLS Certificate: Valid until 2026-01-07
  ⚠️  Running as root (not recommended)
  ✅ Firewall: UFW active

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Summary: 12/13 passed, 1 warning

⚠️  Warnings:
  - Running as root user is not recommended for production
```

### JSON Report

```bash
bq-preflight --report report.json --format json
```

```json
{
  "timestamp": "2025-01-07T16:46:00Z",
  "version": "1.0.0",
  "passed": 12,
  "failed": 0,
  "warnings": 1,
  "categories": {
    "system": {"passed": 5, "failed": 0, "warnings": 0},
    "network": {"passed": 4, "failed": 0, "warnings": 0},
    "security": {"passed": 3, "failed": 0, "warnings": 1},
    "database": {"passed": 0, "failed": 0, "warnings": 0}
  },
  "checks": [...]
}
```

## Integration

### Systemd Service

```ini
[Unit]
Description=BitQuan Node
After=network.target
Requires=network.target

[Service]
Type=simple
ExecStartPre=/usr/local/bin/bq-preflight --config /etc/bitquan/mainnet.toml --strict
ExecStart=/usr/local/bin/bitquan-node run --config /etc/bitquan/mainnet.toml
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### Docker Health Check

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD bq-preflight --checks network,database --quick || exit 1
```

### Kubernetes Init Container

```yaml
initContainers:
- name: preflight
  image: bitquan:latest
  command: ["bq-preflight", "--strict", "--config", "/config/mainnet.toml"]
  volumeMounts:
  - name: config
    mountPath: /config
```

## Exit Codes

- `0` - All checks passed
- `1` - One or more checks failed (strict mode)
- `2` - Configuration error
- `3` - Permissions error
- `10+` - Specific check failures (see docs)

## See Also

- [Operations Runbook](../ops/RUNBOOK.md)
- [Pre-Launch Checklist](../ops/PRELAUNCH_CHECKLIST.md)
- [Deployment Guide](../ops/)

---

*Updated on: 2025-01-07*
