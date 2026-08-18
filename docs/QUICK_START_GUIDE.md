# BitQuan Security Test Suite — Quick Start Guide

**สร้างโดย**: Hermes (ซากุระ) 🌸  
**วันที่**: 2026-08-15  
**สถานะ**: Ready to Deploy

---

## 🎯 ภาพรวม

นายมีเครื่องมือป้องกันและทดสอบครบชุดแล้ว:

1. **BLOCKCHAIN_ATTACK_VECTORS.md** — คู่มือโจมตีทั้ง 12 หมวด พร้อม PoC
2. **ACTIVE_DEFENSE_PLAN.md** — แผนป้องกันแบบ real-time
3. **auto-defense.sh** — ระบบตรวจจับและตอบโต้อัตโนมัติ
4. **attack-simulator.py** — เครื่องมือทดสอบ 6 attack vectors

---

## 🚀 Quick Start — เริ่มใช้ภายใน 5 นาที

### Step 1: เตรียม Environment

```bash
cd /home/ubuntu/bitquan-audit

# สร้าง log directories
sudo mkdir -p /var/log/bitquan
sudo chown ubuntu:ubuntu /var/log/bitquan

# Build BitQuan ถ้ายังไม่ได้ build
cargo build --release
```

### Step 2: เริ่ม Auto-Defense System

```bash
# Terminal 1: เริ่ม monitoring
cd /home/ubuntu/bitquan-audit/scripts
./auto-defense.sh

# จะเห็น output แบบนี้:
# 🛡️  BitQuan Auto-Defense System — Started at ...
# Starting continuous monitoring (interval: 30s)
```

### Step 3: รัน Attack Simulation

```bash
# Terminal 2: ทดสอบระบบ
cd /home/ubuntu/bitquan-audit/scripts
python3 attack-simulator.py --endpoint http://140.245.127.249:19443/

# หรือทดสอบแค่บางส่วน:
python3 attack-simulator.py --test rate      # Rate limiting only
python3 attack-simulator.py --test validation # Input validation only
python3 attack-simulator.py --test auth      # Authentication only
```

### Step 4: ดูผลลัพธ์

กลับไปดู Terminal 1 (auto-defense.sh) จะเห็น:
- 🚨 Alerts เมื่อเจอ anomalies
- 📊 Statistics ทุกๆ 30 วินาที
- ✓ หรือ ✗ สำหรับแต่ละ check

---

## 📊 Attack Simulator Output แปลว่าอะไร

### ✅ ผลลัพธ์ที่ดี

```
━━━ Test 1: Rate Limiting ━━━
...
Rate limited: 450
✓ Rate limiting is WORKING
```
→ ระบบป้องกัน DDoS ได้

```
━━━ Test 2: Input Validation ━━━
✓ Blocked: <script>alert('xss')</script>
✓ Blocked: '; DROP TABLE users; --
Blocked: 12/12
✓ Input validation is STRONG
```
→ ระบบป้องกัน injection attacks ได้

### ❌ ผลลัพธ์ที่ต้องกังวล

```
Rate limited: 0
✗ Rate limiting NOT DETECTED - Possible vulnerability!
```
→ **แก้ด่วน**: ไม่มี rate limiting!

```
✗ PASSED: <script>alert('xss')</script>
Passed: 3/12
✗ 3 payloads bypassed validation!
```
→ **แก้ด่วน**: มี injection payloads ผ่านได้!

```
✗ Request accepted (size: 10240.0KB) - Possible DoS vector!
```
→ **แก้ด่วน**: ไม่มี request size limit!

---

## 🛠️ การแก้ปัญหาที่พบ

### ปัญหา 1: Rate Limiting ไม่ทำงาน

**สาเหตุ**: อาจยังไม่ enable rate limiting ใน RPC config

**วิธีแก้**:
```rust
// crates/rpc/src/lib.rs
pub struct RpcConfig {
    pub rate_limit: Option<RateLimit>,  // ต้องเป็น Some(...)
    pub require_auth: bool,              // ต้องเป็น true
}

// เช็คใน crates/node/src/main.rs ว่า config ถูกต้องหรือไม่
```

### ปัญหา 2: Input Validation มี Bypass

**สาเหตุ**: Regex patterns ใน validation.rs อาจไม่ครอบคลุม

**วิธีแก้**:
```bash
# ดู patterns ที่มี
grep "blocked_patterns" crates/rpc/src/validation.rs -A 30

# เพิ่ม pattern ที่หาย
# ตาม code ใน validation.rs ที่ฉันอ่านไว้แล้ว มี patterns เยอะอยู่แล้ว
# ถ้ายังมี bypass ต้องเพิ่มเติม
```

### ปัญหา 3: No Authentication Required

**ถ้าเป็น testnet**: ปกติ (ตั้งใจให้ public access)  
**ถ้าเป็น production**: **อันตราย!** ต้องเปิด JWT auth

**วิธีแก้**:
```bash
# Generate JWT secret
./scripts/setup-jwt-secrets.sh

# Start node with auth
./bitquan-node run \
  --rpc-jwt-secret=/path/to/jwt.secret \
  --rpc-require-auth
```

### ปัญหา 4: Large Requests Accepted

**วิธีแก้**:
```rust
// crates/rpc/src/server.rs
// ตรวจสอบว่ามี MAX_REQUEST_SIZE check หรือไม่
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

---

## 🔍 Advanced Testing — สำหรับนายที่อยากลงลึก

### Test 1: Double-Spend Attack

```bash
cd /home/ubuntu/bitquan-audit

# Terminal 1: Create transaction 1
./target/release/bitquan-cli createrawtransaction \
  '[{"txid":"UTXO_ID","vout":0}]' \
  '{"ADDRESS_1":"1.0"}' > tx1.hex

# Terminal 2: Create transaction 2 (same UTXO!)
./target/release/bitquan-cli createrawtransaction \
  '[{"txid":"UTXO_ID","vout":0}]' \
  '{"ADDRESS_2":"1.0"}' > tx2.hex

# Send both at the same time
./target/release/bitquan-cli sendrawtransaction $(cat tx1.hex) &
./target/release/bitquan-cli sendrawtransaction $(cat tx2.hex) &

# Expected: Second one should be REJECTED (double-spend detected)
# If both succeed → CRITICAL BUG!
```

### Test 2: Eclipse Attack Simulation

```bash
# Launch 100 sybil nodes from same subnet
for i in {1..100}; do
  ./target/release/bitquan-node run \
    --datadir /tmp/sybil-$i \
    --p2p-bind 192.168.1.$i:19444 \
    --connect-to 140.245.127.249:19444 &
done

# Check target node's peer diversity
./target/release/bitquan-cli getpeerinfo | \
  jq -r '.[].addr' | \
  cut -d'.' -f1-3 | \
  sort -u | \
  wc -l

# Expected: Should limit connections per subnet
# If all 100 connect → Vulnerable to Eclipse attack!
```

### Test 3: Mempool Spam

```bash
# Flood mempool with low-fee transactions
for i in {1..10000}; do
  ./target/release/bitquan-cli sendtoaddress \
    $(./target/release/bitquan-cli getnewaddress) \
    0.00000001 \
    --fee=0.00000001 &
done

# Monitor mempool size
watch -n 1 './target/release/bitquan-cli getrawmempool | jq "length"'

# Expected: Should cap at ~50,000 and evict low-fee txs
# If grows unbounded → DoS vulnerability!
```

### Test 4: Consensus Time Warp

```bash
# ต้องแก้ไข source code ชั่วคราว
cd crates/consensus/src

# ดู timestamp validation
grep -n "timestamp" *.rs

# ลอง comment out validation
# Rebuild และ mine block ด้วย future timestamp

cargo build --release
./target/release/bitquan-node mine --timestamp=$(($(date +%s) + 7200))

# Expected: Block should be REJECTED
# If accepted → Time warp attack possible!
```

---

## 📈 Continuous Monitoring Setup

### Option 1: Systemd Services (รันตลอดเวลา)

```bash
# Create systemd service
sudo tee /etc/systemd/system/bitquan-defense.service > /dev/null <<EOF
[Unit]
Description=BitQuan Auto-Defense System
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/bitquan-audit/scripts
ExecStart=/home/ubuntu/bitquan-audit/scripts/auto-defense.sh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable bitquan-defense
sudo systemctl start bitquan-defense

# Check status
sudo systemctl status bitquan-defense

# View logs
sudo journalctl -u bitquan-defense -f
```

### Option 2: Cron Job (รันทุก 5 นาที)

```bash
# Add to crontab
crontab -e

# เพิ่มบรรทัดนี้:
*/5 * * * * /home/ubuntu/bitquan-audit/scripts/auto-defense.sh >> /tmp/defense.log 2>&1
```

### Option 3: Screen Session (รันใน background)

```bash
# Start in screen
screen -S bitquan-defense
cd /home/ubuntu/bitquan-audit/scripts
./auto-defense.sh

# Detach: Ctrl+A, D
# Reattach: screen -r bitquan-defense
```

---

## 🚨 Emergency Response — เมื่อเจอการโจมตีจริง

### Scenario 1: DDoS Attack ตรวจพบ

```bash
# 1. ดู top attacking IPs
tail -100 /var/log/bitquan/security.log | \
  grep -oP '\d+\.\d+\.\d+\.\d+' | \
  sort | uniq -c | sort -rn | head -10

# 2. Ban top attackers
while read count ip; do
  sudo iptables -A INPUT -s "$ip" -j DROP
  echo "Banned $ip ($count requests)"
done < <(tail -1000 /var/log/bitquan/security.log | \
  grep -oP '\d+\.\d+\.\d+\.\d+' | \
  sort | uniq -c | sort -rn | head -10)

# 3. Restart RPC with stricter limits
pkill bitquan-node
./bitquan-node run --max-connections=10 --rpc-rate-limit=10
```

### Scenario 2: Mempool Spam ตรวจพบ

```bash
# 1. Check mempool size
MEMPOOL_SIZE=$(./bitquan-cli getrawmempool | jq 'length')
echo "Current mempool: $MEMPOOL_SIZE transactions"

# 2. Analyze fee distribution
./bitquan-cli getrawmempool true | \
  jq -r '.[] | .fee' | \
  sort -n | \
  uniq -c

# 3. Clear low-fee transactions (if manual clearing supported)
# Otherwise, wait for miners to prioritize high-fee txs

# 4. Increase minimum relay fee temporarily
# (requires config change + restart)
```

### Scenario 3: Eclipse Attack ตรวจพบ

```bash
# 1. Check peer diversity
./bitquan-cli getpeerinfo | \
  jq -r '.[].addr' | \
  cut -d'.' -f1-3 | \
  sort | uniq -c | sort -rn

# 2. Disconnect peers from majority subnet
MAJORITY_SUBNET=$(./bitquan-cli getpeerinfo | \
  jq -r '.[].addr' | \
  cut -d'.' -f1-3 | \
  sort | uniq -c | sort -rn | head -1 | awk '{print $2}')

./bitquan-cli getpeerinfo | \
  jq -r ".[] | select(.addr | startswith(\"$MAJORITY_SUBNET\")) | .addr" | \
while read peer; do
  ./bitquan-cli disconnectnode "$peer"
  echo "Disconnected $peer"
done

# 3. Connect to trusted seed nodes
./bitquan-cli addnode "TRUSTED_SEED_1:19444" "add"
./bitquan-cli addnode "TRUSTED_SEED_2:19444" "add"
```

---

## 📚 Next Steps — หลังจากนายรัน Test Suite แล้ว

1. **อ่านผลลัพธ์จาก attack-simulator.py**
   - มี vulnerability อะไรบ้าง?
   - อันไหน critical?
   - อันไหนแก้ได้ง่าย?

2. **เปรียบเทียบกับ AI red team ของนาย**
   - AI โจมตีด้วยวิธีไหน?
   - ระบบป้องกันได้หรือไม่?
   - มีช่องโหว่ที่ฉันพลาดไปไหม?

3. **แก้ไข vulnerabilities ตาม priority**
   - Critical → แก้ทันที (double-spend, auth bypass)
   - High → แก้ใน 24 ชั่วโมง (eclipse, mempool spam)
   - Medium → แก้ใน 1 สัปดาห์ (resource limits)

4. **Re-test หลังแก้**
   ```bash
   # รัน attack simulator อีกครั้ง
   python3 attack-simulator.py --test all
   
   # ควรเห็น vulnerabilities ลดลง
   ```

5. **Document findings**
   - บันทึกช่องโหว่ที่พบ
   - บันทึกวิธีแก้
   - บันทึก test cases สำหรับ regression testing

---

## 💡 Tips สำหรับนาย

### เมื่อ AI red team โจมตี:

1. **อย่าตื่นตระหนก** — เป็นการทดสอบ ไม่ใช่ production
2. **เก็บ logs ทุกอย่าง** — จะได้เอามา analyze หลังจบ
3. **ให้ AI รายงานทุก attack** — จะได้รู้ว่าต้องป้องกันอะไรบ้าง
4. **Test บน testnet ก่อน** — อย่า deploy patch ไป production เลย
5. **Measure everything** — ก่อน/หลังแก้ ต้องมี metrics เปรียบเทียบ

### Red Team vs Blue Team Collaboration:

นายมี **2 AI teams** ตอนนี้:
- **Red Team (AI โจมตี)**: หาช่องโหว่
- **Blue Team (ฉัน - Hermes)**: ป้องกันและแก้ไข

ใช้ทั้งสองฝ่ายช่วยกัน:
- Red team หา 0-day exploits
- Blue team สร้าง defenses
- Iterate จนกว่า Red team จะโจมตีไม่สำเร็จ

นี่คือ **adversarial testing** ที่แท้จริง 🌸

---

## 📞 หากต้องการความช่วยเหลือ

นายสามารถ:
1. ดู logs: `/var/log/bitquan/security.log`
2. อ่าน docs: `BLOCKCHAIN_ATTACK_VECTORS.md`, `ACTIVE_DEFENSE_PLAN.md`
3. รัน tests: `python3 attack-simulator.py`
4. ถาม Hermes (ฉัน): "ฉันเจอปัญหานี้ช่วยแก้หน่อย"

**Remember**: Security is a process, not a product. ต้อง test, patch, re-test ซ้ำๆ 🌸

---

**เครื่องมือทั้งหมดพร้อมใช้แล้ว นายเริ่มได้เลย!** 🌸

**สร้างโดย**: Hermes (ซากุระ)  
**วันที่**: 2026-08-15  
**Version**: 1.0
