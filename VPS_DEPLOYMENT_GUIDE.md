# 🚀 BitQuan Testnet VPS Deployment Guide

**Quick guide to deploy BitQuan testnet to a production VPS**

---

## 📋 **What You Need:**

### **VPS Requirements:**
```
Provider: DigitalOcean, Hetzner, AWS, or similar
OS: Ubuntu 22.04 LTS
RAM: 8GB minimum
CPU: 4 cores
Storage: 100GB SSD
Network: Unlimited bandwidth
Cost: ~$20-40/month
```

### **Domain (Optional but Recommended):**
```
testnet.bitquan.io    → Main node
pool.bitquan.io       → Mining pool
faucet.bitquan.io     → Faucet
explorer.bitquan.io   → Block explorer
```

---

## 🚀 **Quick Deploy (5 Minutes):**

### **Step 1: Get VPS**
```bash
# DigitalOcean Example
# 1. Go to: https://www.digitalocean.com
# 2. Create Droplet:
#    - Ubuntu 22.04
#    - 8GB RAM / 4 CPU
#    - $32/month plan
# 3. Add SSH key
# 4. Create
```

### **Step 2: SSH to Server**
```bash
ssh root@YOUR_SERVER_IP
```

### **Step 3: Run Auto-Setup Script**
```bash
curl -fsSL https://raw.githubusercontent.com/AlphaB135/BitQuan/main/scripts/setup-testnet.sh | bash
```

**That's it! The script will:**
- Install dependencies
- Build BitQuan
- Create wallets
- Setup services
- Start node
- Configure firewall

---

## 📝 **Manual Setup (For Full Control):**

### **1. Update System**
```bash
apt update && apt upgrade -y
apt install -y build-essential curl git pkg-config libssl-dev jq
```

### **2. Install Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### **3. Clone & Build**
```bash
git clone https://github.com/AlphaB135/BitQuan.git
cd BitQuan
git checkout v1.0.0
cargo build --release --bin bitquan-node
```

### **4. Create User**
```bash
useradd -m -s /bin/bash bitquan
mkdir -p /opt/bitquan/{bin,data,logs,config,backups}
cp target/release/bitquan-node /opt/bitquan/bin/
chown -R bitquan:bitquan /opt/bitquan
```

### **5. Generate Wallet**
```bash
su - bitquan
/opt/bitquan/bin/bitquan-node wallet-gen \
  --network testnet \
  --output /opt/bitquan/config/pool-wallet.keystore
# Enter password: testnet12345678
```

### **6. Create Config**
```bash
cat > /opt/bitquan/config/testnet.toml << 'EOF'
[network]
network_id = "testnet"
p2p_port = 8333
max_peers = 50

[rpc]
enabled = true
bind = "0.0.0.0"
port = 8334
require_auth = false

[mining]
enabled = true
threads = 4

[pool]
enabled = true
bind = "0.0.0.0"
stratum_port = 3333

[storage]
db_path = "/opt/bitquan/data/chainstate"
cache_size_mb = 1024
EOF
```

### **7. Create Systemd Service**
```bash
cat > /etc/systemd/system/bitquan-testnet.service << 'EOF'
[Unit]
Description=BitQuan Testnet Node
After=network.target

[Service]
Type=simple
User=bitquan
WorkingDirectory=/opt/bitquan
ExecStart=/opt/bitquan/bin/bitquan-node run \
  --config /opt/bitquan/config/testnet.toml
Restart=always
RestartSec=10
StandardOutput=append:/opt/bitquan/logs/node.log
StandardError=append:/opt/bitquan/logs/error.log

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable bitquan-testnet
systemctl start bitquan-testnet
```

### **8. Configure Firewall**
```bash
ufw allow 22/tcp   # SSH
ufw allow 8333/tcp # P2P
ufw allow 8334/tcp # RPC
ufw allow 3333/tcp # Stratum
ufw allow 8080/tcp # Dashboard
ufw enable
```

### **9. Setup SSL (Optional but Recommended)**
```bash
apt install -y certbot
certbot certonly --standalone -d testnet.bitquan.io
# Follow prompts
```

---

## 📊 **Monitoring Setup:**

### **Install Docker**
```bash
apt install -y docker.io docker-compose
systemctl enable docker
systemctl start docker
```

### **Start Monitoring**
```bash
cd /opt/bitquan/BitQuan/monitoring
docker-compose up -d
```

**Access:**
- Grafana: http://YOUR_SERVER_IP:3000
- Prometheus: http://YOUR_SERVER_IP:9090

---

## 🔍 **Verify Deployment:**

### **Check Node Status**
```bash
systemctl status bitquan-testnet
journalctl -u bitquan-testnet -f
```

### **Test RPC**
```bash
curl http://localhost:8334/health
```

### **Check Ports**
```bash
netstat -tulpn | grep -E '8333|8334|3333'
```

---

## 🌐 **Make it Public:**

### **Option 1: Direct IP Access**
```
Your users connect to: http://YOUR_SERVER_IP:8334
```

### **Option 2: Domain Name**
```bash
# Point domain to server IP in DNS
# A record: testnet.bitquan.io → YOUR_SERVER_IP

# Users connect to: https://testnet.bitquan.io
```

### **Option 3: Reverse Proxy (Nginx)**
```bash
apt install -y nginx

cat > /etc/nginx/sites-available/bitquan << 'EOF'
server {
    listen 80;
    server_name testnet.bitquan.io;

    location / {
        proxy_pass http://localhost:8334;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
EOF

ln -s /etc/nginx/sites-available/bitquan /etc/nginx/sites-enabled/
nginx -t
systemctl reload nginx
```

---

## 📈 **Production Checklist:**

### **Before Going Live:**
- [ ] Server secured (SSH key only, no password)
- [ ] Firewall configured
- [ ] SSL certificate installed
- [ ] Monitoring active
- [ ] Backups configured
- [ ] Log rotation setup
- [ ] Domain name configured
- [ ] Email alerts configured

### **After Launch:**
- [ ] Monitor logs daily
- [ ] Check metrics in Grafana
- [ ] Backup wallet keystores
- [ ] Keep software updated
- [ ] Respond to community

---

## 💰 **Cost Breakdown:**

```
VPS (DigitalOcean 8GB):  $32/month
Domain name:             $12/year (~$1/month)
SSL Certificate:         Free (Let's Encrypt)
Monitoring:              Free (self-hosted)
Backups:                 $5/month (optional)
────────────────────────────────────────
Total:                   ~$35-40/month
```

---

## 🔒 **Security Best Practices:**

```bash
# 1. Disable root login
sed -i 's/PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config

# 2. Use SSH keys only
sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config

# 3. Setup fail2ban
apt install -y fail2ban
systemctl enable fail2ban

# 4. Auto security updates
apt install -y unattended-upgrades
dpkg-reconfigure -plow unattended-upgrades

# 5. Regular backups
crontab -e
# Add: 0 2 * * * /opt/bitquan/backup.sh
```

---

## 📞 **Support:**

Issues with deployment?
- 📖 Docs: docs/TESTNET_SETUP.md
- 🐛 Issues: https://github.com/AlphaB135/BitQuan/issues
- 💬 Discord: #deployment-help

---

**Your testnet node is ready for the world! 🌍🚀**
