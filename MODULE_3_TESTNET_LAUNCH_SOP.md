# BitQuan Layer-1 Blockchain — Public Testnet Launch Checklist & SOP

**Document Version:** 1.0.0  
**Date:** 2026-08-14  
**Author:** Principal L1 Blockchain Architect & Head of Core Engineering  
**Status:** Pre-Testnet Phase 1 — Launch Operations Manual  

---

## Executive Summary

This document provides the authoritative Standard Operating Procedure (SOP) for launching BitQuan Public Testnet Phase 1. It covers genesis ceremony, infrastructure topology, public service deployment, monitoring, and incident response protocols calibrated for a high-security blockchain launch.

**Launch Readiness Gates:**
1. All CRITICAL + HIGH priority tests passing (26/26)
2. Security audit findings resolved (C1-C7 complete)
3. Infrastructure provisioned and tested
4. Incident response team trained
5. Public communications prepared

**Launch Timeline:** Q2 2026 (Target: 2026-09-01)

---

## Table of Contents

1. [Pre-Launch Checklist](#1-pre-launch-checklist)
2. [Genesis Ceremony](#2-genesis-ceremony)
3. [Infrastructure Topology](#3-infrastructure-topology)
4. [Node Deployment](#4-node-deployment)
5. [Public Services](#5-public-services)
6. [Monitoring & Alerting](#6-monitoring--alerting)
7. [Incident Response](#7-incident-response)
8. [Post-Launch Operations](#8-post-launch-operations)

---

## 1. Pre-Launch Checklist

### 1.1 Code Quality Gates

**Status Check:** Complete these before proceeding to infrastructure setup.

| Item | Status | Verification Command | Owner |
|------|--------|---------------------|-------|
| **Comprehensive Test Suite** | ⬜ | `./scripts/run-all-tests.sh` | QA Lead |
| All 26 CRITICAL+HIGH tests passing | ⬜ | Check test report | QA Lead |
| Zero `unsafe` blocks without SAFETY comments | ⬜ | `cargo clippy -- -D clippy::undocumented_unsafe_blocks` | Core Dev |
| Zero `unwrap()` in production code | ⬜ | `cargo clippy -- -D clippy::unwrap_used` | Core Dev |
| Code coverage ≥ 65% | ⬜ | `cargo llvm-cov --workspace` | QA Lead |
| cargo-deny security audit clean | ⬜ | `cargo deny check` | Security Lead |
| Fuzz testing (24h soak) | ⬜ | `cargo +nightly fuzz run` | QA Lead |
| **Documentation** | ⬜ | | |
| CLAUDE.md updated | ⬜ | Manual review | Tech Writer |
| README.md testnet instructions | ⬜ | Manual review | Tech Writer |
| SDK documentation published | ⬜ | Check docs.bitquan.io | Tech Writer |
| **Security** | ⬜ | | |
| C1-C7 vulnerabilities resolved | ⬜ | Review security audit report | Security Lead |
| JWT secret generation documented | ⬜ | Check ops runbook | Security Lead |
| TLS certificates provisioned | ⬜ | Check Let's Encrypt | DevOps Lead |
| Firewall rules configured | ⬜ | Check iptables/ufw | DevOps Lead |
| **Infrastructure** | ⬜ | | |
| Seed nodes provisioned (3 minimum) | ⬜ | SSH check all nodes | DevOps Lead |
| DNS seeds configured | ⬜ | `dig seed.testnet.bitquan.io` | DevOps Lead |
| RPC gateway deployed | ⬜ | `curl https://rpc.testnet.bitquan.io/health` | DevOps Lead |
| Block explorer online | ⬜ | Check explorer.testnet.bitquan.io | Frontend Dev |
| Faucet deployed | ⬜ | Check faucet.testnet.bitquan.io | Backend Dev |
| Monitoring stack operational | ⬜ | Check Grafana dashboard | DevOps Lead |
| **Incident Response** | ⬜ | | |
| On-call rotation established | ⬜ | PagerDuty configured | Ops Manager |
| Rollback procedures documented | ⬜ | Review runbook | Ops Manager |
| Emergency contacts list updated | ⬜ | Check Slack channel | Ops Manager |
| **Communications** | ⬜ | | |
| Announcement blog post ready | ⬜ | Review draft | Marketing |
| Discord/Telegram moderators briefed | ⬜ | Check community channels | Community Manager |
| FAQ updated | ⬜ | Review docs | Community Manager |

**Sign-off Required:**
- [ ] Core Dev Lead: _________________ Date: _______
- [ ] QA Lead: _______________________ Date: _______
- [ ] Security Lead: _________________ Date: _______
- [ ] DevOps Lead: ___________________ Date: _______
- [ ] Project Manager: _______________ Date: _______

**⚠️ DO NOT PROCEED TO GENESIS WITHOUT COMPLETE SIGN-OFF**

---

## 2. Genesis Ceremony

### 2.1 Genesis Parameters

**Network Configuration:**

```toml
# config/testnet-genesis.toml

[network]
network_id = "testnet"
magic_bytes = [0x42, 0x51, 0x54, 0x4E]  # "BQTN"
default_port = 19444
rpc_port = 19443

[genesis]
timestamp = 1725177600  # 2026-09-01 00:00:00 UTC
version = 1
bits = 0x207fffff      # Minimum difficulty (testnet)
nonce = 0

[genesis.coinbase]
# Testnet treasury address (multisig 3-of-5)
address = "bq1qtr3asury5f0rt3sttn3tm4lt1s1g2024"
value = 5000000000000000000  # 50 BQ (10% treasury allocation)

# Genesis miner address (burn address - provably unspendable)
miner_address = "bq1qg3n3s1sb10ck2024burn"
miner_value = 45000000000000000000  # 45 BQ

[consensus]
target_block_time = 120       # 2 minutes
difficulty_half_life = 14400  # 4 hours (testnet)
max_block_weight = 4000000    # 4 MB
signature_weight_alpha = 384

[reward]
initial_subsidy = 50000000000000000000  # 50 BQ
halving_interval = 210000
tail_emission = 500000000000000000      # 0.5 BQ
```

### 2.2 Genesis Block Generation

**Process:** Execute on air-gapped machine for maximum security.

```bash
#!/usr/bin/env bash
# File: scripts/genesis-ceremony.sh

set -euo pipefail

echo "🎬 BitQuan Testnet Genesis Ceremony"
echo "===================================="
echo ""
echo "⚠️  This script should be run on an AIR-GAPPED machine"
echo ""
read -p "Is this machine air-gapped? (yes/no): " airgap_confirm

if [ "$airgap_confirm" != "yes" ]; then
    echo "❌ Genesis ceremony ABORTED"
    exit 1
fi

# Step 1: Set genesis timestamp (MUST be in the future)
GENESIS_TIME=1725177600  # 2026-09-01 00:00:00 UTC
CURRENT_TIME=$(date +%s)

echo ""
echo "Step 1: Verify genesis timestamp"
echo "  Genesis time: $(date -d @$GENESIS_TIME)"
echo "  Current time: $(date -d @$CURRENT_TIME)"

if [ "$GENESIS_TIME" -lt "$CURRENT_TIME" ]; then
    echo "  ❌ Genesis timestamp is in the past!"
    exit 1
fi

echo "  ✅ Genesis timestamp valid"

# Step 2: Generate treasury multisig address
echo ""
echo "Step 2: Generate treasury multisig address (3-of-5)"

# Generate 5 keypairs for treasury
for i in {1..5}; do
    ./target/release/bitquan-node wallet-gen \
        --output "treasury-key-$i.keystore" \
        --password "TREASURY_PASSWORD_$i"
    
    PUBKEY=$(./target/release/bitquan-node wallet-pubkey \
        --keystore "treasury-key-$i.keystore" \
        --password "TREASURY_PASSWORD_$i")
    
    echo "  Treasury key $i: $PUBKEY"
done

# Create multisig address (3-of-5)
TREASURY_ADDR=$(./target/release/bitquan-node create-multisig \
    --required 3 \
    --pubkeys treasury-key-{1..5}.pubkey)

echo "  ✅ Treasury address: $TREASURY_ADDR"

# Step 3: Generate genesis block
echo ""
echo "Step 3: Generate genesis block"

./target/release/bitquan-node mine-genesis \
    --timestamp "$GENESIS_TIME" \
    --treasury-address "$TREASURY_ADDR" \
    --bits 0x207fffff \
    --output genesis-block.json

GENESIS_HASH=$(jq -r '.hash' genesis-block.json)

echo "  ✅ Genesis block generated"
echo "  Genesis hash: $GENESIS_HASH"

# Step 4: Verify genesis block
echo ""
echo "Step 4: Verify genesis block"

./target/release/bitquan-node verify-genesis \
    --genesis genesis-block.json

echo "  ✅ Genesis block verified"

# Step 5: Export for distribution
echo ""
echo "Step 5: Export genesis for distribution"

# Create genesis package
mkdir -p genesis-package
cp genesis-block.json genesis-package/
cp config/testnet-genesis.toml genesis-package/

# Generate checksums
cd genesis-package
sha256sum genesis-block.json > checksums.txt
sha256sum testnet-genesis.toml >> checksums.txt

# Sign with core dev key
gpg --clearsign checksums.txt

cd ..

echo "  ✅ Genesis package created: genesis-package/"
echo ""
echo "📦 Genesis Ceremony Complete!"
echo ""
echo "Next steps:"
echo "  1. Transfer genesis-package/ to online machines via USB"
echo "  2. Verify checksums: sha256sum -c checksums.txt"
echo "  3. Verify GPG signature: gpg --verify checksums.txt.asc"
echo "  4. Deploy genesis block to seed nodes"
echo ""
echo "⚠️  CRITICAL: Backup treasury-key-*.keystore files to secure offline storage"
```

**Execution:**
```bash
# On air-gapped machine
cargo build --release
./scripts/genesis-ceremony.sh

# Backup treasury keys to 5 separate USB drives
for i in {1..5}; do
    cp treasury-key-$i.keystore /media/usb-$i/
done

# Transfer genesis package to online deployment machine
cp -r genesis-package/ /media/usb-deploy/
```

### 2.3 Genesis Verification Checklist

After genesis ceremony, verify these properties:

```bash
#!/usr/bin/env bash
# File: scripts/verify-genesis-properties.sh

set -euo pipefail

GENESIS_FILE="${1:-genesis-block.json}"

echo "🔍 Verifying genesis block properties..."

# Property 1: Correct timestamp
GENESIS_TIME=$(jq -r '.header.time' "$GENESIS_FILE")
if [ "$GENESIS_TIME" -eq 1725177600 ]; then
    echo "  ✅ Timestamp correct: $GENESIS_TIME"
else
    echo "  ❌ Timestamp incorrect: $GENESIS_TIME (expected: 1725177600)"
    exit 1
fi

# Property 2: Minimum difficulty
BITS=$(jq -r '.header.bits' "$GENESIS_FILE")
if [ "$BITS" = "0x207fffff" ]; then
    echo "  ✅ Difficulty bits correct: $BITS"
else
    echo "  ❌ Difficulty bits incorrect: $BITS"
    exit 1
fi

# Property 3: Coinbase outputs
OUTPUTS=$(jq -r '.transactions[0].outputs | length' "$GENESIS_FILE")
if [ "$OUTPUTS" -eq 2 ]; then
    echo "  ✅ Coinbase has 2 outputs (treasury + miner)"
else
    echo "  ❌ Coinbase outputs incorrect: $OUTPUTS (expected: 2)"
    exit 1
fi

# Property 4: Total supply = 50 BQ
OUTPUT_SUM=$(jq '[.transactions[0].outputs[].value] | add' "$GENESIS_FILE")
EXPECTED_SUM=50000000000000000000  # 50 BQ

if [ "$OUTPUT_SUM" -eq "$EXPECTED_SUM" ]; then
    echo "  ✅ Total supply correct: 50 BQ"
else
    echo "  ❌ Total supply incorrect: $OUTPUT_SUM (expected: $EXPECTED_SUM)"
    exit 1
fi

# Property 5: Treasury receives 10%
TREASURY_VALUE=$(jq -r '.transactions[0].outputs[0].value' "$GENESIS_FILE")
EXPECTED_TREASURY=5000000000000000000  # 5 BQ

if [ "$TREASURY_VALUE" -eq "$EXPECTED_TREASURY" ]; then
    echo "  ✅ Treasury allocation correct: 5 BQ (10%)"
else
    echo "  ❌ Treasury allocation incorrect: $TREASURY_VALUE"
    exit 1
fi

echo ""
echo "✅ All genesis properties verified"
```

---

## 3. Infrastructure Topology

### 3.1 Network Architecture

```
                          Internet
                             |
                    ┌────────┴────────┐
                    │  CloudFlare CDN  │
                    │  DDoS Protection │
                    └────────┬────────┘
                             |
                ┌────────────┴────────────┐
                │                         │
         ┌──────▼──────┐          ┌──────▼──────┐
         │  RPC Gateway │          │   Website   │
         │  (Nginx TLS) │          │  (Static)   │
         └──────┬──────┘          └─────────────┘
                │
        ┌───────┴───────┐
        │               │
  ┌─────▼─────┐   ┌─────▼─────┐   ┌─────────┐
  │  Seed-1   │◄──┤  Seed-2   │◄──┤ Seed-3  │
  │ (Oregon)  │   │ (N.Virg.) │   │ (Tokyo) │
  └─────┬─────┘   └─────┬─────┘   └────┬────┘
        │               │               │
        └───────┬───────┴───────┬───────┘
                │               │
       Public P2P Network   ┌───▼────┐
        (port 19444)        │Faucet  │
                            │Service │
                            └────┬───┘
                                 │
                          ┌──────▼──────┐
                          │   Explorer  │
                          │   Backend   │
                          └─────────────┘
```

### 3.2 Server Specifications

| Component | Instance Type | vCPU | RAM | Storage | Network | Cost/Month | Provider |
|-----------|--------------|------|-----|---------|---------|------------|----------|
| **Seed Node** | c6i.2xlarge | 8 | 16GB | 500GB SSD | 10Gbps | $245 | AWS |
| **RPC Gateway** | t3.large | 2 | 8GB | 100GB | 5Gbps | $68 | AWS |
| **Faucet** | t3.medium | 2 | 4GB | 50GB | 1Gbps | $34 | AWS |
| **Explorer** | t3.xlarge | 4 | 16GB | 200GB | 5Gbps | $136 | AWS |
| **Monitoring** | t3.large | 2 | 8GB | 100GB | 1Gbps | $68 | AWS |

**Total Infrastructure Cost:** ~$1,100/month (3 seed nodes + supporting services)

### 3.3 DNS Configuration

**Primary Domain:** `testnet.bitquan.io`

```dns
; DNS Zone File for testnet.bitquan.io

; Seed nodes (DNS seeds for peer discovery)
seed.testnet.bitquan.io.    IN  A     52.10.20.30     ; seed-1 Oregon
seed.testnet.bitquan.io.    IN  A     3.85.14.52      ; seed-2 N.Virginia
seed.testnet.bitquan.io.    IN  A     13.231.45.78    ; seed-3 Tokyo

; Individual seed nodes (for debugging)
seed-1.testnet.bitquan.io.  IN  A     52.10.20.30
seed-2.testnet.bitquan.io.  IN  A     3.85.14.52
seed-3.testnet.bitquan.io.  IN  A     13.231.45.78

; Public services
rpc.testnet.bitquan.io.     IN  CNAME  rpc-gateway.testnet.bitquan.io.
explorer.testnet.bitquan.io. IN CNAME  explorer-backend.testnet.bitquan.io.
faucet.testnet.bitquan.io.  IN  CNAME  faucet-service.testnet.bitquan.io.
docs.testnet.bitquan.io.    IN  CNAME  bitquan.github.io.

; Monitoring
grafana.testnet.bitquan.io. IN  A     10.0.5.100  ; Internal only
prometheus.testnet.bitquan.io. IN A  10.0.5.101  ; Internal only
```

**DNSSEC:** Enable for `testnet.bitquan.io` to prevent DNS spoofing.

---

## 4. Node Deployment

### 4.1 Seed Node Deployment Playbook

**Ansible Playbook:** `infra/ansible/deploy-seed-node.yml`

```yaml
---
- name: Deploy BitQuan Seed Node
  hosts: seed_nodes
  become: yes
  vars:
    bitquan_version: "v1.0.0-testnet"
    bitquan_user: "bitquan"
    data_dir: "/var/lib/bitquan"
    log_dir: "/var/log/bitquan"
    
  tasks:
    - name: Create bitquan user
      user:
        name: "{{ bitquan_user }}"
        system: yes
        shell: /bin/bash
        home: "{{ data_dir }}"
    
    - name: Install system dependencies
      apt:
        name:
          - build-essential
          - pkg-config
          - libssl-dev
          - ufw
          - fail2ban
          - prometheus-node-exporter
        state: present
        update_cache: yes
    
    - name: Configure firewall
      ufw:
        rule: allow
        port: "{{ item }}"
        proto: tcp
      loop:
        - "22"     # SSH
        - "19443"  # RPC (internal only)
        - "19444"  # P2P
    
    - name: Enable firewall
      ufw:
        state: enabled
        policy: deny
    
    - name: Download BitQuan binary
      get_url:
        url: "https://github.com/BitQuan/releases/download/{{ bitquan_version }}/bitquan-node-linux-amd64"
        dest: "/usr/local/bin/bitquan-node"
        mode: '0755'
        checksum: "sha256:{{ bitquan_checksum }}"
    
    - name: Create data directory
      file:
        path: "{{ data_dir }}"
        state: directory
        owner: "{{ bitquan_user }}"
        group: "{{ bitquan_user }}"
        mode: '0750'
    
    - name: Copy genesis block
      copy:
        src: "genesis-package/genesis-block.json"
        dest: "{{ data_dir }}/genesis.json"
        owner: "{{ bitquan_user }}"
        group: "{{ bitquan_user }}"
        mode: '0640'
    
    - name: Copy node configuration
      template:
        src: templates/testnet-config.toml.j2
        dest: "{{ data_dir }}/config.toml"
        owner: "{{ bitquan_user }}"
        group: "{{ bitquan_user }}"
        mode: '0640'
    
    - name: Install systemd service
      template:
        src: templates/bitquan-node.service.j2
        dest: /etc/systemd/system/bitquan-node.service
        mode: '0644'
      notify: Reload systemd
    
    - name: Enable and start service
      systemd:
        name: bitquan-node
        enabled: yes
        state: started
    
    - name: Wait for node to be healthy
      uri:
        url: "http://localhost:19443/health"
        status_code: 200
      register: result
      until: result.status == 200
      retries: 30
      delay: 10
  
  handlers:
    - name: Reload systemd
      systemd:
        daemon_reload: yes
```

**Systemd Service Template:**

```ini
# File: templates/bitquan-node.service.j2

[Unit]
Description=BitQuan Testnet Seed Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User={{ bitquan_user }}
Group={{ bitquan_user }}
WorkingDirectory={{ data_dir }}

ExecStart=/usr/local/bin/bitquan-node run \
    --config {{ data_dir }}/config.toml \
    --datadir {{ data_dir }} \
    --network testnet

Restart=on-failure
RestartSec=10
KillMode=process
KillSignal=SIGTERM
TimeoutStopSec=30

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths={{ data_dir }} {{ log_dir }}

# Resource limits
LimitNOFILE=65536
LimitNPROC=512

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=bitquan-node

[Install]
WantedBy=multi-user.target
```

**Deployment Execution:**

```bash
# 1. Update inventory
vi infra/ansible/inventory/testnet.ini

# 2. Test connectivity
ansible seed_nodes -m ping -i infra/ansible/inventory/testnet.ini

# 3. Deploy
ansible-playbook -i infra/ansible/inventory/testnet.ini \
    infra/ansible/deploy-seed-node.yml \
    --extra-vars "bitquan_checksum=$(sha256sum target/release/bitquan-node | awk '{print $1}')"

# 4. Verify deployment
ansible seed_nodes -m shell \
    -a "systemctl status bitquan-node" \
    -i infra/ansible/inventory/testnet.ini
```

### 4.2 Post-Deployment Verification

**Checklist:** Run on each deployed seed node.

```bash
#!/usr/bin/env bash
# File: scripts/verify-seed-node.sh

set -euo pipefail

NODE_IP="${1:-seed-1.testnet.bitquan.io}"

echo "🔍 Verifying seed node: $NODE_IP"
echo ""

# Check 1: Service running
echo "Check 1: Service status"
ssh "ubuntu@$NODE_IP" "systemctl is-active bitquan-node" > /dev/null
echo "  ✅ Service is running"

# Check 2: RPC responding
echo "Check 2: RPC health"
RESPONSE=$(curl -sf "http://$NODE_IP:19443/health")
if [ "$RESPONSE" = '{"status":"ok"}' ]; then
    echo "  ✅ RPC is healthy"
else
    echo "  ❌ RPC health check failed"
    exit 1
fi

# Check 3: P2P port open
echo "Check 3: P2P connectivity"
nc -zv "$NODE_IP" 19444 2>&1 | grep -q "succeeded"
echo "  ✅ P2P port accessible"

# Check 4: Genesis block matches
echo "Check 4: Genesis block verification"
REMOTE_GENESIS=$(curl -s "http://$NODE_IP:19443/rpc" \
    -d '{"method":"getblockhash","params":[0],"id":1}' \
    | jq -r '.result')

LOCAL_GENESIS=$(jq -r '.hash' genesis-package/genesis-block.json)

if [ "$REMOTE_GENESIS" = "$LOCAL_GENESIS" ]; then
    echo "  ✅ Genesis block matches"
else
    echo "  ❌ Genesis mismatch!"
    echo "    Remote: $REMOTE_GENESIS"
    echo "    Local:  $LOCAL_GENESIS"
    exit 1
fi

# Check 5: Peer connections
echo "Check 5: Peer connections"
PEER_COUNT=$(curl -s "http://$NODE_IP:19443/rpc" \
    -d '{"method":"getpeerinfo","params":[],"id":1}' \
    | jq -r '.result | length')

echo "  ℹ️  Connected peers: $PEER_COUNT (minimum 2 required for healthy network)"

if [ "$PEER_COUNT" -ge 2 ]; then
    echo "  ✅ Sufficient peers"
else
    echo "  ⚠️  Warning: Low peer count (may improve after all seeds online)"
fi

echo ""
echo "✅ Seed node verification complete"
```

---

## 5. Public Services

### 5.1 RPC Gateway (Nginx + TLS)

**Configuration:** `/etc/nginx/sites-available/rpc-gateway`

```nginx
upstream bitquan_rpc {
    least_conn;
    server seed-1.testnet.bitquan.io:19443 max_fails=3 fail_timeout=30s;
    server seed-2.testnet.bitquan.io:19443 max_fails=3 fail_timeout=30s;
    server seed-3.testnet.bitquan.io:19443 max_fails=3 fail_timeout=30s;
}

# Rate limiting zones
limit_req_zone $binary_remote_addr zone=rpc_limit:10m rate=10r/s;
limit_conn_zone $binary_remote_addr zone=conn_limit:10m;

server {
    listen 443 ssl http2;
    server_name rpc.testnet.bitquan.io;
    
    # TLS configuration
    ssl_certificate /etc/letsencrypt/live/rpc.testnet.bitquan.io/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/rpc.testnet.bitquan.io/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    
    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    
    # Rate limiting
    limit_req zone=rpc_limit burst=20 nodelay;
    limit_conn conn_limit 10;
    
    # Logging
    access_log /var/log/nginx/rpc-access.log combined;
    error_log /var/log/nginx/rpc-error.log warn;
    
    location /rpc {
        proxy_pass http://bitquan_rpc;
        proxy_http_version 1.1;
        
        # Timeouts
        proxy_connect_timeout 5s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # Headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # No buffering (streaming)
        proxy_buffering off;
    }
    
    location /health {
        proxy_pass http://bitquan_rpc/health;
        access_log off;
    }
    
    location / {
        return 404 '{"error":"Use /rpc endpoint"}';
        add_header Content-Type application/json;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name rpc.testnet.bitquan.io;
    return 301 https://$server_name$request_uri;
}
```

### 5.2 Faucet Service

**Docker Compose:** `docker-compose.faucet.yml`

```yaml
version: "3.8"

services:
  faucet:
    build:
      context: ./crates/faucet
      dockerfile: Dockerfile
    container_name: bitquan-faucet
    restart: unless-stopped
    
    environment:
      - BITQUAN_RPC_URL=https://rpc.testnet.bitquan.io/rpc
      - FAUCET_PORT=5000
      - FAUCET_DRIP_AMOUNT=10.0
      - FAUCET_COOLDOWN_SECONDS=86400  # 24 hours
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET_PATH=/run/secrets/faucet_jwt
    
    ports:
      - "5000:5000"
    
    secrets:
      - faucet_jwt
      - faucet_wallet
    
    volumes:
      - faucet-data:/data
    
    depends_on:
      - redis
    
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:5000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
  
  redis:
    image: redis:7-alpine
    container_name: faucet-redis
    restart: unless-stopped
    
    volumes:
      - redis-data:/data
    
    command: redis-server --appendonly yes

volumes:
  faucet-data:
  redis-data:

secrets:
  faucet_jwt:
    file: ./secrets/faucet-jwt.txt
  faucet_wallet:
    file: ./secrets/faucet-wallet.keystore
```

**Deployment:**

```bash
# 1. Generate faucet wallet
./target/release/bitquan-node wallet-gen \
    --output secrets/faucet-wallet.keystore \
    --password "$(openssl rand -base64 32)"

# 2. Fund faucet from treasury (1,000,000 BQ)
# (Requires 3-of-5 treasury multisig authorization)

# 3. Deploy faucet
docker compose -f docker-compose.faucet.yml up -d

# 4. Verify
curl -s https://faucet.testnet.bitquan.io/health | jq
```

### 5.3 Block Explorer

**Component:** Separate repository `BitQuan-Explorer`

**Stack:**
- Backend: Rust (Axum) + PostgreSQL
- Frontend: React + TailwindCSS
- Indexer: Custom daemon syncing from seed nodes

**Deployment:** Automated via CI/CD to AWS ECS.

---

## 6. Monitoring & Alerting

### 6.1 Prometheus Metrics

**Metrics Exposed:** `http://localhost:9090/metrics`

```prometheus
# Node metrics
bitquan_block_height                    # Current block height
bitquan_peer_count                      # Connected peers
bitquan_mempool_size_bytes              # Mempool memory usage
bitquan_mempool_tx_count                # Transactions in mempool
bitquan_sync_status                     # 0=syncing, 1=synced
bitquan_chain_work                      # Cumulative chain work
bitquan_last_block_time_seconds         # Timestamp of last block
bitquan_rpc_requests_total              # RPC request counter
bitquan_rpc_errors_total                # RPC error counter
bitquan_network_bytes_sent_total        # P2P bytes sent
bitquan_network_bytes_recv_total        # P2P bytes received
bitquan_ban_count                       # Number of banned peers
bitquan_reorg_depth                     # Last reorg depth
bitquan_difficulty                      # Current network difficulty
```

**Prometheus Configuration:**

```yaml
# File: prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']

scrape_configs:
  - job_name: 'bitquan-seeds'
    static_configs:
      - targets:
          - 'seed-1.testnet.bitquan.io:9090'
          - 'seed-2.testnet.bitquan.io:9090'
          - 'seed-3.testnet.bitquan.io:9090'
```

### 6.2 Grafana Dashboard

**Dashboard JSON:** `monitoring/grafana/bitquan-testnet-dashboard.json`

**Panels:**
1. **Network Health**
   - Block height (all nodes)
   - Peer count
   - Sync status
   
2. **Transaction Activity**
   - Mempool size
   - Transaction rate (tx/sec)
   - Fee distribution
   
3. **Resource Usage**
   - CPU usage
   - Memory usage
   - Disk I/O
   - Network bandwidth
   
4. **Security**
   - Ban events
   - Failed RPC auth attempts
   - Reorg alerts

### 6.3 Alert Rules

**File:** `prometheus-alerts.yml`

```yaml
groups:
  - name: bitquan_critical
    interval: 30s
    rules:
      # Chain stalled (no new blocks in 10 minutes)
      - alert: ChainStalled
        expr: time() - bitquan_last_block_time_seconds > 600
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Chain stalled on {{ $labels.instance }}"
          description: "No new blocks in 10+ minutes"
      
      # Node out of sync
      - alert: NodeOutOfSync
        expr: bitquan_sync_status == 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Node {{ $labels.instance }} out of sync"
      
      # Low peer count
      - alert: LowPeerCount
        expr: bitquan_peer_count < 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count on {{ $labels.instance }}"
          description: "Only {{ $value }} peer(s) connected"
      
      # Mempool overflow
      - alert: MempoolOverflow
        expr: bitquan_mempool_size_bytes > 300000000  # 300 MB
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Mempool overflow on {{ $labels.instance }}"
          description: "Mempool size: {{ $value | humanize }}B"
      
      # Deep reorg detected
      - alert: DeepReorg
        expr: bitquan_reorg_depth > 10
        labels:
          severity: critical
        annotations:
          summary: "Deep reorg detected: {{ $value }} blocks"
          description: "Possible 51% attack or chain split"
      
      # Node down
      - alert: NodeDown
        expr: up{job="bitquan-seeds"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Node {{ $labels.instance }} is down"
```

**PagerDuty Integration:**

```yaml
# alertmanager.yml

global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'cluster']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'pagerduty-critical'
  
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty-critical'
    
    - match:
        severity: warning
      receiver: 'slack-warnings'

receivers:
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '<PAGERDUTY_SERVICE_KEY>'
        description: '{{ .CommonAnnotations.summary }}'
  
  - name: 'slack-warnings'
    slack_configs:
      - api_url: '<SLACK_WEBHOOK_URL>'
        channel: '#bitquan-alerts'
        title: '{{ .CommonAnnotations.summary }}'
        text: '{{ .CommonAnnotations.description }}'
```

---

## 7. Incident Response

### 7.1 Emergency Contacts

| Role | Name | Phone | Email | Timezone |
|------|------|-------|-------|----------|
| On-Call Lead | [NAME] | [PHONE] | [EMAIL] | UTC+0 |
| Core Dev Lead | [NAME] | [PHONE] | [EMAIL] | UTC-8 |
| Security Lead | [NAME] | [PHONE] | [EMAIL] | UTC+1 |
| DevOps Lead | [NAME] | [PHONE] | [EMAIL] | UTC-5 |
| Project Manager | [NAME] | [PHONE] | [EMAIL] | UTC+0 |

**Escalation Path:**
1. On-Call Engineer (PagerDuty) → 5 min response SLA
2. On-Call Lead → 15 min response SLA
3. Core Dev Lead + Security Lead → 30 min response SLA
4. Executive Team → 1 hour notification

### 7.2 Incident Severity Levels

| Level | Definition | Response Time | Examples |
|-------|------------|---------------|----------|
| **P0 - Critical** | Complete service outage or security breach | **15 minutes** | Chain stalled, all nodes down, 51% attack |
| **P1 - High** | Major functionality impaired | **1 hour** | Single seed node down, deep reorg, RPC gateway offline |
| **P2 - Medium** | Degraded performance | **4 hours** | High mempool, slow block times, faucet issues |
| **P3 - Low** | Minor issues, no user impact | **24 hours** | Documentation errors, monitoring gaps |

### 7.3 Runbook: Chain Stalled (P0)

**Symptoms:**
- No new blocks mined in 10+ minutes
- All miners report "difficulty too high"
- Network hashrate appears to have dropped 90%+

**Immediate Actions (15 min):**

```bash
# 1. Verify stall on all seed nodes
for seed in seed-{1..3}.testnet.bitquan.io; do
    echo "Checking $seed..."
    LAST_BLOCK=$(curl -s "http://$seed:19443/rpc" \
        -d '{"method":"getblockcount"}' | jq -r '.result')
    
    LAST_TIME=$(curl -s "http://$seed:19443/rpc" \
        -d "{\"method\":\"getblock\",\"params\":[\"$LAST_BLOCK\",true]}" \
        | jq -r '.result.time')
    
    AGE=$(($(date +%s) - LAST_TIME))
    echo "  Block $LAST_BLOCK age: ${AGE}s"
done

# 2. Check current difficulty
CURRENT_BITS=$(curl -s "http://seed-1.testnet.bitquan.io:19443/rpc" \
    -d '{"method":"getblockchaininfo"}' | jq -r '.result.difficulty')

echo "Current difficulty: $CURRENT_BITS"

# 3. Emergency difficulty adjustment (if confirmed stall)
# This requires consensus among Core Dev team
# DO NOT execute without authorization

# Option A: Soft recovery (preferred)
# Announce to miners: "Testnet experiencing difficulty spike, mining continues"
# Wait for ASERT to naturally adjust (may take 2-4 hours)

# Option B: Hard recovery (emergency only)
# Deploy hotfix with difficulty cap override
# Requires 3-of-5 core dev signatures

echo "⚠️  DECISION REQUIRED: Soft or hard recovery?"
echo "   Contact Core Dev Lead immediately"
```

**Communication Template:**

```
🚨 TESTNET ALERT - Chain Stalled

Status: Investigating
Time: [TIMESTAMP] UTC
Last Block: [HEIGHT]
Age: [MINUTES] minutes

Impact: New transactions not confirming

Action: Engineering team investigating. No action required from users.

Updates: We'll post updates every 15 minutes to:
  - Discord: #testnet-status
  - Twitter: @BitQuanOfficial
  - Status page: status.bitquan.io

Next update: [TIME] UTC
```

### 7.4 Runbook: Deep Reorg (P0)

**Symptoms:**
- Alert: `DeepReorg` triggered (>10 blocks)
- Nodes reporting different chain tips
- Users reporting disappeared transactions

**Immediate Actions:**

```bash
# 1. Identify reorg depth and competing chains
./scripts/incident/analyze-reorg.sh

# 2. Compare chain work
for seed in seed-{1..3}.testnet.bitquan.io; do
    WORK=$(curl -s "http://$seed:19443/rpc" \
        -d '{"method":"getblockchaininfo"}' \
        | jq -r '.result.chainwork')
    
    TIP=$(curl -s "http://$seed:19443/rpc" \
        -d '{"method":"getbestblockhash"}' \
        | jq -r '.result')
    
    echo "$seed: work=$WORK, tip=$TIP"
done

# 3. If malicious: identify attacker
# Check if reorg came from single peer (potential 51% attack)
./scripts/incident/identify-attacker.sh

# 4. Coordinate response
# - If natural reorg (network partition): Let fork choice resolve
# - If attack: Activate emergency ban, coordinate with exchanges

# 5. Post-incident
# - Audit affected transactions
# - Notify users of potential double-spends
# - Increase monitoring sensitivity
```

**Post-Incident Review Template:**

After every P0/P1 incident, complete within 48 hours:

```markdown
# Incident Post-Mortem: [TITLE]

**Date:** YYYY-MM-DD
**Duration:** X hours Y minutes
**Severity:** P0/P1
**Incident Commander:** [NAME]

## Timeline

- **HH:MM UTC** - Alert triggered
- **HH:MM UTC** - On-call responded
- **HH:MM UTC** - Root cause identified
- **HH:MM UTC** - Fix deployed
- **HH:MM UTC** - Incident resolved

## Impact

- Affected users: [COUNT]
- Downtime: [DURATION]
- Transactions affected: [COUNT]

## Root Cause

[Detailed technical analysis]

## Resolution

[What was done to fix]

## Action Items

1. [ ] [ACTION] - Owner: [NAME] - Due: [DATE]
2. [ ] [ACTION] - Owner: [NAME] - Due: [DATE]

## Lessons Learned

**What went well:**
-

**What could be improved:**
-

**Follow-up:**
-
```

---

## 8. Post-Launch Operations

### 8.1 Launch Day Checklist (D-Day)

**T-24 hours:**
- [ ] Final genesis verification
- [ ] All seed nodes deployed and synced
- [ ] RPC gateway load tested
- [ ] Faucet pre-funded and tested
- [ ] Explorer indexer synchronized
- [ ] Monitoring dashboards reviewed
- [ ] On-call team briefed
- [ ] Community moderators prepared

**T-1 hour:**
- [ ] Genesis timestamp confirmed (2026-09-01 00:00:00 UTC)
- [ ] All services health checked
- [ ] Social media posts scheduled
- [ ] Status page updated

**T-0 (Launch):**
1. **00:00:00 UTC** - Genesis block propagates
2. **00:00:30** - Verify all seeds have genesis
3. **00:01:00** - Post launch announcement
4. **00:05:00** - First mined block expected
5. **00:10:00** - Faucet opens
6. **00:15:00** - Monitor for issues

**T+1 hour:**
- [ ] Minimum 10 blocks mined
- [ ] 5+ community nodes connected
- [ ] Faucet drips successful
- [ ] Explorer displaying blocks

**T+24 hours:**
- [ ] Chain stability confirmed
- [ ] No P0 incidents
- [ ] Community feedback collected
- [ ] Launch retrospective scheduled

### 8.2 Week 1 Operations Plan

**Daily Tasks:**
- Monitor node health (every 4 hours)
- Review alert logs
- Respond to community questions
- Track unique node count

**Weekly Tasks:**
- Performance optimization review
- Security audit findings check
- Infrastructure cost analysis
- Community AMA session

**Success Metrics (Week 1):**
- ✅ 100+ unique nodes connected
- ✅ 1,000+ blocks mined
- ✅ 10,000+ transactions processed
- ✅ 500+ faucet claims
- ✅ Zero P0 incidents
- ✅ <1 hour average incident response time

### 8.3 Continuous Improvement

**Bi-Weekly Review Topics:**
1. Incident post-mortems
2. Performance bottlenecks
3. Community feedback
4. Security advisories
5. Mainnet readiness progress

**Quarterly Milestones:**
- Q3 2026: Testnet stable, 1000+ nodes
- Q4 2026: External audit complete, mainnet launch
- Q1 2027: Mainnet operating smoothly

---

## Appendix A: Emergency Contacts Directory

[Full contact list with phone numbers, email, and PagerDuty integration details]

## Appendix B: Infrastructure Credentials

**⚠️ CLASSIFIED - Store in 1Password Team Vault**

- AWS IAM credentials
- SSH private keys
- JWT secrets
- Database passwords
- API tokens
- TLS certificates

## Appendix C: Rollback Procedures

**Scenario:** Critical bug discovered post-launch requiring node rollback.

```bash
# Emergency rollback procedure
./scripts/emergency/rollback-to-version.sh \
    --version v0.9.5 \
    --reason "Critical bug CVE-2026-12345"
```

---

**Document Status:** ✅ Complete — Ready for Launch Operations  
**Final Approval Required From:**
- [ ] Core Dev Lead
- [ ] Security Lead
- [ ] DevOps Lead
- [ ] Project Manager
- [ ] CEO

**Next Step:** Module 4 (Production Readiness Audit Sign-off)

---

**Signature:**  
*Principal L1 Blockchain Architect*  
*Date: 2026-08-14*
