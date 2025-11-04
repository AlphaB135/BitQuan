# BitQuan Operations Runbook

**Version**: 1.0  
**Last Updated**: 2024-11-04  
**Audience**: Node operators, emergency responders

---

## Emergency Response

### 🚨 Critical Incident Response

**Decision Tree**:

1. Consensus break detected → **HALT MINING** (see below)
2. Network split detected → **BROADCAST NOTICE** + investigate
3. BurstGuard flapping → **BUMP GUARD** threshold
4. Critical CVE found → **PATCH & DEPLOY** (testnet-only if needed)

---

## HALT: Stop Mining Immediately

### When to Halt

- Consensus rule violation detected
- Invalid block propagating network-wide
- Critical security vulnerability discovered
- Data corruption in majority of nodes

### Halt Procedure

```bash
# 1. Stop mining on all controlled nodes
pkill -SIGTERM bitquan-miner

# 2. Broadcast network notice (if RPC available)
curl -X POST http://localhost:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "setalert",
    "params": [{
      "level": "critical",
      "message": "Mining halted: [REASON]. Do not accept blocks after height [HEIGHT]."
    }],
    "id": 1
  }'

# 3. Pin current chain tip
CHAIN_TIP=$(curl -s http://localhost:18443 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
  | jq -r '.result')

echo "HALT at block $CHAIN_TIP" | tee HALT_HEIGHT.txt

# 4. Stop accepting incoming connections
iptables -A INPUT -p tcp --dport 18444 -j DROP  # P2P port
```

### Communication Template

```markdown
🚨 **TESTNET HALT NOTICE**

**Time**: [UTC timestamp]
**Block Height**: [CHAIN_TIP]
**Reason**: [Brief description]

**Action Required**:

- Stop mining immediately
- Do not accept blocks after height [CHAIN_TIP]
- Await patched release

**ETA for Fix**: [Estimated time]
**Status Updates**: [URL or channel]

— BitQuan Core Team
```

**Channels**:

- GitHub: Pin issue with `emergency` label
- Twitter/X: @bitquanchain (if available)
- Discord/Telegram: Announcement channel

---

## NOTICE: Broadcast Network Warning

### Non-Critical Issues

- Upcoming mandatory upgrade (7-day notice)
- High network load detected
- Suspected eclipse attack
- BurstGuard activating frequently

### Broadcast Procedure

```bash
# Send warning via RPC (custom alert method)
curl -X POST http://localhost:18443 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "setalert",
    "params": [{
      "level": "warning",
      "message": "[NOTICE_TEXT]",
      "expires": 1234567890
    }],
    "id": 1
  }'

# Log to all operators
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] NOTICE: [MESSAGE]" | tee -a /var/log/bitquan-notices.log

# Update status page (if available)
# [URL to status dashboard]
```

---

## BUMP GUARD: Adjust BurstGuard Threshold

### Symptom: BurstGuard Flapping

**Indicators**:

- `guard_activation_total` metric increasing >5 per 100 blocks
- Logs show: `BurstGuard activated: difficulty spike detected`
- Legitimate mining causing false positives

### Analysis

```bash
# Check recent guard activations
curl -s http://localhost:9090/api/v1/query \
  -d 'query=increase(guard_activation_total[1h])' \
  | jq '.data.result[0].value[1]'

# Review block intervals
./scripts/analyze_block_intervals.sh | tail -20

# Check if spike is legitimate (e.g., testnet hash-rate increase)
```

### Bump Procedure

**Option 1: Temporary Override (Testnet Only)**

```bash
# Edit config/testnet.toml
[consensus.burst_guard]
spike_threshold = 15.0  # Increase from 10.0 to 15.0
activation_cooldown = 50  # Increase cooldown blocks

# Restart nodes
systemctl restart bitquan-node
```

**Option 2: Hotfix Patch**

```bash
# Create patch branch
git checkout -b hotfix/burst-guard-threshold-v1.0.0-rc1

# Edit crates/consensus/src/burst_guard.rs
# Change: const SPIKE_THRESHOLD: f64 = 10.0;
# To:     const SPIKE_THRESHOLD: f64 = 15.0;

# Test
cargo test -p bitquan-consensus burst_guard

# Commit
git commit -am "hotfix: bump BurstGuard threshold to 15x (testnet empirical adjustment)"

# Tag and release
git tag -s v1.0.0-rc1-hotfix1 -m "BurstGuard threshold adjustment"
./scripts/release.sh
```

### Rollout

```bash
# 1. Deploy to bootstrap nodes first
ssh node1.bitquan.dev "cd /opt/bitquan && git pull && systemctl restart bitquan-node"

# 2. Monitor for 100 blocks
watch -n 60 'curl -s http://node1.bitquan.dev:9090/metrics | grep guard_activation'

# 3. If stable, deploy to remaining infrastructure
ansible-playbook playbooks/update_nodes.yml -e version=v1.0.0-rc1-hotfix1

# 4. Announce via NOTICE (see above)
```

---

## PATCH: Emergency Security Fix

### Critical CVE Discovered

**Triage**:

1. **Severity**: Use CVSS score and impact analysis
2. **Exploitability**: In-the-wild exploits? Testnet-only?
3. **Scope**: Consensus / Wallet / RPC / Network?

### Patch Workflow

```bash
# 1. Create security patch branch (private until disclosure)
git checkout -b security/CVE-2024-XXXXX

# 2. Implement fix with tests
# [Make changes]
cargo test --all

# 3. Document in SECURITY.md
echo "## CVE-2024-XXXXX: [Title]" >> docs/SECURITY.md
echo "**Fixed in**: v1.0.0-rc1-patch1" >> docs/SECURITY.md

# 4. Commit with DCO sign-off
git commit -S -m "security: fix [CVE-2024-XXXXX] - [brief description]

Severity: Critical
Impact: [description]
Mitigation: [fix summary]

Signed-off-by: [name] <[email]>"

# 5. Tag release
git tag -s v1.0.0-rc1-patch1 -m "Security patch for CVE-2024-XXXXX"

# 6. Build and sign release
./scripts/release.sh

# 7. Test on isolated testnet
./scripts/spin_up_local_testnet.sh
# [Run exploit PoC, verify fix]

# 8. Deploy to testnet (coordinated)
```

### Coordinated Disclosure

```markdown
**Timeline** (Responsible Disclosure):

Day 0: Vulnerability reported
Day 1: Acknowledged, investigation begins
Day 2-3: Patch developed and tested
Day 4: Private disclosure to major node operators
Day 5: Public release with advisory
Day 30: Full technical details published
```

**Advisory Template** (`docs/advisories/CVE-2024-XXXXX.md`):

```markdown
# Security Advisory: CVE-2024-XXXXX

**Severity**: Critical / High / Medium / Low
**Component**: [consensus/wallet/rpc/p2p]
**Affected Versions**: v1.0.0-rc1 and earlier
**Fixed in**: v1.0.0-rc1-patch1

## Summary

[Brief description of vulnerability]

## Impact

[What can an attacker do?]

## Mitigation

Upgrade to v1.0.0-rc1-patch1 immediately.

## Workaround

[If upgrade not possible, temporary mitigation]

## Credit

[Researcher name, responsible disclosure timeline]

## Timeline

- 2024-11-01: Vulnerability reported
- 2024-11-04: Patch released
- 2024-12-04: Full disclosure (30 days)
```

---

## Monitoring & Alerts

### Key Metrics

```yaml
# Prometheus metrics to watch

# Consensus health
block_interval_seconds_p50{network="testnet"}  # Target: 600 ± 120
reorg_count_total{network="testnet"}           # Target: < 3/day
guard_activation_total{network="testnet"}      # Target: < 2/200 blocks

# Network health
p2p_peer_count{network="testnet"}              # Target: > 5
p2p_banned_peers_total{network="testnet"}      # Watch for sudden spikes

# RPC health
rpc_requests_total{status="5xx"}               # Target: < 0.1% error rate
rpc_request_duration_seconds{quantile="0.99"}  # Target: < 5s
```

### Alert Rules (Prometheus)

```yaml
groups:
  - name: bitquan_testnet_critical
    interval: 30s
    rules:
      - alert: ChainReorgSpike
        expr: increase(reorg_count_total{network="testnet"}[1h]) > 3
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Testnet reorg rate exceeds threshold"
          description: "{{ $value }} reorgs in last hour"

      - alert: BurstGuardFlapping
        expr: increase(guard_activation_total{network="testnet"}[10m]) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "BurstGuard activating frequently"
          description: "{{ $value }} activations in 10 minutes"

      - alert: RPCErrorRateHigh
        expr: rate(rpc_requests_total{status="5xx"}[5m]) > 0.1
        for: 2m
        labels:
          severity: high
        annotations:
          summary: "RPC error rate above 10%"
          description: "{{ $value }} errors/sec"

      - alert: PeerCountLow
        expr: p2p_peer_count{network="testnet"} < 3
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Testnet peer count critically low"
          description: "Only {{ $value }} peers connected"
```

### Structured Logging

**Log events requiring JSON format**:

```json
{
  "timestamp": "2024-11-04T09:53:20.123Z",
  "level": "WARN",
  "component": "rpc",
  "event": "rate_limit_exceeded",
  "client_ip": "203.0.113.42",
  "endpoint": "/wallet/send",
  "retry_after": 60,
  "status_code": 429
}
```

**Log these events**:

- 401 Unauthorized: `client_ip`, `endpoint`, `auth_method`
- 413 Payload Too Large: `client_ip`, `payload_size`, `limit`
- 429 Too Many Requests: `client_ip`, `endpoint`, `retry_after`
- 408 Request Timeout: `client_ip`, `endpoint`, `duration_ms`

**Analysis**:

```bash
# Top IPs hitting rate limits
jq -r 'select(.status_code == 429) | .client_ip' /var/log/bitquan.log \
  | sort | uniq -c | sort -rn | head

# RPC error patterns
jq -r 'select(.level == "ERROR" and .component == "rpc") | .event' /var/log/bitquan.log \
  | sort | uniq -c
```

---

## Rollback Procedure

### When to Rollback

- New version causes consensus divergence
- Critical bug in patch
- Unexpected network behavior

### Rollback Steps

```bash
# 1. Identify last known good version
git tag -l 'v1.0.0-rc*' | tail -2

# 2. Checkout previous tag
git checkout v1.0.0-rc1  # Previous stable

# 3. Rebuild
cargo build --release --locked

# 4. Stop current node
systemctl stop bitquan-node

# 5. Backup current data
cp -r /var/lib/bitquan /var/lib/bitquan.backup.$(date +%s)

# 6. Start with rollback version
systemctl start bitquan-node

# 7. Monitor sync
tail -f /var/log/bitquan.log | grep -E '(sync|block|peer)'

# 8. Broadcast notice
# [Use NOTICE procedure above]
```

---

## Network Split Detection

### Symptoms

- Two valid chains with equal length
- Peer reports different best block hash
- Block propagation stalled

### Investigation

```bash
# 1. Query multiple nodes for best block
for node in node{1..5}.bitquan.dev; do
  echo -n "$node: "
  curl -s http://$node:18443 \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"getbestblockhash","params":[],"id":1}' \
    | jq -r '.result'
done

# 2. Compare chain tips
# If different hashes at same height → SPLIT DETECTED

# 3. Identify fork point
curl -s http://node1.bitquan.dev:18443 \
  -d '{"jsonrpc":"2.0","method":"getchainconflicts","params":[],"id":1}' \
  | jq '.result'
```

### Resolution

```bash
# 1. Determine canonical chain (most PoW)
# [Manual analysis of block headers and cumulative difficulty]

# 2. Broadcast correct chain
# [Use NOTICE to guide nodes to correct chain]

# 3. Ban nodes on wrong chain (temporary)
curl -X POST http://localhost:18443 \
  -d '{"jsonrpc":"2.0","method":"setban","params":["203.0.113.42","add",86400],"id":1}'

# 4. Monitor convergence
watch -n 10 './scripts/check_network_consensus.sh'
```

---

## Backup & Recovery

### Critical Data

- **Blockchain**: `/var/lib/bitquan/blocks/`
- **Chainstate**: `/var/lib/bitquan/chainstate/`
- **Wallet**: `/var/lib/bitquan/wallet/` (encrypted)
- **Config**: `/etc/bitquan/config.toml`

### Backup Script

```bash
#!/bin/bash
# scripts/backup_node.sh

BACKUP_DIR="/backup/bitquan/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Stop node gracefully
systemctl stop bitquan-node

# Backup data
rsync -av /var/lib/bitquan/ "$BACKUP_DIR/data/"
cp /etc/bitquan/config.toml "$BACKUP_DIR/"

# Restart node
systemctl start bitquan-node

# Compress backup
tar -czf "$BACKUP_DIR.tar.gz" "$BACKUP_DIR"
rm -rf "$BACKUP_DIR"

echo "Backup completed: $BACKUP_DIR.tar.gz"
```

### Recovery

```bash
# 1. Stop node
systemctl stop bitquan-node

# 2. Clear corrupted data
rm -rf /var/lib/bitquan/blocks/*
rm -rf /var/lib/bitquan/chainstate/*

# 3. Restore from backup
tar -xzf /backup/bitquan/YYYYMMDD-HHMMSS.tar.gz -C /
rsync -av /backup/bitquan/YYYYMMDD-HHMMSS/data/ /var/lib/bitquan/

# 4. Verify permissions
chown -R bitquan:bitquan /var/lib/bitquan

# 5. Restart and resync
systemctl start bitquan-node
journalctl -u bitquan-node -f
```

---

## Contact & Escalation

### Escalation Path

1. **L1 - Node Operator**: Check logs, restart node
2. **L2 - Team Lead**: Coordinate response, broadcast notices
3. **L3 - Core Developer**: Code fixes, emergency patches
4. **L4 - Security Team**: CVE disclosure, audit coordination

### Emergency Contacts

- **General**: [GitHub Issues](https://github.com/AlphaB135/BitQuan/issues)
- **Security**: [PGP-encrypted email]
- **Real-time**: [Discord/Telegram TBD]

---

## Post-Incident Review

After any incident, create post-mortem:

**Template** (`docs/incidents/YYYY-MM-DD-[title].md`):

```markdown
# Incident: [Title]

**Date**: YYYY-MM-DD
**Duration**: [Start] - [End] (HH:MM)
**Severity**: Critical / High / Medium / Low

## Summary

[What happened?]

## Impact

- Nodes affected: [count]
- Blocks lost: [count]
- Downtime: [duration]

## Timeline

- HH:MM - Incident detected
- HH:MM - [Actions taken]
- HH:MM - Resolution

## Root Cause

[Why did this happen?]

## Resolution

[How was it fixed?]

## Action Items

- [ ] Update monitoring: [link to issue]
- [ ] Improve documentation: [link to PR]
- [ ] Code fix: [link to commit]

## Lessons Learned

[What did we learn?]
```

---

**Runbook Version**: 1.0  
**Maintained by**: BitQuan Core Team  
**Last Tested**: 2024-11-04
