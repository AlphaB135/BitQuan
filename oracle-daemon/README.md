# Oracle Daemon - BitQuan Project Monitoring

สร้างระบบรีพอร์ตอัตโนมัติทุกชั่วโมงโดยใช้ Claude API + 10 Parallel Agents

## การติดตั้ง

```bash
cd oracle-daemon
cargo build --release
```

## การใช้งาน

### 1. รันครั้งเดียว (Test)

```bash
# สำหรับ Z.ai (แนะนำ)
export ANTHROPIC_BASE_URL="https://api.z.ai/api/anthropic"
export ANTHROPIC_AUTH_TOKEN="686d28cad99b47aea9d33238783db522.08CQT57ooptrtiE4"
export ANTHROPIC_MODEL="glm-4.7"

./target/release/oracle-daemon --once --project-path "/Volumes/ACASIS Media/BitQuan"
```

### 2. รันเป็น Daemon (ทุก 60 นาที)

```bash
export ANTHROPIC_BASE_URL="https://api.z.ai/api/anthropic"
export ANTHROPIC_AUTH_TOKEN="686d28cad99b47aea9d33238783db522.08CQT57ooptrtiE4"
export ANTHROPIC_MODEL="glm-4.7"

./target/release/oracle-daemon --project-path "/Volumes/ACASIS Media/BitQuan"
```

## ติดตั้งเป็น macOS Service

Plist file ถูกสร้างไว้แล้วที่: `~/Library/LaunchAgents/com.bitquan.oracle.plist`

```bash
# Load และ Start
launchctl load ~/Library/LaunchAgents/com.bitquan.oracle.plist
launchctl start com.bitquan.oracle

# Stop และ Unload
launchctl stop com.bitquan.oracle
launchctl unload ~/Library/LaunchAgents/com.bitquan.oracle.plist

# เช็ค status
launchctl list | grep oracle
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | API endpoint |
| `ANTHROPIC_AUTH_TOKEN` | required | API authentication token |
| `ANTHROPIC_API_KEY` | required | Fallback to AUTH_TOKEN |
| `ANTHROPIC_MODEL` | `claude-haiku-4-5-20251001` | Model to use |

**สำหรับ Z.ai:**
```bash
ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic
ANTHROPIC_AUTH_TOKEN=686d28cad99b47aea9d33238783db522.08CQT57ooptrtiE4
ANTHROPIC_MODEL=glm-4.7
```

## รีพอร์ตที่สร้าง

รีพอร์ตจะถูกเก็บที่: `oracle-reports/`

- `report-YYYY-MM-DD-HH-MM-SS.json` - Machine readable
- `report-YYYY-MM-DD-HH-MM-SS.md` - Human readable
- `daemon.log` - Daemon logs

## 10 Agents

| Agent | Focus |
|-------|-------|
| consensus | Block validation, difficulty, merkle, timestamp |
| network | P2P protocol, peer management, message handling |
| storage | RocksDB, persistence, transactions, recovery |
| node | RPC, worker coordination, chain management |
| error_handling | Error types, propagation, panic/unwrap usage |
| tests | Test coverage, edge cases, organization |
| security | Input validation, DoS protection, rate limiting |
| architecture | Module separation, dependencies, design patterns |
| code_quality | Rust best practices, duplication, documentation |
| performance | Bottlenecks, async usage, memory, caching |

## Configuration

```
Options:
  -p, --project-path <PATH>    Project path [default: .]
  -k, --api-key <KEY>         API key (or ANTHROPIC_AUTH_TOKEN env var)
  -o, --once                  Run once and exit
  -i, --interval-minutes <MIN> Report interval [default: 60]
```

## Monitoring Logs

```bash
# ดู daemon logs
tail -f oracle-reports/daemon.log

# ดูรีพอร์ตล่าสุด
ls -lt oracle-reports/*.md | head -1 | xargs cat
```

## ค่าใช้จ่าย API โดยประมาณ

- รันทุกชม. = 24 ครั้ง/วัน
- 10 agents × ~5,000 tokens/ครั้ง = ~50,000 tokens/ชม.
- ~1.2M tokens/วัน
- แล้วจาก provider และ model ที่ใช้
