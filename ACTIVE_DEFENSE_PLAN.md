# BitQuan Active Defense Plan — Real-Time Protection

**สถานการณ์**: AI red team กำลังโจมตีเหรียน BitQuan แบบ real-time  
**เป้าหมาย**: ป้องกัน, ตรวจจับ, และ respond ต่อการโจมตีทันที  
**สร้างโดย**: Hermes (ซากุระ) 🌸

---

## 🔍 การวิเคราะห์จุดแข็งที่มีอยู่

จากการตรวจสอบโค้ดจริงของนาย ฉันพบว่า BitQuan **มีการป้องกันที่ดีอยู่แล้ว**:

### ✅ Security Features ที่ใช้งานอยู่

1. **RPC Security** (`crates/rpc/src/server.rs`):
   - ✅ JWT Authentication
   - ✅ Rate Limiting (Token Bucket per IP)
   - ✅ Method-specific Rate Limiting
   - ✅ Authentication Backoff (exponential delay)
   - ✅ TLS/SSL Support
   - ✅ Security Event Logging with severity levels
   - ✅ Slowloris Attack Detection
   - ✅ Connection timeout protection

2. **Input Validation** (`crates/rpc/src/validation.rs`):
   - ✅ JSON-RPC 2.0 format validation
   - ✅ Method whitelist (allowed_methods)
   - ✅ XSS injection blocking (regex patterns)
   - ✅ SQL injection blocking
   - ✅ Command injection blocking
   - ✅ Path traversal blocking
   - ✅ Max parameters limit (100 default, 50 strict)
   - ✅ Max string length (1 MB default)
   - ✅ Max array length (10,000 items)
   - ✅ Max nesting depth (10 levels)
   - ✅ Null byte filtering
   - ✅ Control character filtering
   - ✅ Strict vs Permissive modes

3. **Security Event System**:
   ```rust
   enum SecurityEventType {
       RateLimitExceeded,
       AuthenticationFailed,
       InputValidationFailed,
       SuspiciousRequest,
       SlowlorisAttackDetected,
       RepeatedAuthFailures,
       InjectionAttempt,
   }
   ```

---

## 🚨 จุดอ่อนที่ต้องเสริม (Priority Order)

### 🔴 CRITICAL — แก้ทันที

#### 1. **Double-Spend Detection in Mempool**
**ปัญหา**: ต้องตรวจสอบว่า mempool มี atomic UTXO locking หรือไม่

**วิธีทดสอบ**:
```bash
# Terminal 1: Send transaction using UTXO_A
./bitquan-cli sendtoaddress addr1 1.0 --utxo=UTXO_A

# Terminal 2: Send transaction using same UTXO_A (พร้อมกัน)
./bitquan-cli sendtoaddress addr2 1.0 --utxo=UTXO_A
```

**วิธีแก้**:
```rust
// crates/mempool/src/lib.rs
use std::collections::HashSet;
use std::sync::Mutex;

struct Mempool {
    used_outpoints: Arc<Mutex<HashSet<OutPoint>>>,
}

impl Mempool {
    fn add_transaction(&mut self, tx: Transaction) -> Result<()> {
        let mut locked = self.used_outpoints.lock().unwrap();
        
        // Check double-spend atomically
        for input in &tx.inputs {
            if locked.contains(&input.previous_output) {
                return Err(Error::DoubleSpend);
            }
        }
        
        // Lock all inputs
        for input in &tx.inputs {
            locked.insert(input.previous_output.clone());
        }
        
        self.transactions.insert(tx.txid(), tx);
        Ok(())
    }
}
```

#### 2. **P2P Connection Limits (Eclipse Attack Prevention)**
**ปัญหา**: ต้องตรวจสอบว่ามี peer diversity checks หรือไม่

**วิธีทดสอบ**:
```bash
# Launch 100 malicious nodes from same subnet
for i in {1..100}; do
  ./bitquan-node run --datadir /tmp/sybil-$i \
    --p2p-bind 192.168.1.$i:19444 &
done

# Check if target node accepts all connections
./bitquan-cli getpeerinfo | jq 'length'
```

**วิธีแก้**:
```rust
// crates/network/src/peer_manager.rs
struct PeerManager {
    max_inbound: usize,      // Max 125 inbound
    max_outbound: usize,     // Max 8 outbound
    max_per_subnet: usize,   // Max 10 per /24 subnet
}

impl PeerManager {
    fn should_accept_connection(&self, ip: IpAddr) -> bool {
        // Check total connections
        if self.inbound_count() >= self.max_inbound {
            return false;
        }
        
        // Check subnet diversity (CIDR-based)
        let subnet = ip_to_subnet(ip, 24); // /24 subnet
        let subnet_count = self.peers_in_subnet(&subnet);
        if subnet_count >= self.max_per_subnet {
            return false;
        }
        
        true
    }
}
```

#### 3. **RPC Rate Limiting Bypass via IP Rotation**
**ปัญหา**: Rate limiting ตาม IP อาจ bypass ได้ด้วย proxy rotation

**วิธีทดสอบ**:
```bash
# Use multiple proxy IPs
for ip in $(cat proxy_list.txt); do
  curl --interface $ip -X POST http://140.245.127.249:19443/ \
    -H "Authorization: Bearer $JWT" \
    -d '{"method":"getblockcount","jsonrpc":"2.0","id":1}' &
done
```

**วิธีแก้**:
```rust
// crates/rpc/src/server.rs
struct RateLimiter {
    ip_limits: HashMap<IpAddr, TokenBucket>,
    user_limits: HashMap<String, TokenBucket>,  // User ID from JWT
    global_limit: TokenBucket,                  // Global cap
}

impl RateLimiter {
    fn check_rate_limit(&self, ip: IpAddr, user_id: &str) -> Result<()> {
        // Check IP-based limit
        if !self.check_ip_limit(ip)? {
            return Err(Error::RateLimitExceeded);
        }
        
        // Check user-based limit (from JWT)
        if !self.check_user_limit(user_id)? {
            return Err(Error::RateLimitExceeded);
        }
        
        // Check global limit (system-wide protection)
        if !self.check_global_limit()? {
            return Err(Error::SystemOverload);
        }
        
        Ok(())
    }
}
```

---

### 🟡 HIGH — แก้ใน 24 ชั่วโมง

#### 4. **Memory Exhaustion via Large RPC Requests**
**ปัญหา**: Request body size limit

**วิธีทดสอบ**:
```bash
# Send 10 MB request
dd if=/dev/urandom bs=1M count=10 | base64 | \
  jq -R '{jsonrpc:"2.0",method:"getblock",params:[.],id:1}' | \
  curl -X POST http://140.245.127.249:19443/ -d @-
```

**วิธีแก้**:
```rust
// crates/rpc/src/server.rs
const MAX_REQUEST_SIZE: usize = 1_048_576; // 1 MB

async fn read_request(stream: &mut impl AsyncRead) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut limited = stream.take(MAX_REQUEST_SIZE as u64);
    
    limited.read_to_end(&mut buffer).await?;
    
    if buffer.len() >= MAX_REQUEST_SIZE {
        return Err(Error::RequestTooLarge);
    }
    
    Ok(buffer)
}
```

#### 5. **Consensus Time Warp Attack**
**ปัญหา**: ต้องตรวจสอบ timestamp validation ใน ASERT algorithm

**วิธีทดสอบ**:
```bash
# Modify consensus code to set future timestamp
cd crates/consensus
grep -n "timestamp" src/*.rs

# ลอง mine block ด้วย timestamp +2 hours
```

**วิธีแก้**:
```rust
// crates/consensus/src/validator.rs
fn validate_timestamp(block: &Block, prev_blocks: &[Block]) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    // Rule 1: Block timestamp must not be > 2 hours in future
    if block.header.timestamp > now + 7200 {
        return Err(Error::TimestampTooFarInFuture);
    }
    
    // Rule 2: Block timestamp must be > median of last 11 blocks
    let median = calculate_median_time_past(prev_blocks)?;
    if block.header.timestamp <= median {
        return Err(Error::TimestampTooOld);
    }
    
    Ok(())
}
```

---

## 🛡️ Real-Time Monitoring & Auto-Response

### Setup 1: Security Event Dashboard

**สร้างไฟล์**: `/home/ubuntu/bitquan-audit/scripts/security-monitor.sh`

```bash
#!/bin/bash
# Real-time security monitoring for BitQuan

LOG_FILE="/var/log/bitquan/security.log"
ALERT_THRESHOLD_CRITICAL=5
ALERT_THRESHOLD_HIGH=20

# Colors
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🛡️  BitQuan Security Monitor — Started at $(date)"
echo "Watching: $LOG_FILE"
echo "Critical threshold: $ALERT_THRESHOLD_CRITICAL events/min"
echo ""

# Monitor security events
tail -F "$LOG_FILE" | while read line; do
    # Detect critical events
    if echo "$line" | grep -q "Critical"; then
        echo -e "${RED}🚨 CRITICAL: $line${NC}"
        
        # Extract IP address
        IP=$(echo "$line" | grep -oP '\d+\.\d+\.\d+\.\d+' | head -1)
        
        if [ ! -z "$IP" ]; then
            # Auto-ban IP
            echo "Banning IP: $IP"
            iptables -A INPUT -s "$IP" -j DROP
            
            # Log to ban list
            echo "$(date) - Auto-banned $IP" >> /var/log/bitquan/banned_ips.log
        fi
    fi
    
    # Detect rate limit exceeded
    if echo "$line" | grep -q "RateLimitExceeded"; then
        echo -e "${YELLOW}⚠️  Rate limit exceeded${NC}"
    fi
    
    # Detect authentication failures
    if echo "$line" | grep -q "AuthenticationFailed"; then
        echo -e "${YELLOW}⚠️  Auth failed: $line${NC}"
    fi
done
```

### Setup 2: Auto-Response Rules

**สร้างไฟล์**: `/home/ubuntu/bitquan-audit/scripts/auto-defense.sh`

```bash
#!/bin/bash
# Automated defense responses

FIREWALL_RULES="/etc/bitquan/firewall.rules"

# Function: Ban IP
ban_ip() {
    local IP=$1
    local REASON=$2
    
    echo "$(date) - Banning $IP: $REASON"
    iptables -A INPUT -s "$IP" -j DROP
    
    # Persist ban
    echo "$IP # $REASON" >> "$FIREWALL_RULES"
}

# Function: Rate limit IP
rate_limit_ip() {
    local IP=$1
    local RATE=$2  # packets per second
    
    echo "$(date) - Rate limiting $IP to $RATE pps"
    iptables -A INPUT -s "$IP" -m limit --limit "$RATE/s" -j ACCEPT
    iptables -A INPUT -s "$IP" -j DROP
}

# Function: Monitor mempool for spam
check_mempool_spam() {
    MEMPOOL_SIZE=$(./bitquan-cli getrawmempool | jq 'length')
    MAX_MEMPOOL_SIZE=50000
    
    if [ "$MEMPOOL_SIZE" -gt "$MAX_MEMPOOL_SIZE" ]; then
        echo "🚨 Mempool spam detected: $MEMPOOL_SIZE transactions"
        
        # Clear low-fee transactions
        ./bitquan-cli clearmempool --min-fee=0.0001
    fi
}

# Function: Check for Eclipse attack (peer diversity)
check_peer_diversity() {
    PEERS=$(./bitquan-cli getpeerinfo)
    
    # Count unique /24 subnets
    UNIQUE_SUBNETS=$(echo "$PEERS" | jq -r '.[].addr' | \
        cut -d'.' -f1-3 | sort -u | wc -l)
    
    if [ "$UNIQUE_SUBNETS" -lt 5 ]; then
        echo "🚨 Low peer diversity: Only $UNIQUE_SUBNETS subnets"
        echo "Possible Eclipse attack!"
        
        # Disconnect peers from majority subnet
        MAJORITY_SUBNET=$(echo "$PEERS" | jq -r '.[].addr' | \
            cut -d'.' -f1-3 | sort | uniq -c | sort -rn | head -1 | awk '{print $2}')
        
        echo "$PEERS" | jq -r ".[] | select(.addr | startswith(\"$MAJORITY_SUBNET\")) | .addr" | \
        while read peer; do
            echo "Disconnecting $peer"
            ./bitquan-cli disconnectnode "$peer"
        done
    fi
}

# Main monitoring loop
while true; do
    check_mempool_spam
    check_peer_diversity
    sleep 30
done
```

### Setup 3: Grafana Alert Rules

**สร้างไฟล์**: `/home/ubuntu/bitquan-audit/alerts/bitquan-alerts.yml`

```yaml
groups:
  - name: bitquan_security
    interval: 10s
    rules:
      # High RPC error rate
      - alert: HighRPCErrorRate
        expr: rate(rpc_errors_total[1m]) > 10
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "High RPC error rate detected"
          description: "{{ $value }} errors per second"

      # Repeated authentication failures
      - alert: RepeatedAuthFailures
        expr: rate(rpc_auth_failures_total[5m]) > 5
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Repeated authentication failures from {{ $labels.ip }}"

      # Mempool spam attack
      - alert: MempoolSpam
        expr: mempool_size > 50000
        for: 5m
        labels:
          severity: high
        annotations:
          summary: "Mempool size abnormally high: {{ $value }}"

      # Network partition / Eclipse attack
      - alert: LowPeerCount
        expr: peer_count < 5
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Very low peer count: {{ $value }}"

      # Block production stopped (51% attack?)
        expr: time() - last_block_timestamp > 600  # 10 minutes
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "No new blocks for 10+ minutes"

      # High orphan rate (selfish mining?)
      - alert: HighOrphanRate
        expr: rate(orphan_blocks_total[1h]) / rate(blocks_total[1h]) > 0.05
        for: 10m
        labels:
          severity: high
        annotations:
          summary: "Orphan rate > 5%: Possible selfish mining"

      # Disk space low
      - alert: LowDiskSpace
        expr: disk_free_bytes < 10737418240  # 10 GB
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low disk space: {{ $value | humanize }}B remaining"

      # CPU exhaustion
      - alert: HighCPUUsage
        expr: cpu_usage_percent > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "CPU usage > 90%: {{ $value }}%"

      # Memory exhaustion
      - alert: HighMemoryUsage
        expr: memory_usage_percent > 90
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Memory usage > 90%: {{ $value }}%"
```

---

## 🔬 Immediate Action Items

### Phase 1: Deploy Monitoring (ทำเลย — 15 นาที)

```bash
# 1. Create log directory
sudo mkdir -p /var/log/bitquan
sudo chown ubuntu:ubuntu /var/log/bitquan

# 2. Enable security logging in node
# Edit crates/rpc/src/server.rs to write SecurityEvent to file
# (อาจต้อง compile ใหม่)

# 3. Start security monitor
cd /home/ubuntu/bitquan-audit/scripts
chmod +x security-monitor.sh auto-defense.sh
./security-monitor.sh &
./auto-defense.sh &

# 4. Setup Grafana alerts
cp /home/ubuntu/bitquan-audit/alerts/bitquan-alerts.yml \
   /etc/grafana/provisioning/alerting/

sudo systemctl reload grafana-server
```

### Phase 2: Test Defenses (ทำใน 1 ชั่วโมง)

```bash
# Test 1: RPC Rate Limiting
for i in {1..1000}; do
  curl -X POST http://140.245.127.249:19443/ \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"getblockcount","id":1}' &
done
wait
# Expected: Rate limit triggered after ~100 requests

# Test 2: Input Validation
curl -X POST http://140.245.127.249:19443/ \
  -d '{"jsonrpc":"2.0","method":"getblock","params":["<script>alert(1)</script>"],"id":1}'
# Expected: Input validation failure

# Test 3: Authentication
curl -X POST http://140.245.127.249:19443/ \
  -d '{"jsonrpc":"2.0","method":"getblockcount","id":1}'
# Expected: Authentication required (if JWT enabled)

# Test 4: Large Request
dd if=/dev/urandom bs=1M count=10 | base64 > large_payload.txt
curl -X POST http://140.245.127.249:19443/ \
  -d @large_payload.txt
# Expected: Request too large (if MAX_REQUEST_SIZE implemented)
```

### Phase 3: Harden Code (ทำใน 4-8 ชั่วโมง)

**Priority 1** — Double-Spend Protection:
```bash
cd /home/ubuntu/bitquan-audit/crates/mempool
# แก้ไขตาม code snippet ด้านบน
cargo test
cargo build --release
```

**Priority 2** — P2P Peer Limits:
```bash
cd /home/ubuntu/bitquan-audit/crates/network
# เพิ่ม subnet-based connection limits
cargo test
cargo build --release
```

**Priority 3** — RPC Request Size Limit:
```bash
cd /home/ubuntu/bitquan-audit/crates/rpc
# เพิ่ม MAX_REQUEST_SIZE check
cargo test
cargo build --release
```

---

## 📊 Attack Surface Summary

| Component | Current Status | Risk Level | Action Needed |
|-----------|---------------|------------|---------------|
| **RPC Auth** | ✅ JWT + Rate Limit | 🟢 Low | Monitor logs |
| **Input Validation** | ✅ Comprehensive | 🟢 Low | Add request size limit |
| **Mempool** | ⚠️ Unknown UTXO locking | 🔴 Critical | **Audit + Test now** |
| **P2P Network** | ⚠️ Unknown peer limits | 🟡 High | Add subnet diversity |
| **Consensus** | ⚠️ Unknown time validation | 🟡 High | Verify ASERT implementation |
| **Wallet Crypto** | ✅ Dilithium5 + Argon2id | 🟢 Low | Monitor for weak passwords |
| **Storage** | ⚠️ Unknown integrity checks | 🟡 Medium | Add checksums |
| **TLS/SSL** | ✅ Supported | 🟢 Low | Enforce in production |

---

## 🎯 Red Team vs Blue Team Strategy

### Your AI Red Team กำลังทำอะไร?

ให้ฉันเดาจาก common attack patterns:

1. **Network Layer**:
   - Sybil attack (spawn หลายร้อย nodes)
   - Eclipse attack (isolate target nodes)
   - DDoS (flood connections)

2. **RPC Layer**:
   - Brute-force JWT (ถ้ารู้ secret)
   - Rate limit bypass (IP rotation)
   - JSON-RPC injection

3. **Consensus Layer**:
   - Selfish mining simulation
   - Time warp attempts
   - Mining empty blocks (spam)

4. **Transaction Layer**:
   - Double-spend attempts
   - Transaction spam (flood mempool)
   - Dust attacks

5. **Resource Exhaustion**:
   - Memory exhaustion (large requests)
   - CPU exhaustion (signature verification spam)
   - Disk exhaustion (blockchain bloat)

### Blue Team Counter-Strategies

**ต่อ Sybil/Eclipse**:
- ใช้ peer reputation scoring
- Enforce subnet diversity
- ใช้ seed nodes ที่ trust ได้

**ต่อ DDoS**:
- Rate limiting (IP + user + global)
- Connection limits
- SYN cookies (kernel level)
- CloudFlare DDoS protection (สำหรับ public RPC)

**ต่อ Mempool Spam**:
- Minimum fee policy (dynamic fee market)
- Mempool size limits (50k transactions)
- Transaction eviction (lowest fee first)

**ต่อ Double-Spend**:
- Atomic UTXO locking
- Transaction conflict detection
- Replace-by-Fee (RBF) policy

**ต่อ Resource Exhaustion**:
- Request size limits
- Memory quotas per connection
- CPU quotas per peer
- Disk space monitoring + pruning

---

## 🚀 Quick Start Checklist

นายทำตามนี้เลย:

- [ ] 1. รัน security monitor:
  ```bash
  cd /home/ubuntu/bitquan-audit/scripts
  ./security-monitor.sh > /tmp/security.log 2>&1 &
  ```

- [ ] 2. รัน auto-defense:
  ```bash
  ./auto-defense.sh > /tmp/defense.log 2>&1 &
  ```

- [ ] 3. ตรวจสอบ mempool code:
  ```bash
  grep -r "double.spend\|UTXO\|outpoint" crates/mempool/src/
  ```

- [ ] 4. ตรวจสอบ peer limits:
  ```bash
  grep -r "max.*peer\|connection.*limit" crates/network/src/
  ```

- [ ] 5. ทดสอบ RPC rate limiting:
  ```bash
  for i in {1..200}; do
    curl -s http://140.245.127.249:19443/ \
      -d '{"jsonrpc":"2.0","method":"getblockcount","id":1}' &
  done
  wait
  ```

- [ ] 6. Monitor Grafana dashboard:
  ```
  http://140.245.127.249:3030/
  ```

- [ ] 7. ดู security logs real-time:
  ```bash
  tail -f /var/log/bitquan/security.log
  ```

---

## 📞 Emergency Response Plan

ถ้าเกิด incident จริงๆ:

### 🆘 Step 1: Stop the Bleeding
```bash
# Disconnect all peers (isolate node)
./bitquan-cli disconnectnode "*"

# Stop accepting new connections
iptables -A INPUT -p tcp --dport 19444 -j DROP

# Stop RPC server (if being attacked)
iptables -A INPUT -p tcp --dport 19443 -j DROP
```

### 🔍 Step 2: Investigate
```bash
# Dump current state
./bitquan-cli getblockchaininfo > /tmp/blockchain_state.json
./bitquan-cli getpeerinfo > /tmp/peers.json
./bitquan-cli getrawmempool > /tmp/mempool.json
./bitquan-cli getnetworkinfo > /tmp/network.json

# Check logs
tail -n 1000 /var/log/bitquan/debug.log > /tmp/recent_logs.txt

# Check connections
netstat -an | grep :19444 > /tmp/connections.txt
```

### 🛠️ Step 3: Mitigate
```bash
# Clear mempool (if spam attack)
./bitquan-cli clearmempool

# Ban malicious IPs
while read ip; do
  ./bitquan-cli setban "$ip" add 86400  # 24 hours
done < /tmp/bad_ips.txt

# Restart with stricter config
./bitquan-node run \
  --max-peers=20 \
  --max-inbound=10 \
  --rpc-require-auth \
  --enable-strict-validation
```

### 📊 Step 4: Analyze & Report
```bash
# Generate incident report
echo "## BitQuan Security Incident Report" > /tmp/incident_report.md
echo "Date: $(date)" >> /tmp/incident_report.md
echo "" >> /tmp/incident_report.md
echo "### Blockchain State" >> /tmp/incident_report.md
cat /tmp/blockchain_state.json >> /tmp/incident_report.md
echo "" >> /tmp/incident_report.md
echo "### Suspicious IPs" >> /tmp/incident_report.md
cat /tmp/bad_ips.txt >> /tmp/incident_report.md
```

---

## 📚 Resources & References

### Internal Docs
- `/home/ubuntu/bitquan-audit/BLOCKCHAIN_ATTACK_VECTORS.md` — ฉันสร้างไว้แล้ว
- `/home/ubuntu/bitquan-audit/CLAUDE.md` — CI/CD และ build rules
- `/home/ubuntu/bitquan-audit/MODULE_1_TEST_SPECIFICATION_MATRIX.md` — Test matrix

### External References
- **Bitcoin Security Model**: https://bitcoin.org/bitcoin.pdf (Section 6: Incentive)
- **Eclipse Attacks on Bitcoin**: https://eprint.iacr.org/2015/263.pdf
- **Selfish Mining**: https://arxiv.org/abs/1311.0243
- **ASERT DAA**: https://read.cash/@jtoomim/bch-upgrade-proposal-use-asert-as-the-new-daa-1d875696

---

## 💡 ข้อเสนอแนะสำหรับนาย

นายควร:

1. **ให้ AI red team รายงาน attack vectors ที่พวกมันใช้** — จะได้รู้ว่าต้องเน้นป้องกันตรงไหน
2. **Setup honeypot nodes** — nodes ปลอมที่ดูเหมือนจะมีช่องโหว่ เพื่อดัก attackers
3. **Enable verbose logging** — เปิด debug logs เพื่อดู attack patterns
4. **Run testnet in parallel** — ใช้ testnet ทดสอบ defenses ก่อน deploy ไป production
5. **Automated regression testing** — ทุกครั้งที่แก้ช่องโหว่ ต้อง verify ว่าไม่สร้างช่องโหว่ใหม่

**ที่สำคัญ**: อย่า assume ว่า AI red team จะเล่นตามกฎ — มันอาจใช้ zero-day exploits ที่ฉันไม่รู้ก็ได้ ดังนั้น:
- Monitor ทุกอย่าง
- Log ทุกอย่าง
- Alert ทันที
- Respond เร็ว

---

**สร้างโดย**: Hermes (ซากุระ) 🌸  
**วันที่**: 2026-08-15  
**สถานะ**: Active Defense — Real-Time Protection  
**Next Review**: ทุก 6 ชั่วโมง หรือเมื่อมี incident
